use super::*;
use sqlx::Row;

impl Repository {
    pub(super) async fn purge_account(&self, account: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        expire_pending(&mut tx, account).await?;
        let pending: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM miniapp_school_change_requests WHERE account_id=$1 AND status='pending' ORDER BY id FOR UPDATE").bind(account).fetch_all(&mut *tx).await?;
        for id in pending {
            finish(
                &mut tx,
                id,
                "invalidated",
                None,
                None,
                "account_closed",
                None,
            )
            .await?;
        }
        sqlx::query("UPDATE miniapp_school_change_requests SET declared_student_name=NULL,declared_student_number=NULL,reason=NULL,result_reason=NULL,personal_data_purged_at=now() WHERE account_id=$1").bind(account).execute(&mut *tx).await?;
        sqlx::query("UPDATE miniapp_school_change_documents SET purge_after=now() WHERE owner_id=$1 AND state IN ('ready','uploading')").bind(account).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub(super) async fn cleanup_documents(&self) -> Result<()> {
        let overdue: Vec<Uuid> = sqlx::query_scalar("SELECT account_id FROM miniapp_school_change_requests WHERE status='pending' AND review_due_at<=clock_timestamp() ORDER BY review_due_at,id LIMIT 50")
            .fetch_all(&self.pool).await?;
        for account in overdue {
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account)
                .fetch_one(&mut *tx)
                .await?;
            expire_pending(&mut tx, account).await?;
            tx.commit().await?;
        }
        self.delete_documents(None).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE miniapp_school_change_requests SET declared_student_name=NULL,declared_student_number=NULL,reason=NULL,result_reason=NULL,personal_data_purged_at=now() WHERE closed_at IS NOT NULL AND closed_at+personal_retention_seconds*interval '1 second'<=now() AND personal_data_purged_at IS NULL").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM miniapp_school_change_events e WHERE e.purge_after<=now() AND (e.request_id IS NULL OR EXISTS(SELECT 1 FROM miniapp_school_change_requests r WHERE r.id=e.request_id AND r.status<>'pending'))").execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub(super) async fn delete_documents(&self, request: Option<Uuid>) -> Result<()> {
        let settings = &self.settings;
        let candidates = sqlx::query("SELECT id,owner_id FROM miniapp_school_change_documents WHERE store_id=$1 AND ($2::uuid IS NULL OR request_id=$2) AND retry_at<=now() AND ((request_id IS NULL AND expires_at<=now() AND state IN ('ready','uploading')) OR (purge_after<=now() AND state IN ('ready','uploading')) OR state='deleting') ORDER BY retry_at,id LIMIT 50")
            .bind(settings.files.store_id).bind(request).fetch_all(&self.pool).await?;
        for candidate in candidates {
            let id: Uuid = candidate.get("id");
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(candidate.get::<Uuid, _>("owner_id"))
                .fetch_one(&mut *tx)
                .await?;
            let claimed: Option<Uuid> = sqlx::query_scalar("UPDATE miniapp_school_change_documents SET state='deleting',retry_at=now()+interval '5 minutes',attempts=attempts+1 WHERE id=$1 AND retry_at<=now() AND ((request_id IS NULL AND expires_at<=now() AND state IN ('ready','uploading')) OR (purge_after<=now() AND state IN ('ready','uploading')) OR state='deleting') RETURNING id").bind(id).fetch_optional(&mut *tx).await?;
            tx.commit().await?;
            if claimed.is_none() {
                continue;
            }
            if settings
                .files
                .delete(settings.files.store_id, id)
                .await
                .is_err()
            {
                tracing::warn!("private school change document deletion will retry");
                continue;
            }
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(candidate.get::<Uuid, _>("owner_id"))
                .fetch_one(&mut *tx)
                .await?;
            let request: Option<Uuid> = sqlx::query_scalar("UPDATE miniapp_school_change_documents SET state='deleted',deleted_at=now(),sha256='' WHERE id=$1 AND state='deleting' RETURNING request_id").bind(id).fetch_one(&mut *tx).await?;
            event(
                &mut tx,
                request,
                Some(id),
                None,
                "document_deleted",
                "retention",
                settings.policy.audit_retention_seconds,
            )
            .await?;
            tx.commit().await?;
        }
        Ok(())
    }
}
