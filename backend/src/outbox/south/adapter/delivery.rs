//! Server-only HTTP adapter. Persisted bytes, identity and receipt survive every retry.
use crate::{
    outbox::south::port::{Delivery, DeliveryResult, ReceiptResult, SourceTransport},
    source_access::{configuration::ConnectionConfig, domain::*, north::local::request},
};
use serde::{Deserialize, Serialize};

pub struct HttpTransport;
impl SourceTransport for HttpTransport {
    async fn deliver(&self, d: &Delivery) -> DeliveryResult {
        let result = async {
            ensure(
                digest(d.body.as_bytes()) == d.body_sha256,
                "body_digest_mismatch",
            )?;
            let event: Document<Event> =
                serde_json::from_str(&d.body).map_err(|_| Fault("invalid_request"))?;
            event.data.attributes.validate()?;
            ensure(
                event.data.kind == "source-events"
                    && event.data.id == d.id
                    && event.data.attributes.event_id == d.id
                    && d.body.len() <= MAX_BODY,
                "invalid_request",
            )?;
            let secret = ConnectionConfig::from_env()?
                .ok_or(Fault("wekeeper_not_configured"))?
                .material(&d.origin, &d.key_id)?;
            let (method, path, bytes) = if let Some(receipt) = d.receipt_id {
                (
                    "GET",
                    format!("/api/v2/source-receipts/{receipt}"),
                    b"".as_slice(),
                )
            } else {
                ("POST", "/api/v2/source-events".into(), d.body.as_bytes())
            };
            let response =
                request(&d.origin, &d.key_id, &secret, method, &path, d.id, bytes).await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
                .clamp(0, 3600);
            let media = response
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .to_owned();
            // Stream a bounded response rather than accepting an unbounded error/HTML page.
            let mut response = response;
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| Fault("temporarily_unavailable"))?
            {
                ensure(
                    bytes.len().saturating_add(chunk.len()) <= MAX_BODY,
                    "protocol_error",
                )?;
                bytes.extend_from_slice(&chunk);
            }
            Ok((status.as_u16(), media, bytes, retry_after))
        }
        .await;
        match result {
            Ok((status, media, bytes, delay)) => {
                parse_response(d, status, &media, &bytes).map_err(|f| (f, delay))
            }
            Err(f) => Err((f, 0)),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    event_id: uuid::Uuid,
    status: String,
    received_at: chrono::DateTime<chrono::Utc>,
}
pub fn parse_response(
    d: &Delivery,
    status: u16,
    media: &str,
    bytes: &[u8],
) -> Result<ReceiptResult> {
    ensure(
        media.split(';').next().map(str::trim) == Some(MEDIA_TYPE),
        "protocol_error",
    )?;
    let document: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| Fault("protocol_error"))?;
    if !(200..300).contains(&status) {
        let code = document["errors"][0]["code"]
            .as_str()
            .unwrap_or("protocol_error");
        return Err(Fault(match code {
            "timestamp_expired" => "timestamp_expired",
            "replay_detected" => "replay_detected",
            "unsupported_version" => "unsupported_version",
            "authentication_failed" => "authentication_failed",
            "credential_unavailable" => "credential_unavailable",
            "source_disabled" => "source_disabled",
            "route_unavailable" => "route_unavailable",
            "event_conflict" => "event_conflict",
            _ if status == 429 => "rate_limited",
            _ if status >= 500 => "temporarily_unavailable",
            "invalid_request" => "invalid_request",
            "forbidden_field" => "forbidden_field",
            "content_empty" => "content_empty",
            "content_too_long" => "content_too_long",
            "unsupported_event_type" => "unsupported_event_type",
            "event_identity_mismatch" => "event_identity_mismatch",
            "receipt_not_found" => "receipt_not_found",
            "unsupported_media_type" => "unsupported_media_type",
            _ => {
                if code.len() <= 64 && code.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                {
                    tracing::warn!(upstream_status = status, upstream_code = code, event_id = %d.id, "source request rejected");
                }
                "upstream_error"
            }
        }));
    }
    ensure(matches!(status, 200 | 202), "protocol_error")?;
    let r: Document<Receipt> =
        serde_json::from_value(document).map_err(|_| Fault("protocol_error"))?;
    ensure(
        r.data.kind == "source-receipts"
            && d.receipt_id.is_none_or(|id| id == r.data.id)
            && r.data.attributes.event_id == d.id
            && r.data.attributes.status == "received",
        "protocol_error",
    )?;
    Ok(ReceiptResult {
        id: r.data.id,
        status: r.data.attributes.status.clone(),
        summary: serde_json::to_value(r.data.attributes).map_err(|_| Fault("protocol_error"))?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn delivery() -> Delivery {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../api-contract/fixtures/source-receipt-v2.json"
        ))
        .unwrap();
        Delivery {
            id: doc["data"]["attributes"]["event_id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            token: uuid::Uuid::new_v4(),
            origin: String::new(),
            key_id: String::new(),
            body: String::new(),
            body_sha256: String::new(),
            receipt_id: None,
            attempts: 1,
            auth_failures: 0,
        }
    }
    #[test]
    fn receipt_identity_and_media_are_required() {
        let mut d = delivery();
        let bytes = include_bytes!("../../../../../api-contract/fixtures/source-receipt-v2.json");
        let result = parse_response(&d, 200, MEDIA_TYPE, bytes).unwrap();
        assert_eq!(result.status, "received");
        assert_eq!(result.summary.as_object().unwrap().len(), 3);
        assert!(parse_response(&d, 200, "text/html", b"<html>OK</html>").is_err());
        d.receipt_id = Some(uuid::Uuid::new_v4());
        assert!(parse_response(&d, 200, MEDIA_TYPE, bytes).is_err());
        d.receipt_id = None;
        d.id = uuid::Uuid::new_v4();
        assert!(parse_response(&d, 200, MEDIA_TYPE, bytes).is_err());
        let valid: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        for (path, value) in [("type", "other"), ("id", "bad-uuid")] {
            let mut bad = valid.clone();
            bad["data"][path] = serde_json::json!(value);
            assert!(
                parse_response(
                    &delivery(),
                    200,
                    MEDIA_TYPE,
                    &serde_json::to_vec(&bad).unwrap()
                )
                .is_err()
            );
        }
    }
    #[test]
    fn minimal_acceptance_and_precise_protocol_errors() {
        let d = delivery();
        let bytes = include_bytes!("../../../../../api-contract/fixtures/source-receipt-v2.json");
        for status in [200, 202] {
            assert_eq!(
                parse_response(&d, status, MEDIA_TYPE, bytes)
                    .unwrap()
                    .status,
                "received"
            );
        }
        assert_eq!(
            parse_response(&d, 204, MEDIA_TYPE, bytes).err().unwrap().0,
            "protocol_error"
        );
        for code in [
            "forbidden_field",
            "content_empty",
            "content_too_long",
            "unsupported_event_type",
            "event_identity_mismatch",
            "receipt_not_found",
            "unsupported_media_type",
        ] {
            let bytes = serde_json::to_vec(&serde_json::json!({"errors":[{"code":code}]})).unwrap();
            assert_eq!(
                parse_response(&d, 400, MEDIA_TYPE, &bytes).err().unwrap().0,
                code
            );
        }
        let bytes = br#"{"errors":[{"code":"unexpected_internal_failure"}]}"#;
        assert_eq!(
            parse_response(&d, 400, MEDIA_TYPE, bytes).err().unwrap().0,
            "upstream_error"
        );
        assert_eq!(
            parse_response(&d, 429, MEDIA_TYPE, bytes).err().unwrap().0,
            "rate_limited"
        );
        assert_eq!(
            parse_response(&d, 503, MEDIA_TYPE, bytes).err().unwrap().0,
            "temporarily_unavailable"
        );
    }
}
