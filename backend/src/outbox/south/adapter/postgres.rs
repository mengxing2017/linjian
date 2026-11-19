use crate::source_access::domain::{Fault, Result as SourceResult};
use crate::{
    business::{Actor, Error, Page, Result, text},
    outbox::{
        domain::{ReplayInput, recoverable, retry_seconds, retryable},
        south::port::{Delivery, DeliveryResult, OutboxStore},
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);

// Explicit public projection. No frozen body, signatures, credentials or upstream internal metadata.
const VIEW: &str = "SELECT o.id,o.post_id,o.sequence,o.payload->>'event_type' AS event_type,o.attempts,o.next_attempt_at,o.receipt_id,o.receipt_status,o.receipt_summary,o.last_error,o.schema_version,o.route_id,o.blocked_reason,o.accepted_at,o.created_at,o.frozen_at,o.binding_version,o.recovery_reason,CASE WHEN o.request_body IS NOT NULL THEN jsonb_build_object('content',o.payload->>'content','characters',length(o.payload->>'content'),'version',o.message_version) ELSE NULL END AS message,CASE WHEN p.event_id IS NOT NULL THEN jsonb_build_object('state',p.state,'revision',p.revision,'attempts',p.attempts,'next_attempt_at',p.next_attempt_at,'error_code',p.last_error_code,'recoverable',o.request_body IS NULL AND p.state IN ('blocked','retry_wait') AND forum_publication_is_current(o.id)) ELSE NULL END AS preparation,CASE WHEN o.attempts=0 AND o.delivery_state IN ('ready','retry_wait') AND o.receipt_id IS NULL AND o.payload->>'event_type'='content.upsert' AND b.active=false THEN 'paused' ELSE o.delivery_state END AS delivery_state FROM forum_outbox o LEFT JOIN school_source_bindings b ON b.school_id=o.school_id LEFT JOIN forum_publication_preparations p ON p.event_id=o.id";
fn public(mut v: Value) -> Result<Value> {
    v["recoverable"] = json!(recoverable(
        v["schema_version"].as_i64().unwrap_or(1) as i32,
        v["delivery_state"].as_str().unwrap_or("blocked"),
        v["receipt_status"].as_str(),
        v["blocked_reason"].as_str()
    ));
    let view: crate::outbox::domain::OutboxView =
        serde_json::from_value(v).map_err(|e| Error::Database(e.to_string()))?;
    serde_json::to_value(view).map_err(|e| Error::Database(e.to_string()))
}
async fn authorize(pool: &PgPool, actor: &Actor) -> Result<()> {
    actor.platform()?;
    let mut tx = pool.begin().await?;
    crate::schools::south::adapter::postgres::preview::platform(&mut tx, actor).await?;
    tx.commit().await?;
    Ok(())
}

impl Repository {
    pub async fn list(&self, actor: &Actor, school: Uuid, page: Page) -> Result<Vec<Value>> {
        authorize(&self.0, actor).await?;
        let (limit, offset) = page.fetch_bounds()?;
        if let Some(status) = &page.status {
            crate::business::require(
                matches!(
                    status.as_str(),
                    "waiting_binding"
                        | "waiting_configuration"
                        | "preparing_content"
                        | "paused"
                        | "ready"
                        | "in_flight"
                        | "retry_wait"
                        | "blocked"
                        | "accepted"
                        | "cancelled"
                ),
                "发件状态不合法",
            )?;
        }
        let sql = format!(
            "SELECT to_jsonb(v) FROM ({VIEW} WHERE o.school_id=$1) v WHERE ($4::text IS NULL OR v.delivery_state=$4) ORDER BY v.created_at DESC,v.id DESC LIMIT $2 OFFSET $3"
        );
        let rows: Vec<Value> = sqlx::query_scalar(&sql)
            .bind(school)
            .bind(limit)
            .bind(offset)
            .bind(page.status)
            .fetch_all(&self.0)
            .await?;
        rows.into_iter().map(public).collect()
    }
    pub async fn detail(&self, actor: &Actor, school: Uuid, id: Uuid) -> Result<Value> {
        authorize(&self.0, actor).await?;
        let mut value: Value = sqlx::query_scalar(&format!(
            "SELECT to_jsonb(v) FROM ({VIEW} WHERE o.school_id=$1 AND o.id=$2) v"
        ))
        .bind(school)
        .bind(id)
        .fetch_optional(&self.0)
        .await?
        .ok_or(Error::NotFound)?;
        let audit:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('actor_id',actor_id,'reason',detail->>'reason','created_at',created_at) FROM audit_events WHERE school_id=$1 AND object_id=$2 AND action IN ('source_event.replay','source_preparation.retry') ORDER BY created_at DESC,id DESC LIMIT 50").bind(school).bind(id).fetch_all(&self.0).await?;
        value["recoveries"] = json!(audit);
        let related:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'route_id',route_id,'sequence',sequence,'event_type',payload->>'event_type','receipt_status',receipt_status) FROM forum_outbox WHERE school_id=$1 AND post_id=(SELECT post_id FROM forum_outbox WHERE id=$2 AND school_id=$1) ORDER BY sequence DESC,id DESC LIMIT 50").bind(school).bind(id).fetch_all(&self.0).await?;
        value["related_events"] = json!(related);
        let publication:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('summary',s.summary,'summary_algorithm',s.summary_algorithm,'approved_post_revision',s.approved_post_revision,'link_type',coalesce(a.link_type,p.resolved_policy->>'link_type',s.link_policy->>'link_type'),'link',a.link,'expires_at',a.expires_at) FROM forum_outbox o JOIN forum_publication_snapshots s ON s.id=o.publication_snapshot_id LEFT JOIN forum_publication_preparations p ON p.event_id=o.id LEFT JOIN miniapp_link_assets a ON a.id=p.link_asset_id WHERE o.id=$1 AND o.school_id=$2").bind(id).bind(school).fetch_optional(&self.0).await?;
        if let Some(p) = publication {
            value["publication"] = p;
        }

        public(value)
    }
    pub async fn replay(
        &self,
        actor: &Actor,
        school: Uuid,
        id: Uuid,
        input: ReplayInput,
    ) -> Result<()> {
        authorize(&self.0, actor).await?;
        let reason = text(&input.reason, 1, 500)?;
        let mut tx = self.0.begin().await?;
        crate::schools::south::adapter::postgres::preview::platform(&mut tx, actor).await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_one(&mut *tx)
            .await?;
        let row=sqlx::query("SELECT o.*,v.origin,v.key_id,b.active AS binding_active FROM forum_outbox o LEFT JOIN source_connection_versions v ON v.id=o.connection_version LEFT JOIN school_source_bindings b ON b.school_id=o.school_id WHERE o.id=$1 AND o.school_id=$2 FOR UPDATE OF o").bind(id).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if row.get::<Option<String>, _>("request_body").is_none() {
            return Err(Error::Conflict);
        }
        // Repeated clicks while already scheduled/in flight do not revoke a lease or add another intent.
        let state: String = row.get("delivery_state");
        if state == "in_flight" {
            tx.commit().await?;
            return Ok(());
        }
        if !recoverable(
            row.get("schema_version"),
            &state,
            row.get::<Option<String>, _>("receipt_status").as_deref(),
            row.get::<Option<String>, _>("blocked_reason").as_deref(),
        ) {
            return Err(Error::Conflict);
        }
        if row.get::<i32, _>("attempts") == 0
            && row.get::<Option<bool>, _>("binding_active") == Some(false)
            && row.get::<Value, _>("payload")["event_type"] == "content.upsert"
            && row.get::<Option<String>, _>("receipt_id").is_none()
        {
            return Err(Error::Conflict);
        }
        let config = crate::source_access::configuration::ConnectionConfig::from_env()
            .map_err(|_| Error::Conflict)?
            .ok_or(Error::Conflict)?;
        config
            .material(row.get("origin"), row.get("key_id"))
            .map_err(|_| Error::Conflict)?;
        sqlx::query("UPDATE forum_outbox SET next_attempt_at=now(),blocked_reason=NULL,blocked_at=NULL,auth_failure_count=0,recovery_reason=$2,delivery_state='ready' WHERE id=$1").bind(id).bind(&reason).execute(&mut *tx).await?;
        crate::audit::record(
            &mut tx,
            actor,
            school,
            "source_event.replay",
            id,
            json!({"reason":reason,"previous_state":state}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
impl OutboxStore for Repository {
    async fn prepare(&self) -> SourceResult<bool> {
        let mut tx = self.0.begin().await?;
        // No post locks: content triggers may already own a post row when requesting the school lock.
        let school:Option<Uuid>=sqlx::query_scalar("SELECT s.id FROM schools s JOIN school_source_bindings b ON b.school_id=s.id JOIN source_active_configuration ac ON ac.connection_id=b.connection_id WHERE b.active AND s.active AND EXISTS(SELECT 1 FROM forum_outbox o WHERE o.school_id=s.id AND o.delivery_state IN ('waiting_binding','waiting_configuration')) ORDER BY s.id FOR UPDATE OF s SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await?;
        if let Some(school) = school {
            let row=sqlx::query("SELECT o.id,b.connection_version,b.route_id,bv.id AS binding_version FROM forum_outbox o JOIN school_source_bindings b ON b.school_id=o.school_id JOIN school_source_binding_versions bv ON bv.binding_id=b.id AND bv.revision=b.revision WHERE o.school_id=$1 AND o.delivery_state IN ('waiting_binding','waiting_configuration') ORDER BY o.sequence,o.created_at,o.id FOR UPDATE OF o SKIP LOCKED LIMIT 1").bind(school).fetch_optional(&mut *tx).await?;
            if let Some(row) = row {
                sqlx::query("SELECT forum_bind_prepared_source($1,$2,$3,$4)")
                    .bind(row.get::<Uuid, _>("id"))
                    .bind(row.get::<Uuid, _>("connection_version"))
                    .bind(row.get::<Uuid, _>("binding_version"))
                    .bind(row.get::<String, _>("route_id"))
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Ok(true);
            }
        }
        tx.commit().await?;
        Ok(false)
    }
    async fn claim(&self) -> SourceResult<Option<Delivery>> {
        // Frozen links cannot be refreshed by replaying or mutating an original event.
        sqlx::query("UPDATE forum_outbox o SET delivery_state='blocked',blocked_reason='link_expired_before_delivery',last_error='link_expired_before_delivery' FROM forum_publication_preparations p JOIN miniapp_link_assets a ON a.id=p.link_asset_id WHERE p.event_id=o.id AND o.attempts=0 AND o.receipt_id IS NULL AND o.lease_token IS NULL AND o.delivery_state IN ('ready','retry_wait') AND a.expires_at IS NOT NULL AND a.expires_at<=now()+make_interval(secs=>(p.resolved_policy->>'min_remaining')::integer)").execute(&self.0).await?;
        let mut tx = self.0.begin().await?;
        // Lock the school before the event so pause/configuration and claiming have a defined commit order.
        let school:Option<Uuid>=sqlx::query_scalar("SELECT s.id FROM schools s JOIN LATERAL (SELECT min(o.next_attempt_at) AS due FROM forum_outbox o LEFT JOIN school_source_bindings b ON b.school_id=o.school_id WHERE o.school_id=s.id AND o.schema_version=2 AND o.request_body IS NOT NULL AND o.blocked_reason IS NULL AND o.next_attempt_at<=now() AND (o.delivery_state IN ('ready','retry_wait') OR (o.delivery_state='in_flight' AND o.lease_until<now())) AND (o.attempts>0 OR (s.active AND coalesce(b.active,false)))) candidate ON candidate.due IS NOT NULL ORDER BY candidate.due,s.id FOR UPDATE OF s SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await?;
        let Some(school) = school else {
            return Ok(None);
        };
        let row=sqlx::query("SELECT o.*,v.origin,v.key_id FROM forum_outbox o JOIN source_connection_versions v ON v.id=o.connection_version LEFT JOIN school_source_bindings b ON b.school_id=o.school_id WHERE o.school_id=$1 AND o.schema_version=2 AND o.request_body IS NOT NULL AND o.blocked_reason IS NULL AND o.next_attempt_at<=now() AND (o.delivery_state IN ('ready','retry_wait') OR (o.delivery_state='in_flight' AND o.lease_until<now())) AND (o.attempts>0 OR ((SELECT active FROM schools WHERE id=$1) AND coalesce(b.active,false))) ORDER BY o.next_attempt_at,o.created_at,o.id FOR UPDATE OF o SKIP LOCKED LIMIT 1").bind(school).fetch_optional(&mut *tx).await?;
        let Some(row) = row else { return Ok(None) };
        let id: Uuid = row.get("id");
        let token = Uuid::new_v4();
        sqlx::query("UPDATE forum_outbox SET lease_token=$2,lease_until=now()+interval '30 seconds',delivery_state='in_flight',attempts=attempts+1 WHERE id=$1").bind(id).bind(token).execute(&mut *tx).await?;
        let delivery = Delivery {
            id,
            token,
            origin: row.get("origin"),
            key_id: row.get("key_id"),
            body: row
                .get::<Option<String>, _>("request_body")
                .ok_or(Fault("invalid_request"))?,
            body_sha256: row
                .get::<Option<String>, _>("body_sha256")
                .ok_or(Fault("invalid_request"))?,
            receipt_id: row
                .get::<Option<String>, _>("receipt_id")
                .map(|r| r.parse())
                .transpose()
                .map_err(|_| Fault("protocol_error"))?,
            attempts: row.get::<i32, _>("attempts") + 1,
            auth_failures: row.get("auth_failure_count"),
        };
        tx.commit().await?;
        Ok(Some(delivery))
    }
    async fn finish(&self, d: &Delivery, result: DeliveryResult) -> SourceResult<()> {
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT s.id FROM schools s JOIN forum_outbox o ON o.school_id=s.id WHERE o.id=$1 FOR UPDATE OF s").bind(d.id).fetch_one(&mut *tx).await?;
        match result {
            Ok(receipt) => {
                sqlx::query("UPDATE forum_outbox SET receipt_id=$3,receipt_status='received',receipt_summary=$4,last_error=NULL,blocked_reason=NULL,auth_failure_count=0,accepted_at=coalesce(accepted_at,now()),lease_token=NULL,lease_until=NULL,delivery_state='accepted' WHERE id=$1 AND lease_token=$2 AND lease_until>now()")
                    .bind(d.id).bind(d.token).bind(receipt.id.to_string()).bind(receipt.summary).execute(&mut *tx).await?;
            }
            Err((fault, retry_after)) => {
                let code = fault.0;
                let retry = retryable(code, d.auth_failures);
                let delay = (retry_seconds(d.attempts - 1).max(retry_after)
                    + (Uuid::new_v4().as_u128() % 5) as i64)
                    .min(3600);
                sqlx::query("UPDATE forum_outbox SET last_error=$3,blocked_reason=$4,blocked_at=CASE WHEN $4::text IS NULL THEN NULL ELSE now() END,next_attempt_at=now()+make_interval(secs=>$5),auth_failure_count=$6,lease_token=NULL,lease_until=NULL,delivery_state=CASE WHEN $4::text IS NULL THEN 'retry_wait' ELSE 'blocked' END WHERE id=$1 AND lease_token=$2 AND lease_until>now()").bind(d.id).bind(d.token).bind(code).bind(if retry {None}else{Some(code)}).bind(delay as f64).bind(if matches!(code,"timestamp_expired"|"replay_detected"){d.auth_failures+1}else{0}).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

impl Repository {
    pub async fn terminate(&self, a: &Actor, school: Uuid, id: Uuid, reason: &str) -> Result<()> {
        a.platform()?;
        let reason = text(reason, 1, 500)?;
        let mut tx = self.0.begin().await?;
        crate::schools::south::adapter::postgres::preview::platform(&mut tx, a).await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let row=sqlx::query("SELECT o.attempts,o.lease_token,o.delivery_state,o.receipt_id,p.lease_token AS preparation_lease,p.state AS preparation_state FROM forum_outbox o LEFT JOIN forum_publication_preparations p ON p.event_id=o.id WHERE o.id=$1 AND o.school_id=$2 FOR UPDATE OF o")
            .bind(id).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        let state: String = row.get("delivery_state");
        if state == "cancelled"
            && sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM forum_source_terminations WHERE event_id=$1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?
        {
            tx.commit().await?;
            return Ok(());
        }
        if row.get::<i32, _>("attempts") != 0
            || row.get::<Option<Uuid>, _>("lease_token").is_some()
            || row.get::<Option<Uuid>, _>("preparation_lease").is_some()
            || row.get::<Option<String>, _>("receipt_id").is_some()
            || !(state == "blocked"
                || row.get::<Option<String>, _>("preparation_state").as_deref() == Some("blocked"))
        {
            return Err(Error::Policy("source_termination_not_safe"));
        }
        sqlx::query(
            "INSERT INTO forum_source_terminations(event_id,actor_id,reason) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(a.id)
        .bind(&reason)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE forum_outbox SET delivery_state='cancelled' WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        crate::audit::record(
            &mut tx,
            a,
            school,
            "post.source_terminated",
            id,
            json!({"reason":reason,"evidence":"never_attempted_no_lease"}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
