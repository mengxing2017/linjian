use crate::business::{Error, Result, require};
use crate::media::south::adapter::filesystem::Files;
use crate::school_changes::domain::policy::Policy;
use image::{ImageFormat, ImageReader, Limits};
use std::io::{Cursor, Read, Write};
use std::os::unix::fs::DirBuilderExt;

#[cfg(test)]
mod tests;

pub fn defaults(public: &Files) -> Result<Policy> {
    let private_root = public
        .config
        .storage
        .filesystem_root
        .join(".school-change-documents");
    match std::fs::DirBuilder::new().mode(0o700).create(&private_root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(Error::Unavailable),
    }
    let root = std::fs::File::from(
        rustix::fs::open(
            &private_root,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(|_| Error::Unavailable)?,
    );
    rustix::fs::fchmod(
        &root,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR | rustix::fs::Mode::XUSR,
    )
    .map_err(|_| Error::Unavailable)?;
    rustix::fs::flock(&root, rustix::fs::FlockOperation::LockExclusive)
        .map_err(|_| Error::Unavailable)?;
    let marker = ".linjian-school-change-key";
    match rustix::fs::openat(
        &root,
        marker,
        rustix::fs::OFlags::CREATE
            | rustix::fs::OFlags::EXCL
            | rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    ) {
        Ok(descriptor) => {
            let mut file = std::fs::File::from(descriptor);
            let key = format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            );
            file.write_all(key.as_bytes())
                .map_err(|_| Error::Unavailable)?;
            file.sync_all().map_err(|_| Error::Unavailable)?;
            root.sync_all().map_err(|_| Error::Unavailable)?;
        }
        Err(rustix::io::Errno::EXIST) => {}
        Err(_) => return Err(Error::Unavailable),
    }
    let descriptor = rustix::fs::openat(
        &root,
        marker,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| Error::Unavailable)?;
    let file = std::fs::File::from(descriptor);
    require(
        file.metadata().map_err(|_| Error::Unavailable)?.is_file(),
        "换校摘要密钥文件无效",
    )?;
    let mut key = String::new();
    file.take(65)
        .read_to_string(&mut key)
        .map_err(|_| Error::Unavailable)?;
    require(
        key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "换校摘要密钥文件无效",
    )?;
    let mut policy = Policy::defaults(private_root, key);
    policy.max_file_bytes = public.config.upload.max_image_bytes;
    policy.max_total_bytes = (policy.max_file_bytes * policy.max_documents) as i64;
    policy.max_pixels = public.config.upload.max_pixels;
    Ok(policy)
}

pub fn open(policy: &Policy, public: &Files) -> Result<Files> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&policy.private_root)
        .map_err(|_| Error::Unavailable)?;
    let root = std::fs::File::from(
        rustix::fs::open(
            &policy.private_root,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(|_| Error::Unavailable)?,
    );
    rustix::fs::flock(&root, rustix::fs::FlockOperation::LockExclusive)
        .map_err(|_| Error::Unavailable)?;
    let private = Files::open(crate::media::config::Config::local(
        policy.private_root.clone(),
    ))?;
    let private_path =
        std::fs::canonicalize(&policy.private_root).map_err(|_| Error::Unavailable)?;
    let public_path = std::fs::canonicalize(&public.config.storage.filesystem_root)
        .map_err(|_| Error::Unavailable)?;
    require(
        (!private_path.starts_with(&public_path) && !public_path.starts_with(&private_path))
            || private_path == public_path.join(".school-change-documents"),
        "证件目录必须与公开媒体目录隔离",
    )?;
    Ok(private)
}
pub async fn normalize(bytes: Vec<u8>, policy: &Policy) -> Result<Vec<u8>> {
    require(
        !bytes.is_empty() && bytes.len() <= policy.max_file_bytes,
        "材料文件超过大小限制",
    )?;
    let max_pixels = policy.max_pixels;
    let max_bytes = policy.max_file_bytes;
    tokio::task::spawn_blocking(move || {
        let mut reader = ImageReader::new(Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|_| Error::Invalid("材料图片格式无效".into()))?;
        require(
            matches!(
                reader.format(),
                Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP)
            ),
            "仅支持 JPEG/PNG/WebP 图片",
        )?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(max_pixels.saturating_mul(16).min(384_000_000));
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|_| Error::Invalid("材料图片损坏或尺寸过大".into()))?;
        require(
            u64::from(image.width()) * u64::from(image.height()) <= max_pixels,
            "材料像素超过限制",
        )?;
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, ImageFormat::Png)
            .map_err(|_| Error::Unavailable)?;
        let output = output.into_inner();
        require(output.len() <= max_bytes, "规范化后的材料超过大小限制")?;
        Ok(output)
    })
    .await
    .map_err(|_| Error::Unavailable)?
}
