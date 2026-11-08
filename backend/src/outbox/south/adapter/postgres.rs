use crate::business::{Actor, Error, Page, Result};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
impl Repository {
    pub async fn list(&self, actor: &Actor, school: Uuid, page: Page) -> Result<Vec<Value>> {
        actor.authorize(school, "operator")?;
        let (limit, offset) = page.fetch_bounds()?;
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',id,'post_id',post_id,'sequence',sequence,'event_type',payload->>'event_type','attempts',attempts,'next_attempt_at',next_attempt_at,'receipt_id',receipt_id,'receipt_status',receipt_status,'receipt_summary',receipt_summary,'last_error',last_error,'schema_version',schema_version,'route_id',route_id,'blocked_reason',blocked_reason,'accepted_at',accepted_at,'created_at',created_at) FROM forum_outbox WHERE school_id=$1 ORDER BY created_at DESC,id DESC LIMIT $2 OFFSET $3").bind(school).bind(limit).bind(offset).fetch_all(&self.0).await?)
    }
    pub async fn replay(&self, actor: &Actor, school: Uuid, id: Uuid) -> Result<()> {
        actor.authorize(school, "operator")?;
        let mut tx = self.0.begin().await?;
        let count=sqlx::query("UPDATE forum_outbox SET next_attempt_at=now(),blocked_reason=CASE WHEN schema_version=1 THEN 'legacy_version_blocked' ELSE NULL END,blocked_at=CASE WHEN schema_version=1 THEN now() ELSE NULL END,auth_failure_count=0 WHERE id=$1 AND school_id=$2").bind(id).bind(school).execute(&mut *tx).await?.rows_affected();
        if count == 0 {
            return Err(Error::NotFound);
        }
        crate::audit::record(
            &mut tx,
            actor,
            school,
            "source_event.replay",
            id,
            serde_json::json!({}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
