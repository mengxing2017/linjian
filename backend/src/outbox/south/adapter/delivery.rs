use crate::source_access::south::port::CredentialStore;
use crate::{
    business::Result,
    source_access::{domain::*, north::local::request, south::adapter::postgres::Repository},
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Worker {
    pool: PgPool,
}
impl Worker {
    pub fn from_env(pool: PgPool) -> Result<Option<Self>> {
        Ok(Some(Self { pool }))
    }
    pub async fn run_once(&self) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let row=sqlx::query("SELECT * FROM forum_outbox WHERE schema_version=2 AND blocked_reason IS NULL AND next_attempt_at<=now() AND coalesce(receipt_status,'') NOT IN ('ignored_stale','rejected') ORDER BY next_attempt_at,created_at,id FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await?;
        let Some(row) = row else { return Ok(false) };
        let id: Uuid = row.get("id");
        let conn=sqlx::query("SELECT c.id,c.key_id,c.source_id,v.origin FROM source_connection_versions v JOIN source_connections c ON c.id=v.connection_id WHERE v.id=$1 AND v.source_id=c.source_id").bind(row.get::<Uuid,_>("connection_version")).fetch_one(&mut *tx).await?;
        let body = match row.get::<Option<String>, _>("request_body") {
            Some(body) => body,
            None => {
                let public = std::env::var("LINJIAN_PUBLIC_ORIGIN").unwrap_or_default();
                let public = match origin(&public) {
                    Ok(u) => u,
                    Err(_) => {
                        sqlx::query("UPDATE forum_outbox SET blocked_reason='public_origin_invalid',blocked_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await?;
                        tx.commit().await?;
                        return Ok(true);
                    }
                };
                let mut payload: Value = row.get("payload");
                payload["detail_url"] = Value::String(format!(
                    "{}/community/{}/posts/{}",
                    public.origin().ascii_serialization(),
                    row.get::<Uuid, _>("school_id"),
                    row.get::<Uuid, _>("post_id")
                ));
                let body=serde_json::to_string(&serde_json::json!({"data":{"type":"source-events","id":id,"attributes":payload}})).map_err(|_|crate::business::Error::Unavailable)?;
                sqlx::query("UPDATE forum_outbox SET request_body=$2,body_sha256=$3 WHERE id=$1")
                    .bind(id)
                    .bind(&body)
                    .bind(digest(body.as_bytes()))
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Ok(true);
            }
        };
        let receipt: Option<Uuid> = row
            .get::<Option<String>, _>("receipt_id")
            .and_then(|s| s.parse().ok());
        let result = async {
            let raw = Repository(self.pool.clone())
                .load_material(conn.get("id"))
                .await?;
            let base: String = conn.get("origin");
            let key: String = conn.get("key_id");
            let (method, path, bytes) = if let Some(r) = receipt {
                (
                    "GET",
                    format!("/api/v2/source-receipts/{r}"),
                    b"".as_slice(),
                )
            } else {
                ("POST", "/api/v2/source-events".into(), body.as_bytes())
            };
            let response = request(&base, &key, &raw, method, &path, id, bytes).await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
                .clamp(0, 3600);
            ensure(
                response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.split(';').next() == Some(MEDIA_TYPE)),
                "protocol_error",
            )?;
            let document: Value = response.json().await.map_err(|_| Fault("protocol_error"))?;
            if !status.is_success() {
                let code = document["errors"][0]["code"]
                    .as_str()
                    .unwrap_or("protocol_error");
                let code = match code {
                    "timestamp_expired" => "timestamp_expired",
                    "replay_detected" => "replay_detected",
                    "unsupported_version" => "unsupported_version",
                    "authentication_failed" => "authentication_failed",
                    "credential_unavailable" => "credential_unavailable",
                    "source_disabled" => "source_disabled",
                    "route_unavailable" => "route_unavailable",
                    "event_conflict" => "event_conflict",
                    _ if status.as_u16() == 429 => "rate_limited",
                    _ if status.is_server_error() => "temporarily_unavailable",
                    _ => "invalid_request",
                };
                return Ok::<_, Fault>(Err((code, retry_after)));
            }
            let r: Document<Receipt> =
                serde_json::from_value(document).map_err(|_| Fault("protocol_error"))?;
            ensure(
                r.data.kind == "source-receipts"
                    && receipt.is_none_or(|id| id == r.data.id)
                    && r.data.attributes.event_id == id
                    && matches!(
                        r.data.attributes.status.as_str(),
                        "received" | "applied" | "ignored_stale" | "rejected"
                    ),
                "protocol_error",
            )?;
            Ok(Ok((
                r.data.id,
                r.data.attributes.status.clone(),
                serde_json::json!(r.data.attributes),
            )))
        }
        .await;
        match result {
            Ok(Ok((receipt, status, summary))) => {
                sqlx::query("UPDATE forum_outbox SET attempts=attempts+1,receipt_id=$2,receipt_status=$3,receipt_summary=$4,last_error=NULL,auth_failure_count=0,accepted_at=coalesce(accepted_at,now()),next_attempt_at=now()+interval '30 seconds' WHERE id=$1").bind(id).bind(receipt.to_string()).bind(status).bind(summary).execute(&mut *tx).await?;
            }
            other => {
                let (code, retry_after) = match other {
                    Ok(Err(e)) => e,
                    Err(e) => (e.0, 0),
                    _ => unreachable!(),
                };
                let retry = matches!(
                    code,
                    "rate_limited"
                        | "temporarily_unavailable"
                        | "timestamp_expired"
                        | "replay_detected"
                        | "protocol_error"
                );
                let retry = retry
                    && !(matches!(code, "timestamp_expired" | "replay_detected")
                        && row.get::<i32, _>("auth_failure_count") >= 2);
                let blocked = if retry { None } else { Some(code) };
                let delay = crate::outbox::domain::retry_seconds(row.get("attempts"))
                    .max(retry_after)
                    + (Uuid::new_v4().as_u128() % 5) as i64;
                sqlx::query("UPDATE forum_outbox SET attempts=attempts+1,last_error=$2,blocked_reason=$3,blocked_at=CASE WHEN $3::text IS NULL THEN NULL ELSE now() END,next_attempt_at=now()+make_interval(secs=>$4),auth_failure_count=$5 WHERE id=$1").bind(id).bind(code).bind(blocked).bind(delay.min(3600) as f64).bind(if matches!(code,"timestamp_expired"|"replay_detected"){row.get::<i32,_>("auth_failure_count")+1}else{0}).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(true)
    }
}
#[derive(serde::Serialize, Deserialize)]
struct Receipt {
    event_id: Uuid,
    status: String,
    reason_code: Option<String>,
    processing_stage: String,
    received_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    retryable: bool,
    sending: Option<crate::outbox::domain::SendingSummary>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_receipt_preserves_applied_without_claiming_sent() {
        let doc: Document<Receipt> = serde_json::from_str(include_str!(
            "../../../../../api-contract/fixtures/source-receipt-v2.json"
        ))
        .unwrap();
        assert_eq!(doc.data.kind, "source-receipts");
        assert_eq!(doc.data.attributes.status, "applied");
        assert!(doc.data.attributes.sending.is_none());
    }
}
