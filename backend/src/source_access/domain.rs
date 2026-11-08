//! Published source-v2 wire and byte rules. No HTTP or database dependencies.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
pub const MEDIA_TYPE: &str = "application/vnd.api+json";
pub const MAX_BODY: usize = 1_048_576;
pub type Result<T> = std::result::Result<T, Fault>;
#[derive(Debug)]
pub struct Fault(pub &'static str);
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Document<T> {
    pub data: Resource<T>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Resource<T> {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: Uuid,
    pub attributes: T,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub event_id: Uuid,
    pub schema_version: u32,
    pub route_id: String,
    pub event_type: String,
    pub external_content_id: String,
    pub version: String,
    pub sequence: i64,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub detail_url: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub route_id: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}
pub fn ensure(ok: bool, code: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(Fault(code)) }
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn canonical(
    key: &str,
    method: &str,
    path: &str,
    timestamp: &str,
    nonce: &str,
    identity: &str,
    body: &[u8],
) -> String {
    format!(
        "2\n{key}\n{method}\n{path}\n{timestamp}\n{nonce}\n{identity}\n{}",
        digest(body)
    )
}
pub fn sign(secret: &[u8], text: &str) -> String {
    ring::hmac::sign(
        &ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret),
        text.as_bytes(),
    )
    .as_ref()
    .iter()
    .map(|v| format!("{v:02x}"))
    .collect()
}
pub fn verify(secret: &[u8], text: &str, hex: &str) -> Result<()> {
    ensure(
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "authentication_failed",
    )?;
    let bytes: Vec<u8> = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    ring::hmac::verify(
        &ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret),
        text.as_bytes(),
        &bytes,
    )
    .map_err(|_| Fault("authentication_failed"))
}
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
pub fn encode(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for b in bytes {
        acc = (acc << 8) | u32::from(*b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            result.push(ALPHABET[((acc >> bits) & 63) as usize] as char);
        }
    }
    if bits > 0 {
        result.push(ALPHABET[((acc << (6 - bits)) & 63) as usize] as char);
    }
    result
}
pub fn decode(value: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for b in value.bytes() {
        let n = ALPHABET
            .iter()
            .position(|v| *v == b)
            .ok_or(Fault("invalid_secret"))?;
        acc = (acc << 6) | n as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    ensure(encode(&out) == value, "invalid_secret")?;
    Ok(out)
}
pub fn secret(value: &str) -> Result<Vec<u8>> {
    let bytes = decode(value)?;
    ensure(bytes.len() >= 32, "invalid_secret")?;
    Ok(bytes)
}
pub fn window(timestamp: &str, now: i64) -> Result<i64> {
    let seconds = timestamp
        .parse::<i64>()
        .map_err(|_| Fault("invalid_request"))?;
    ensure(
        seconds.to_string() == timestamp && seconds >= 0,
        "invalid_request",
    )?;
    ensure(seconds.abs_diff(now) <= 300, "timestamp_expired")?;
    Ok(seconds)
}
pub fn origin(value: &str) -> Result<url::Url> {
    let u = url::Url::parse(value).map_err(|_| Fault("invalid_request"))?;
    ensure(
        u.scheme() == "https"
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
            && u.fragment().is_none()
            && u.query().is_none(),
        "invalid_request",
    )?;
    Ok(u)
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
impl Event {
    pub fn validate(&self) -> Result<()> {
        ensure(self.schema_version == 2, "unsupported_version")?;
        ensure(
            identifier(&self.route_id)
                && !self.external_content_id.is_empty()
                && self.external_content_id.len() <= 256
                && !self.version.is_empty()
                && self.version.len() <= 256
                && (1..=9_007_199_254_740_991).contains(&self.sequence),
            "invalid_request",
        )?;
        ensure(
            matches!(
                self.event_type.as_str(),
                "content.upsert" | "content.withdrawn"
            ),
            "invalid_request",
        )?;
        if self.event_type == "content.upsert" {
            ensure(
                self.title.as_ref().is_some_and(|v| !v.trim().is_empty())
                    && self.content.as_ref().is_some_and(|v| !v.trim().is_empty()),
                "invalid_request",
            )?;
        } else {
            ensure(self.content.is_none(), "invalid_request")?;
        }
        ensure(
            self.title.as_ref().is_none_or(|v| v.chars().count() <= 512)
                && self
                    .content
                    .as_ref()
                    .is_none_or(|v| v.chars().count() <= 100_000)
                && self.detail_url.len() <= 2048,
            "invalid_request",
        )?;
        origin(&self.detail_url)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_vectors() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../api-contract/fixtures/source-v2.json"
        ))
        .unwrap();
        for v in fixtures["vectors"].as_array().unwrap() {
            let body = v["body"].as_str().unwrap().as_bytes();
            let text = canonical(
                v["key_id"].as_str().unwrap(),
                v["method"].as_str().unwrap(),
                v["path"].as_str().unwrap(),
                v["timestamp"].as_str().unwrap(),
                v["nonce"].as_str().unwrap(),
                v["identity"].as_str().unwrap(),
                body,
            );
            assert_ne!(digest(&[body, b"!"].concat()), v["digest"]);
            assert_eq!(digest(body), v["digest"]);
            assert_eq!(text, v["canonical"]);
            let key = secret(v["secret"].as_str().unwrap()).unwrap();
            assert_eq!(sign(&key, &text), v["signature"]);
            verify(&key, &text, v["signature"].as_str().unwrap()).unwrap();
            assert!(verify(&key, &format!("{text}\n"), v["signature"].as_str().unwrap()).is_err());
        }
    }
    #[test]
    fn window_and_encoding() {
        for n in [-300, 300] {
            assert!(window(&(1000 + n).to_string(), 1000).is_ok());
        }
        for n in [-301, 301] {
            assert!(window(&(1000 + n).to_string(), 1000).is_err());
        }
        assert!(window("01000", 1000).is_err());
        assert!(secret("short").is_err());
        for n in 0..100 {
            let v = vec![255; n];
            assert_eq!(decode(&encode(&v)).unwrap(), v);
        }
    }
}
#[cfg(test)]
mod boundary_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn strict_envelope_forbids_execution_and_school_fields() {
        let id = Uuid::nil();
        let attributes = json!({"event_id":id,"schema_version":2,"route_id":"route-1","event_type":"content.upsert","external_content_id":"post","version":"1","sequence":1,"occurred_at":"2026-11-08T00:00:00Z","title":"中文","content":"正文","detail_url":"https://forum.example.com/community/a/posts/1"});
        let doc = json!({"data":{"type":"source-events","id":id,"attributes":attributes}});
        let valid: Document<Event> = serde_json::from_value(doc.clone()).unwrap();
        valid.data.attributes.validate().unwrap();
        for field in [
            "school_id",
            "school_name",
            "external_organization_id",
            "channel",
            "group_ids",
            "device_id",
            "sending_account",
            "ad_id",
            "ad_pool_id",
        ] {
            let mut bad = doc.clone();
            bad["data"]["attributes"][field] = json!("injected");
            assert!(
                serde_json::from_value::<Document<Event>>(bad).is_err(),
                "{field}"
            );
        }
        let mut bad = doc.clone();
        bad["data"]["relationships"] = json!({});
        assert!(serde_json::from_value::<Document<Event>>(bad).is_err());
        let mut bad = doc;
        bad["included"] = json!([]);
        assert!(serde_json::from_value::<Document<Event>>(bad).is_err());
    }
    #[test]
    fn rejects_unsafe_endpoint_components() {
        for value in [
            "http://forum.example.com",
            "https://user:pass@forum.example.com",
            "https://forum.example.com/#fragment",
            "https://forum.example.com/?next=evil",
        ] {
            assert!(origin(value).is_err());
        }
    }
}
