use super::*;
use crate::community::domain::post_import::*;
impl<R: Store> Service<R> {
    pub async fn preview_post_import(
        &self,
        a: &Actor,
        s: Uuid,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<PostImportPreview> {
        a.platform()?;
        self.0.check_post_import_access(a, s).await?;
        require(
            !bytes.is_empty() && bytes.len() <= MAX_BYTES,
            "文件须为 5 MiB 以内的非空文件",
        )?;
        use sha2::{Digest, Sha256};
        let file_digest = format!("{:x}", Sha256::digest(&bytes));
        // Bound parsing runs outside the async request executor.
        let (mut rows, mut errors) = tokio::task::spawn_blocking(move || {
            crate::community::south::adapter::post_import_file::parse(&name, &bytes, s)
        })
        .await
        .map_err(|_| crate::business::Error::Unavailable)??;
        let mut valid = Vec::new();
        for row in rows.drain(..) {
            let result = async {
                row.post.validate(s)?;
                self.0.validate_media(s, &row.post.images).await?;
                self.0
                    .check_words(s, &format!("{} {}", row.post.title, row.post.content))
                    .await
            }
            .await;
            match result {
                Ok(()) => valid.push(row),
                Err(crate::business::Error::Invalid(message)) => errors.push(ImportRowError {
                    row: row.row,
                    message,
                }),
                Err(error) => return Err(error),
            }
        }
        self.0
            .import_preview(a, s, file_digest, valid, errors)
            .await
    }
    pub async fn post_import_detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> Result<PostImportPreview> {
        a.platform()?;
        self.0.import_detail(a, s, id).await
    }
    pub async fn commit_post_import(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: CommitPostImport,
    ) -> Result<PostImportPreview> {
        a.platform()?;
        require(v.revision > 0, "版本号无效")?;
        self.0.import_commit(a, s, id, v).await
    }
}
