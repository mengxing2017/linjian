//! Durable, bounded media garbage collection. Files are removed outside row transactions.
use super::{
    lifecycle::{self, FileLock},
    postgres::Repository,
};
use crate::{business::Result, media::south::port::ContentStore};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

impl Repository {
    pub async fn cleanup(&self) -> Result<()> {
        let Some(mut lock) = FileLock::try_exclusive(&self.0).await? else {
            tracing::debug!(
                "file cleanup skipped while a backup or another cleaner holds the lock"
            );
            return Ok(());
        };
        let batch = self.1.config.lifecycle.cleanup_batch_size;
        let mut tx = lock.connection.begin().await?;
        lifecycle::configure(&mut tx, self.1.config.lifecycle.orphan_grace_hours).await?;
        // Lock assets before expired lease/token rows: renewal uses that same order.
        let expired: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM file_assets f WHERE store_id=$1 AND (EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=f.id AND l.expires_at<=now()) OR EXISTS(SELECT 1 FROM media_access_tokens t WHERE t.asset_id=f.id AND t.expires_at<=now())) ORDER BY id LIMIT $2 FOR UPDATE SKIP LOCKED")
            .bind(self.1.store_id).bind(batch).fetch_all(&mut *tx).await?;
        sqlx::query("DELETE FROM file_asset_leases WHERE ctid IN (SELECT ctid FROM file_asset_leases WHERE asset_id=ANY($1) AND expires_at<=now() ORDER BY asset_id LIMIT $2)")
            .bind(&expired).bind(batch).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM media_access_tokens WHERE token_hash IN (SELECT token_hash FROM media_access_tokens WHERE asset_id=ANY($1) AND expires_at<=now() ORDER BY asset_id LIMIT $2)")
            .bind(&expired).bind(batch).execute(&mut *tx).await?;
        tx.commit().await?;

        // Previews are business objects: lock them before the union of their assets.
        let mut tx = lock.connection.begin().await?;
        lifecycle::configure(&mut tx, self.1.config.lifecycle.orphan_grace_hours).await?;
        let previews: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM product_previews p WHERE expires_at<=now() AND NOT EXISTS(SELECT 1 FROM file_asset_references r JOIN file_assets f ON f.id=r.asset_id WHERE r.object_type='product_previews' AND r.object_id=p.id AND f.store_id<>$2) ORDER BY id LIMIT $1 FOR UPDATE SKIP LOCKED")
            .bind(batch).bind(self.1.store_id).fetch_all(&mut *tx).await?;
        sqlx::query("SELECT id FROM file_assets WHERE id IN (SELECT asset_id FROM file_asset_references WHERE object_type='product_previews' AND object_id=ANY($1)) ORDER BY id FOR UPDATE")
            .bind(&previews).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM product_previews WHERE id=ANY($1)")
            .bind(&previews)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let mut tx = lock.connection.begin().await?;
        lifecycle::configure(&mut tx, self.1.config.lifecycle.orphan_grace_hours).await?;
        let orphans: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM file_assets f WHERE store_id=$1 AND state='ready' AND gc_after IS NULL AND NOT linjian_file_protected(id) ORDER BY id LIMIT $2 FOR UPDATE SKIP LOCKED")
            .bind(self.1.store_id).bind(batch).fetch_all(&mut *tx).await?;
        // A fresh command after row locking rechecks protection, including concurrent commits.
        sqlx::query("SELECT linjian_refresh_file_gc($1)")
            .bind(&orphans)
            .execute(&mut *tx)
            .await?;
        let candidates: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM file_assets WHERE store_id=$1 AND retry_at<=now() AND ((state IN ('ready','uploading') AND gc_after<=now()) OR state='pending_delete') ORDER BY id LIMIT $2 FOR UPDATE SKIP LOCKED")
            .bind(self.1.store_id).bind(batch).fetch_all(&mut *tx).await?;
        let mut claimed = Vec::new();
        let mut protected = 0;
        for id in candidates {
            if sqlx::query_scalar::<_, bool>("SELECT linjian_file_protected($1)")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?
            {
                protected += 1;
                let state: String = sqlx::query_scalar("SELECT state FROM file_assets WHERE id=$1")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                if state == "pending_delete" {
                    tracing::error!(asset=%id,"protected file has a pending deletion; deletion stopped");
                    sqlx::query(
                        "UPDATE file_assets SET retry_at=now()+interval '1 hour' WHERE id=$1",
                    )
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                } else {
                    sqlx::query("UPDATE file_assets SET gc_after=NULL WHERE id=$1")
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
                continue;
            }
            let n = sqlx::query("UPDATE file_assets SET state='pending_delete',attempts=attempts+1,retry_at=now()+interval '5 minutes' WHERE id=$1 AND retry_at<=now() AND (state='pending_delete' OR (state IN ('ready','uploading') AND gc_after<=now()))")
                .bind(id).execute(&mut *tx).await?.rows_affected();
            if n == 1 {
                claimed.push(id);
            }
        }
        tx.commit().await?;
        let mut deleted = 0;
        let mut failed = 0;
        for id in &claimed {
            if sqlx::query_scalar::<_, bool>("SELECT linjian_file_protected($1)")
                .bind(id)
                .fetch_one(&mut lock.connection)
                .await?
            {
                protected += 1;
                tracing::error!(asset=%id,"file acquired unexpected protection after claim; deletion stopped");
                continue;
            }
            if self.1.remove(self.1.store_id, *id).await.is_err() {
                failed += 1;
                tracing::warn!(asset=%id,"file deletion failed; durable task will retry");
                continue;
            }
            finish(&mut lock.connection, *id).await?;
            deleted += 1;
        }
        tracing::info!(
            scheduled = orphans.len(),
            claimed = claimed.len(),
            deleted,
            failed,
            protected,
            "media cleanup completed"
        );
        lock.release().await?;
        Ok(())
    }
}

async fn finish(connection: &mut PgConnection, id: Uuid) -> Result<()> {
    let mut tx = connection.begin().await?;
    sqlx::query("DELETE FROM media_access_tokens WHERE asset_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for table in ["media_assets", "operational_media", "miniapp_avatar_assets"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE id=$1"))
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE file_assets SET state='deleted' WHERE id=$1 AND state='pending_delete'")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
