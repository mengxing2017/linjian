use crate::business::{Error, Result};
use crate::media::config::Config;
use rustix::fs::{AtFlags, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    sync::Arc,
};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Files {
    root: Arc<File>,
    pub store_id: Uuid,
    pub config: Arc<Config>,
    decode: Arc<Semaphore>,
}
fn io(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl Files {
    pub fn open(config: Config) -> Result<Self> {
        config.validate().map_err(|_| Error::Unavailable)?;
        std::fs::create_dir_all(&config.storage.filesystem_root).map_err(io)?;
        let root = File::from(
            rustix::fs::open(
                &config.storage.filesystem_root,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(io)?,
        );
        let marker = ".linjian-store";
        match rustix::fs::openat(
            &root,
            marker,
            OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::NOFOLLOW,
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(fd) => {
                let mut f = File::from(fd);
                f.write_all(Uuid::new_v4().to_string().as_bytes())
                    .map_err(io)?;
                f.sync_all().map_err(io)?;
                root.sync_all().map_err(io)?;
            }
            Err(rustix::io::Errno::EXIST) => {}
            Err(e) => return Err(io(e)),
        }
        let mut value = String::new();
        File::from(
            rustix::fs::openat(
                &root,
                marker,
                OFlags::RDONLY | OFlags::NOFOLLOW,
                Mode::empty(),
            )
            .map_err(io)?,
        )
        .take(64)
        .read_to_string(&mut value)
        .map_err(io)?;
        let store_id = value.parse().map_err(io)?;
        Ok(Self {
            root: Arc::new(root),
            store_id,
            decode: Arc::new(Semaphore::new(config.upload.decode_concurrency)),
            config: Arc::new(config),
        })
    }
    pub async fn validate(&self, bytes: Vec<u8>) -> Result<(String, u32, u32, Vec<u8>)> {
        let permit = self.decode.clone().acquire_owned().await.map_err(io)?;
        let limits = self.config.upload.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            crate::business::require(bytes.len() <= limits.max_image_bytes, "图片超过大小限制")?;
            let (mime, w, h) = crate::media::domain::validate(&bytes)?;
            crate::business::require(
                w <= limits.max_width
                    && h <= limits.max_height
                    && u64::from(w) * u64::from(h) <= limits.max_pixels,
                "图片尺寸超过限制",
            )?;
            Ok((mime, w, h, bytes))
        })
        .await
        .map_err(io)?
    }
    pub async fn write(&self, id: Uuid, bytes: Vec<u8>) -> Result<()> {
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            let temp = format!("{id}.tmp");
            let key = format!("{id}.blob");
            let fd = rustix::fs::openat(
                &*root,
                &temp,
                OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::NOFOLLOW,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(io)?;
            let mut file = File::from(fd);
            file.write_all(&bytes).map_err(io)?;
            file.sync_all().map_err(io)?;
            rustix::fs::renameat(&*root, &temp, &*root, &key).map_err(io)?;
            root.sync_all().map_err(io)?;
            Ok(())
        })
        .await
        .map_err(io)?
    }
    pub async fn read(&self, store: Uuid, id: Uuid, size: i64, hash: String) -> Result<Vec<u8>> {
        if store != self.store_id || !(1..=5 * 1024 * 1024).contains(&size) {
            return Err(Error::Unavailable);
        }
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            let fd = rustix::fs::openat(
                &*root,
                format!("{id}.blob"),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(io)?;
            let f = File::from(fd);
            if !f.metadata().map_err(io)?.is_file() {
                return Err(Error::Unavailable);
            }
            let mut bytes = Vec::new();
            f.take(size as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(io)?;
            if bytes.len() as i64 != size || digest(&bytes) != hash {
                return Err(Error::Unavailable);
            }
            Ok(bytes)
        })
        .await
        .map_err(io)?
    }
    pub async fn delete(&self, store: Uuid, id: Uuid) -> Result<()> {
        if store != self.store_id {
            return Err(Error::Unavailable);
        }
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            for suffix in ["tmp", "blob"] {
                match rustix::fs::unlinkat(&*root, format!("{id}.{suffix}"), AtFlags::empty()) {
                    Ok(()) | Err(rustix::io::Errno::NOENT) => {}
                    Err(e) => return Err(io(e)),
                }
            }
            root.sync_all().map_err(io)?;
            Ok(())
        })
        .await
        .map_err(io)?
    }
}
impl Default for Files {
    fn default() -> Self {
        static TEST_FILES: std::sync::OnceLock<Files> = std::sync::OnceLock::new();
        TEST_FILES
            .get_or_init(|| {
                // Explicitly used by test application builders. Production loads a
                // required TOML file; it never falls back to this temporary directory.
                Self::open(Config::local(
                    std::env::temp_dir().join(format!("linjian-test-media-{}", std::process::id())),
                ))
                .expect("test media directory")
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Files, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("linjian-files-test-{}", Uuid::new_v4()));
        (Files::open(Config::local(root.clone())).unwrap(), root)
    }
    #[tokio::test]
    async fn persistence_integrity_and_idempotent_deletion() {
        let (files, root) = fixture();
        let id = Uuid::new_v4();
        let bytes = b"private image".to_vec();
        files.write(id, bytes.clone()).await.unwrap();
        let reopened = Files::open(Config::local(root.clone())).unwrap();
        assert_eq!(files.store_id, reopened.store_id);
        assert_eq!(
            reopened
                .read(files.store_id, id, bytes.len() as i64, digest(&bytes))
                .await
                .unwrap(),
            bytes
        );
        assert!(
            files
                .read(Uuid::new_v4(), id, bytes.len() as i64, digest(&bytes))
                .await
                .is_err()
        );
        assert!(
            files
                .read(files.store_id, id, bytes.len() as i64, digest(b"other"))
                .await
                .is_err()
        );
        files.delete(files.store_id, id).await.unwrap();
        files.delete(files.store_id, id).await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_symlink_reads_and_symlink_root() {
        let (files, root) = fixture();
        let id = Uuid::new_v4();
        let outside = root.with_extension("private");
        std::fs::write(&outside, b"secret").unwrap();
        std::os::unix::fs::symlink(&outside, root.join(format!("{id}.blob"))).unwrap();
        assert!(
            files
                .read(files.store_id, id, 6, digest(b"secret"))
                .await
                .is_err()
        );
        let alias = root.with_extension("alias");
        std::os::unix::fs::symlink(&root, &alias).unwrap();
        assert!(Files::open(Config::local(alias.clone())).is_err());
        std::fs::remove_file(alias).unwrap();
        std::fs::remove_file(outside).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
