use super::{Repository, post_management};
use crate::{
    business::{Actor, Error, Page, Result},
    community::domain::post_recommendation::{RecommendPost, RecommendationView},
};
use serde_json::{Value, json};
use sqlx::{PgConnection, Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) async fn can_recommend(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Uuid,
) -> Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT forum_has_permission($1,$2,'post.recommend')")
            .bind(a.id)
            .bind(s)
            .fetch_one(&mut **tx)
            .await?,
    )
}
const VIEW: &str = "SELECT jsonb_build_object('id',o.id,'school_id',o.school_id,'post_id',o.post_id,'recommendation_id',r.id,'trigger_type',o.trigger_type,'state',CASE WHEN o.delivery_state='accepted' THEN 'accepted' WHEN o.delivery_state='cancelled' THEN 'cancelled' WHEN o.delivery_state IN ('blocked','waiting_binding','waiting_configuration','paused') OR p.state='blocked' THEN 'blocked' WHEN o.request_body IS NULL THEN 'preparing' ELSE 'processing' END,'created_at',o.created_at,'accepted_at',o.accepted_at,'actor_name',a.display_name,'reason',r.reason,'terminated_reason',t.reason) FROM forum_outbox o LEFT JOIN forum_post_recommendations r ON r.event_id=o.id LEFT JOIN accounts a ON a.id=r.actor_id LEFT JOIN forum_publication_preparations p ON p.event_id=o.id LEFT JOIN forum_source_terminations t ON t.event_id=o.id";
async fn record(conn: &mut PgConnection, s: Uuid, post: Uuid, id: Uuid) -> Result<Value> {
    let value: Value = sqlx::query_scalar(&format!(
        "{VIEW} WHERE o.school_id=$1 AND o.post_id=$2 AND o.id=$3"
    ))
    .bind(s)
    .bind(post)
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(Error::NotFound)?;
    // Validate the same explicit projection used by TS and fixtures.
    let view: RecommendationView = serde_json::from_value(value).map_err(|_| Error::Unavailable)?;
    serde_json::to_value(view).map_err(|_| Error::Unavailable)
}
async fn eligibility(tx: &mut Transaction<'_, Postgres>, s: Uuid, id: Uuid) -> Result<Value> {
    let row:Value=sqlx::query_scalar("SELECT to_jsonb(p)||jsonb_build_object('school_name',s.name,'summary',snap.summary,'evidence_valid',forum_post_has_publication_evidence(p)) FROM posts p JOIN schools s ON s.id=p.school_id LEFT JOIN forum_publication_snapshots snap ON snap.id=p.approved_snapshot_id WHERE p.school_id=$1 AND p.id=$2 AND p.visibility<>'deleted' FOR UPDATE OF p")
        .bind(s).bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    Ok(row)
}
async fn unavailable(
    tx: &mut Transaction<'_, Postgres>,
    s: Uuid,
    id: Uuid,
    row: &Value,
) -> Result<Option<&'static str>> {
    if row["audit_status"] != "approved"
        || row["visibility"] != "visible"
        || row["evidence_valid"] != true
    {
        return Ok(Some("recommendation_not_eligible"));
    }
    let binding=sqlx::query("SELECT v.origin,v.key_id FROM school_source_bindings b JOIN source_active_configuration ac ON ac.connection_id=b.connection_id JOIN source_connection_versions v ON v.id=b.connection_version WHERE b.school_id=$1 AND b.active FOR SHARE OF b,ac")
        .bind(s).fetch_optional(&mut **tx).await?;
    let configured = binding.as_ref().is_some_and(|r| {
        crate::source_access::configuration::ConnectionConfig::from_env()
            .ok()
            .flatten()
            .is_some_and(|c| c.material(r.get("origin"), r.get("key_id")).is_ok())
    });
    if !configured {
        return Ok(Some("recommendation_configuration_required"));
    }
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum_outbox WHERE school_id=$1 AND post_id=$2 AND delivery_state NOT IN ('accepted','cancelled'))")
        .bind(s).bind(id).fetch_one(&mut **tx).await?;
    if pending {
        return Ok(Some("recommendation_pending"));
    }
    let cooldown:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum_outbox WHERE school_id=$1 AND post_id=$2 AND created_at>now()-interval '24 hours')")
        .bind(s).bind(id).fetch_one(&mut **tx).await?;
    Ok(cooldown.then_some("recommendation_cooldown"))
}
impl Repository {
    pub async fn recommendation_preview(&self, a: &Actor, s: Uuid, id: Uuid) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        post_management::guard(&mut tx, a, s, false).await?;
        let allowed = can_recommend(&mut tx, a, s).await?;
        let row = eligibility(&mut tx, s, id).await?;
        let reason = if allowed {
            unavailable(&mut tx, s, id, &row).await?
        } else {
            Some("forbidden")
        };
        let latest:Option<Uuid>=sqlx::query_scalar("SELECT id FROM forum_outbox WHERE school_id=$1 AND post_id=$2 ORDER BY created_at DESC,id DESC LIMIT 1").bind(s).bind(id).fetch_optional(&mut *tx).await?;
        let next:Option<chrono::DateTime<chrono::Utc>>=sqlx::query_scalar("SELECT max(created_at)+interval '24 hours' FROM forum_outbox WHERE school_id=$1 AND post_id=$2").bind(s).bind(id).fetch_one(&mut *tx).await?;
        let latest = match latest {
            Some(event) => Some(record(&mut tx, s, id, event).await?),
            None => None,
        };
        let result = json!({"id":id,"school_id":s,"school_name":row["school_name"],"title":row["title"],"summary":row["summary"],"revision":row["revision"],"content_revision":row["content_revision"],"publication_snapshot_id":row["approved_snapshot_id"],"authorized":allowed,"can_recommend":reason.is_none(),"unavailable_reason":reason,"next_eligible_at":next,"latest":latest});
        let typed: crate::community::domain::post_recommendation::RecommendationPreview =
            serde_json::from_value(result).map_err(|_| Error::Unavailable)?;
        tx.commit().await?;
        serde_json::to_value(typed).map_err(|_| Error::Unavailable)
    }
    pub async fn recommend_post(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        mut input: RecommendPost,
    ) -> Result<(Value, bool)> {
        input.validate()?;
        let digest = crate::business::hash(&json!([s, id, a.id, input]).to_string());
        let mut tx = self.0.begin().await?;
        // Live authorization precedes idempotency; replay does not revalidate mutable post content.
        crate::schools::south::adapter::postgres::permissions::authorize(
            &mut tx,
            a,
            s,
            "post.recommend",
        )
        .await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(s)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        if !can_recommend(&mut tx, a, s).await? {
            return Err(Error::Forbidden);
        }
        let existing=sqlx::query("SELECT event_id,post_id,request_digest FROM forum_post_recommendations WHERE school_id=$1 AND actor_id=$2 AND idempotency_key=$3")
            .bind(s).bind(a.id).bind(input.idempotency_key).fetch_optional(&mut *tx).await?;
        if let Some(r) = existing {
            if r.get::<String, _>("request_digest") != digest {
                return Err(Error::Policy("recommendation_idempotency_conflict"));
            }
            let value = record(&mut tx, s, r.get("post_id"), r.get("event_id")).await?;
            tx.commit().await?;
            return Ok((value, false));
        }
        let active: bool = sqlx::query_scalar("SELECT active FROM schools WHERE id=$1")
            .bind(s)
            .fetch_one(&mut *tx)
            .await?;
        if !active {
            return Err(Error::Policy("recommendation_not_eligible"));
        }
        let row = eligibility(&mut tx, s, id).await?;
        if row["revision"].as_i64() != Some(input.revision)
            || row["content_revision"].as_i64() != Some(input.content_revision)
            || row["approved_snapshot_id"].as_str()
                != Some(input.publication_snapshot_id.to_string().as_str())
        {
            return Err(Error::Conflict);
        }
        if let Some(reason) = unavailable(&mut tx, s, id, &row).await? {
            return Err(Error::Policy(reason));
        }
        let event:Uuid=sqlx::query_scalar("SELECT forum_create_source_intent(p,'manual_recommendation') FROM posts p WHERE p.school_id=$1 AND p.id=$2")
            .bind(s).bind(id).fetch_one(&mut *tx).await?;
        let recommendation = Uuid::now_v7();
        sqlx::query("INSERT INTO forum_post_recommendations(id,school_id,post_id,actor_id,reason,publication_snapshot_id,post_revision,content_revision,idempotency_key,request_digest,event_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(recommendation).bind(s).bind(id).bind(a.id).bind(&input.reason).bind(input.publication_snapshot_id).bind(input.revision).bind(input.content_revision).bind(input.idempotency_key).bind(digest).bind(event).execute(&mut *tx).await?;
        crate::audit::record(&mut tx,a,s,"post.recommend",id,json!({"recommendation_id":recommendation,"event_id":event,"snapshot_id":input.publication_snapshot_id,"reason":input.reason})).await?;
        let value = record(&mut tx, s, id, event).await?;
        tx.commit().await?;
        Ok((value, true))
    }
    pub async fn recommendations(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        event: Option<Uuid>,
        page: Page,
    ) -> Result<Vec<Value>> {
        let (limit, offset) = page.fetch_bounds()?;
        let mut tx = self.0.begin().await?;
        crate::schools::south::adapter::postgres::permissions::authorize(
            &mut tx,
            a,
            s,
            "content.post.read",
        )
        .await?;
        let rows=sqlx::query_scalar(&format!("{VIEW} WHERE o.school_id=$1 AND o.post_id=$2 AND ($3::uuid IS NULL OR o.id=$3) ORDER BY o.created_at DESC,o.id DESC LIMIT $4 OFFSET $5"))
            .bind(s).bind(id).bind(event).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows)
    }
}
