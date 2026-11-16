use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn retry_seconds(attempts: i32) -> i64 {
    5_i64
        .saturating_mul(2_i64.pow(attempts.clamp(0, 10) as u32))
        .min(3600)
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
            "accepted"
                | "cancelled"
                | "waiting_binding"
                | "waiting_configuration"
                | "paused"
                | "in_flight"
                | "preparing_content"
        )
        && receipt.is_none()
        && !matches!(
            reason,
            Some(
                "link_expired_before_delivery"
                    | "snapshot_incomplete"
                    | "event_conflict"
                    | "body_digest_mismatch"
                    | "unsupported_version"
                    | "invalid_request"
                    | "forbidden_field"
                    | "content_empty"
                    | "content_too_long"
                    | "unsupported_event_type"
                    | "event_identity_mismatch"
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
    pub recovery_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
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
        ] {
            assert!(!recoverable(2, "blocked", None, Some(reason)));
        }
        for state in [
            "accepted",
            "cancelled",
            "paused",
            "waiting_binding",
            "waiting_configuration",
            "in_flight",
        ] {
            assert!(!recoverable(2, state, None, None));
        }
        assert!(!recoverable(1, "blocked", None, None));
        assert!(!recoverable(2, "blocked", Some("received"), None));
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
