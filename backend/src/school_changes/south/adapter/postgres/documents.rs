use super::*;
use crate::business::require;
use sqlx::Row;

impl Repository {
    pub(super) async fn upload_document(&self, actor: &Actor, bytes: Vec<u8>) -> Result<Value> {
        let settings = &self.settings;
        let _permit = self
            .decode
            .acquire()
            .await
            .map_err(|_| Error::Unavailable)?;
        let mut tx = self.pool.begin().await?;
        own_session(&mut tx, actor)
            .await?
            .ok_or(Error::Policy("school_change_unbound"))?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_school_change_documents WHERE owner_id=$1 AND request_id IS NULL AND state IN ('uploading','ready') AND expires_at>now()")
            .bind(actor.id).fetch_one(&mut *tx).await?;
        if count >= settings.policy.max_unbound_documents {
            return Err(Error::RateLimited);
        }
        let bytes = super::super::private_files::normalize(bytes, &settings.policy).await?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO miniapp_school_change_documents(id,owner_id,state,mime,size_bytes,sha256,store_id,expires_at) VALUES($1,$2,'uploading','image/png',$3,$4,$5,now()+$6::bigint*interval '1 second')")
            .bind(id).bind(actor.id).bind(bytes.len() as i64).bind(crate::media::south::adapter::filesystem::digest(&bytes)).bind(settings.files.store_id).bind(settings.policy.temporary_seconds).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx = self.pool.begin().await?;
        own_session(&mut tx, actor).await?;
        sqlx::query("SELECT id FROM miniapp_school_change_documents WHERE id=$1 AND state='uploading' AND expires_at>now() FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::Policy("school_change_document_unavailable"))?;
        let size = bytes.len();
        settings.files.write(id, bytes).await?;
        sqlx::query("UPDATE miniapp_school_change_documents SET state='ready' WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        event(
            &mut tx,
            None,
            Some(id),
            Some(actor.id),
            "uploaded",
            "user",
            settings.policy.audit_retention_seconds,
        )
        .await?;
        tx.commit().await?;
        Ok(json!({"id":id,"mime":"image/png","size_bytes":size,"available":true}))
    }
    pub(super) async fn read_document(
        &self,
        actor: &Actor,
        id: Uuid,
        request: Option<Uuid>,
    ) -> Result<Vec<u8>> {
        let settings = &self.settings;
        let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM miniapp_school_change_documents WHERE id=$1 AND ($2 OR owner_id=$3)").bind(id).bind(request.is_some()).bind(actor.id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let mut tx = self.pool.begin().await?;
        school_binding::lock_accounts(&mut tx, actor, owner).await?;
        if request.is_some() {
            school_binding::platform_session(&mut tx, actor).await?;
        } else {
            own_session(&mut tx, actor).await?;
        }
        expire_pending(&mut tx, owner).await?;
        let row = sqlx::query("SELECT request_id,state,expires_at,purge_after,store_id,size_bytes,sha256,EXISTS(SELECT 1 FROM miniapp_school_change_requests r WHERE r.id=miniapp_school_change_documents.request_id AND r.status='approved') AS approved FROM miniapp_school_change_documents WHERE id=$1 FOR SHARE").bind(id).fetch_one(&mut *tx).await?;
        if let Some(request) = request {
            require(
                row.get::<Option<Uuid>, _>("request_id") == Some(request),
                "材料不属于本申请",
            )?;
        }
        let now = chrono::Utc::now();
        let available = row.get::<String, _>("state") == "ready"
            && !row.get::<bool, _>("approved")
            && row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at")
                .is_none_or(|expiry| expiry > now)
            && row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("purge_after")
                .is_none_or(|expiry| expiry > now);
        if !available {
            return Err(Error::Policy("school_change_document_unavailable"));
        }
        require(
            row.get::<Uuid, _>("store_id") == settings.files.store_id,
            "证件存储卷不匹配",
        )?;
        let request: Option<Uuid> = row.get("request_id");
        let retention = if let Some(request) = request {
            sqlx::query_scalar(
                "SELECT audit_retention_seconds FROM miniapp_school_change_requests WHERE id=$1",
            )
            .bind(request)
            .fetch_one(&mut *tx)
            .await?
        } else {
            settings.policy.audit_retention_seconds
        };
        let bytes = settings
            .files
            .read(
                settings.files.store_id,
                id,
                row.get("size_bytes"),
                row.get("sha256"),
            )
            .await?;
        event(
            &mut tx,
            request,
            Some(id),
            Some(actor.id),
            "document_read",
            if actor.is_admin_browser() {
                "platform"
            } else {
                "owner"
            },
            retention,
        )
        .await?;
        tx.commit().await?;
        Ok(bytes)
    }
}
