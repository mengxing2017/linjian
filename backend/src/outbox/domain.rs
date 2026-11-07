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
