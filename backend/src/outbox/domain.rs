use crate::business::{Error, Result, require};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MEDIA_TYPE: &str = "application/vnd.api+json";
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvent {
    pub event_id: Uuid,
    pub schema_version: u32,
    pub event_type: String,
    pub external_organization_id: Uuid,
    pub external_content_id: Uuid,
    pub version: String,
    pub sequence: i64,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub title: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub detail_url: Option<String>,
}
impl SourceEvent {
    pub fn document(mut self, origin: &str) -> Result<String> {
        require(
            self.schema_version == 1 && self.sequence > 0,
            "事件版本无效",
        )?;
        require(
            matches!(
                self.event_type.as_str(),
                "content.upsert" | "content.withdrawn"
            ),
            "事件类型无效",
        )?;
        if self.event_type == "content.upsert" {
            require(
                self.title.as_ref().is_some_and(|v| !v.is_empty())
                    && self.content.as_ref().is_some_and(|v| !v.is_empty()),
                "缺少正文",
            )?;
        }
        let origin =
            url::Url::parse(origin).map_err(|_| Error::Invalid("论坛公开地址无效".into()))?;
        require(
            origin.scheme() == "https" && origin.host_str().is_some(),
            "推送详情地址必须使用 HTTPS",
        )?;
        self.detail_url = Some(format!(
            "{}/community/{}/posts/{}",
            origin.origin().ascii_serialization(),
            self.external_organization_id,
            self.external_content_id
        ));
        serde_json::to_string(&serde_json::json!({"data":{"type":"source-events","id":self.event_id,"attributes":self}})).map_err(|_| Error::Unavailable)
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SendingSummary {
    pub pending: u64,
    pub sent: u64,
    pub failed: u64,
    pub unknown: u64,
}

/// Only allow the business summary into persistence; discard other upstream fields.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReceiptAttributes {
    pub status: String,
    pub sending: Option<SendingSummary>,
}

/// UTF-8 fields separated by LF, no trailing LF. Hex digests are lowercase.
pub fn canonical(
    key_id: &str,
    method: &str,
    path: &str,
    timestamp: &str,
    nonce: &str,
    idempotency: &str,
    body: &[u8],
) -> String {
    format!(
        "1\n{key_id}\n{method}\n{path}\n{timestamp}\n{nonce}\n{idempotency}\n{:x}",
        Sha256::digest(body)
    )
}
pub fn sign(key: &[u8], canonical: &str) -> String {
    let tag = ring::hmac::sign(
        &ring::hmac::Key::new(ring::hmac::HMAC_SHA256, key),
        canonical.as_bytes(),
    );
    tag.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
pub fn retry_seconds(attempts: i32) -> i64 {
    5_i64
        .saturating_mul(2_i64.pow(attempts.clamp(0, 10) as u32))
        .min(3600)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_independently_generated_utf8_signing_vector() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../api-contract/fixtures/source-event-v1.json"
        ))
        .unwrap();
        let event: SourceEvent = serde_json::from_value(fixture["event"].clone()).unwrap();
        let body = event.document(fixture["origin"].as_str().unwrap()).unwrap();
        assert_eq!(body, fixture["body"].as_str().unwrap());
        let canonical = canonical(
            "fixture-key",
            "POST",
            "/api/v1/source-events",
            "1794096000",
            "fixture-nonce",
            "00000000-0000-0000-0000-000000000010",
            body.as_bytes(),
        );
        assert_eq!(canonical, fixture["canonical"].as_str().unwrap());
        assert_eq!(
            sign(fixture["secret"].as_str().unwrap().as_bytes(), &canonical),
            fixture["signature"].as_str().unwrap()
        );
        assert_ne!(
            sign(b"wrong-key", &canonical),
            fixture["signature"].as_str().unwrap()
        );
    }
    #[test]
    fn validates_policy_and_bounds_retries() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../api-contract/fixtures/source-event-v1.json"
        ))
        .unwrap();
        let mut event: SourceEvent = serde_json::from_value(fixture["event"].clone()).unwrap();
        event.schema_version = 2;
        assert!(event.document("https://forum.example.com").is_err());
        let event: SourceEvent = serde_json::from_value(fixture["event"].clone()).unwrap();
        assert!(event.document("http://forum.example.com").is_err());
        assert_eq!(retry_seconds(0), 5);
        assert_eq!(retry_seconds(1), 10);
        assert_eq!(retry_seconds(i32::MAX), 3600);
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayInput {
    pub reason: String,
}

