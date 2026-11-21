use super::*;
use crate::business::require;
use crate::school_changes::domain::{Decision, pending};
use sqlx::Row;

impl Repository {
    pub(super) async fn close_request(
        &self,
        actor: &Actor,
        id: Uuid,
        revision: i64,
        decision: Option<Decision>,
        approve: bool,
    ) -> Result<Value> {
        let platform = decision.is_some();
        require(!approve || platform, "批准需要平台审核结论")?;
        let owner: Uuid = sqlx::query_scalar("SELECT account_id FROM miniapp_school_change_requests WHERE id=$1 AND ($2 OR account_id=$3)").bind(id).bind(platform).bind(actor.id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let mut tx = self.pool.begin().await?;
        school_binding::lock_accounts(&mut tx, actor, owner).await?;
        let (bound, status, _) =
            crate::miniapp_users::south::adapter::postgres::lock_user(&mut tx, owner).await?;
        if platform {
            school_binding::platform_session(&mut tx, actor).await?;
            if actor.id == owner {
                return Err(Error::Policy("school_change_self_review"));
            }
        } else {
            own_session(&mut tx, actor).await?;
        }
        let row = sqlx::query("SELECT status,revision,from_school_id,to_school_id FROM miniapp_school_change_requests WHERE id=$1 FOR UPDATE").bind(id).fetch_one(&mut *tx).await?;
        pending(
            &row.get::<String, _>("status"),
            row.get("revision"),
            revision,
        )?;
        if expire_pending(&mut tx, owner).await? {
            tx.commit().await?;
            return Err(Error::Policy("school_change_review_timeout"));
        }
        let from: Uuid = row.get("from_school_id");
        let target: Uuid = row.get("to_school_id");
        if approve {
            let settings = &self.settings;
            let relationship: Option<String> = sqlx::query_scalar(
                "SELECT student_relationship FROM miniapp_school_change_requests WHERE id=$1",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if relationship.is_none() {
                return Err(Error::Policy("school_change_relationship_required"));
            }
            if bound != Some(from) {
                return Err(Error::Policy("school_change_source_changed"));
            }
            if status != "normal" {
                return Err(Error::MiniappBanned);
            }
            let active: bool = sqlx::query_scalar(
                "SELECT active AND account_kind='ordinary' FROM accounts WHERE id=$1",
            )
            .bind(owner)
            .fetch_one(&mut *tx)
            .await?;
            if !active {
                return Err(Error::Policy("school_change_account_unavailable"));
            }
            school_binding::validate_target(&mut tx, owner, bound, target).await?;
            let revoked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_memberships WHERE account_id=$1 AND school_id=$2 AND role='revoked')").bind(owner).bind(target).fetch_one(&mut *tx).await?;
            if revoked && !settings.policy.restore_revoked_member {
                return Err(Error::Policy("school_change_target_member_revoked"));
            }
            let documents = sqlx::query("SELECT id,state,purge_after,store_id,size_bytes,sha256 FROM miniapp_school_change_documents WHERE request_id=$1 ORDER BY id FOR UPDATE").bind(id).fetch_all(&mut *tx).await?;
            if documents.is_empty() {
                return Err(Error::Policy("school_change_document_unavailable"));
            }
            for document in documents {
                let available = document.get::<String, _>("state") == "ready"
                    && document.get::<Uuid, _>("store_id") == settings.files.store_id
                    && document
                        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("purge_after")
                        .is_none_or(|expiry| expiry > chrono::Utc::now());
                if !available {
                    return Err(Error::Policy("school_change_document_unavailable"));
                }
                settings
                    .files
                    .read(
                        settings.files.store_id,
                        document.get("id"),
                        document.get("size_bytes"),
                        document.get("sha256"),
                    )
                    .await?;
            }
            if expire_pending(&mut tx, owner).await? {
                tx.commit().await?;
                return Err(Error::Policy("school_change_review_timeout"));
            }
            finish(
                &mut tx,
                id,
                "approved",
                Some(actor.id),
                Some(actor.id),
                "verified_target_school",
                decision.as_ref().map(|input| input.reason.trim()),
            )
            .await?;
            school_binding::change(&mut tx, actor, owner, bound, target).await?;
        } else if let Some(input) = &decision {
            finish(
                &mut tx,
                id,
                "rejected",
                Some(actor.id),
                Some(actor.id),
                "review_rejected",
                Some(input.reason.trim()),
            )
            .await?;
        } else {
            finish(
                &mut tx,
                id,
                "cancelled",
                None,
                Some(actor.id),
                "user_cancelled",
                None,
            )
            .await?;
        }
        let result = view(&mut tx, id).await?;
        tx.commit().await?;
        if approve && self.delete_documents(Some(id)).await.is_err() {
            tracing::warn!("approved school change document deletion will retry");
        }
        Ok(result)
    }
}
