use super::{Repository, post_content, post_management::guard};
use crate::{
    business::{Actor, Error, Result, hash, require},
    community::domain::post_import::*,
};
use serde_json::{Value, json};
use uuid::Uuid;
fn decode(row: Value) -> Result<PostImportPreview> {
    serde_json::from_value(row).map_err(|_| Error::Unavailable)
}
impl Repository {
    pub async fn check_post_import_access(&self, a: &Actor, s: Uuid) -> Result<()> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, true).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Payload retention is independent from forum writes and import result identity.
    pub async fn cleanup_post_imports(&self) -> Result<()> {
        sqlx::query("UPDATE post_import_jobs SET rows='[]',errors='[]',state='expired' WHERE state='preview' AND expires_at<=now()")
            .execute(&self.0).await?;
        Ok(())
    }

    pub async fn import_preview(
        &self,
        a: &Actor,
        s: Uuid,
        file_digest: String,
        rows: Vec<ImportRow>,
        errors: Vec<ImportRowError>,
    ) -> Result<PostImportPreview> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, true).await?;
        // Scrub expired payloads on use; no runtime dependency on an external cleanup worker.
        sqlx::query("UPDATE post_import_jobs SET rows='[]',errors='[]',state='expired' WHERE state='preview' AND expires_at<=now() AND actor_id=$1").bind(a.id).execute(&mut *tx).await?;
        let rows = serde_json::to_value(rows).map_err(|_| Error::Unavailable)?;
        let errors = serde_json::to_value(errors).map_err(|_| Error::Unavailable)?;
        let digest = hash(&json!([s, a.id, file_digest, &rows, &errors]).to_string());
        let row:Value=sqlx::query_scalar("INSERT INTO post_import_jobs(id,school_id,actor_id,digest,rows,errors) VALUES($1,$2,$3,$4,$5,$6) RETURNING to_jsonb(post_import_jobs)-'idempotency_key'-'created_at'")
  .bind(Uuid::now_v7()).bind(s).bind(a.id).bind(digest).bind(rows).bind(errors).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        decode(row)
    }
    pub async fn import_detail(&self, a: &Actor, s: Uuid, id: Uuid) -> Result<PostImportPreview> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, true).await?;
        sqlx::query("UPDATE post_import_jobs SET rows='[]',errors='[]',state='expired' WHERE id=$1 AND school_id=$2 AND actor_id=$3 AND state='preview' AND expires_at<=now()").bind(id).bind(s).bind(a.id).execute(&mut *tx).await?;
        let row:Value=sqlx::query_scalar("SELECT to_jsonb(j)-'idempotency_key'-'created_at' FROM post_import_jobs j WHERE id=$1 AND school_id=$2 AND actor_id=$3")
  .bind(id).bind(s).bind(a.id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        tx.commit().await?;
        decode(row)
    }
    pub async fn import_commit(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: CommitPostImport,
    ) -> Result<PostImportPreview> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, true).await?;
        let row:Value=sqlx::query_scalar("SELECT to_jsonb(j) FROM post_import_jobs j WHERE id=$1 AND school_id=$2 AND actor_id=$3 FOR UPDATE")
  .bind(id).bind(s).bind(a.id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if row["digest"] != v.digest {
            return Err(Error::Conflict);
        }
        if row["state"] == "committed" {
            if row["idempotency_key"] != v.idempotency_key.to_string() {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return decode(row);
        }
        if row["revision"].as_i64() != Some(v.revision) {
            return Err(Error::Conflict);
        }
        let valid: bool =
            sqlx::query_scalar("SELECT expires_at>now() FROM post_import_jobs WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        require(
            valid && row["state"] == "preview",
            "导入预览已过期，请重新上传",
        )?;
        let reused:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM post_import_jobs WHERE actor_id=$1 AND school_id=$2 AND idempotency_key=$3 AND id<>$4)").bind(a.id).bind(s).bind(v.idempotency_key).bind(id).fetch_one(&mut *tx).await?;
        if reused {
            return Err(Error::Conflict);
        }
        let preview = decode(row)?;
        require(
            preview.errors.is_empty() && !preview.rows.is_empty() && preview.rows.len() <= MAX_ROWS,
            "请先修正导入错误",
        )?;
        let mut ids = Vec::new();
        for row in preview.rows {
            let item = post_content::create(&mut tx, a, s, row.post).await?;
            ids.push(crate::business::uuid(&item, "id")?);
        }
        let current:Value=sqlx::query_scalar("UPDATE post_import_jobs SET state='committed',revision=revision+1,result=$2,idempotency_key=$3,rows='[]',errors='[]' WHERE id=$1 RETURNING to_jsonb(post_import_jobs)-'idempotency_key'-'created_at'")
  .bind(id).bind(json!(ids)).bind(v.idempotency_key).fetch_one(&mut *tx).await?;
        crate::audit::record(
            &mut tx,
            a,
            s,
            "post.import",
            id,
            json!({"count":ids.len(),"post_ids":ids}),
        )
        .await?;
        tx.commit().await?;
        decode(current)
    }
}