pub fn recoverable(schema: i32, state: &str, receipt: Option<&str>, reason: Option<&str>) -> bool {
    schema == 2
        && !matches!(
            state,
            "settled"
                | "superseded"
                | "waiting_binding"
                | "waiting_configuration"
                | "paused"
                | "in_flight"
                | "preparing_content"
        )
        && !matches!(receipt, Some("rejected" | "ignored_stale"))
        && !matches!(
            reason,
            Some(
                "legacy_version_blocked"
                    | "link_expired_before_delivery"
                    | "snapshot_incomplete"
                    | "event_conflict"
                    | "body_digest_mismatch"
                    | "unsupported_version"
                    | "invalid_request"
            )
        )
}

pub fn retryable(code: &str, auth_failures: i32) -> bool {
    matches!(
        code,
        "rate_limited" | "temporarily_unavailable" | "protocol_error"
    ) || (matches!(code, "timestamp_expired" | "replay_detected") && auth_failures < 2)
}

/// The local school view is independent of the upstream receipt protocol.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutboxView {
    pub id: Uuid,
    pub post_id: Uuid,
    pub sequence: i64,
    pub event_type: String,
    pub attempts: i32,
    pub next_attempt_at: chrono::DateTime<chrono::Utc>,
    pub receipt_id: Option<String>,
    pub receipt_status: Option<String>,
    pub receipt_summary: Option<serde_json::Value>,
    pub last_error: Option<String>,
    pub schema_version: i32,
    pub route_id: Option<String>,
    pub blocked_reason: Option<String>,
    pub accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub frozen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub binding_version: Option<Uuid>,
    pub superseded_by: Option<Uuid>,
    pub recovery_reason: Option<String>,
    pub delivery_state: String,
    pub recoverable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preparation: Option<PreparationView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoveries: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_events: Option<Vec<serde_json::Value>>,
}
#[cfg(test)]
mod local_contract_tests {
    use super::*;
    #[test]
    fn local_outbox_fixture_matches_the_public_view() {
        for raw in [
            include_str!("../../../api-contract/fixtures/source-outbox.json"),
            include_str!("../../../api-contract/fixtures/source-outbox-preparation.json"),
        ] {
            let fixture: serde_json::Value = serde_json::from_str(raw).unwrap();
            let mut attributes = fixture["data"]["attributes"].clone();
            attributes["id"] = fixture["data"]["id"].clone();
            let view: OutboxView = serde_json::from_value(attributes.clone()).unwrap();
            assert_eq!(serde_json::to_value(view).unwrap(), attributes);
        }
    }
    #[test]
    fn recovery_does_not_unblock_terminal_or_corrupt_events() {
        for reason in [
            "event_conflict",
            "body_digest_mismatch",
            "unsupported_version",
            "invalid_request",
            "legacy_version_blocked",
        ] {
            assert!(!recoverable(2, "blocked", None, Some(reason)));
        }
        for state in [
            "settled",
            "superseded",
            "paused",
            "waiting_binding",
            "waiting_configuration",
            "in_flight",
        ] {
            assert!(!recoverable(2, state, None, None));
        }
        assert!(!recoverable(1, "blocked", None, None));
        assert!(!recoverable(2, "blocked", Some("rejected"), None));
        assert!(recoverable(
            2,
            "blocked",
            None,
            Some("credential_unavailable")
        ));
        assert!(retryable("timestamp_expired", 1));
        assert!(!retryable("timestamp_expired", 2));
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationView {
    pub state: String,
    pub revision: i64,
    pub attempts: i32,
    pub next_attempt_at: chrono::DateTime<chrono::Utc>,
    pub error_code: Option<String>,
    pub recoverable: bool,
}
