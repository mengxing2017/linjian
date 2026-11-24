use crate::business::{Result, require};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    pub kind: String,
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub code: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub ticket: String,
    #[serde(default)]
    pub acceptances: Vec<Acceptance>,
    pub school_id: Option<Uuid>,
    pub request_key: Option<Uuid>,
}
impl Command {
    pub fn validate(&self) -> Result<()> {
        require(
            self.ticket.len() == 64 && self.ticket.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "入驻凭据不合法",
        )?;
        require(self.acceptances.len() <= 2, "协议类型重复")?;
        let mut kinds = std::collections::BTreeSet::new();
        for value in &self.acceptances {
            crate::legal_documents::domain::kind(&value.kind)?;
            require(
                value.revision > 0 && kinds.insert(&value.kind),
                "协议版本或类型不合法",
            )?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_spoofed_identity_duplicate_versions_and_invalid_tickets() {
        let valid =
            json!({"ticket":"a".repeat(64),"acceptances":[{"kind":"privacy_policy","revision":2}]});
        assert!(
            serde_json::from_value::<Command>(valid.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for invalid in [
            json!({"ticket":"x".repeat(64)}),
            json!({"ticket":"a".repeat(63)}),
            json!({"ticket":"a".repeat(64),"acceptances":[{"kind":"privacy_policy","revision":0}]}),
            json!({"ticket":"a".repeat(64),"acceptances":[{"kind":"privacy_policy","revision":1},{"kind":"privacy_policy","revision":2}]}),
        ] {
            assert!(
                serde_json::from_value::<Command>(invalid)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        for field in ["openid", "account_id", "first_login", "completed"] {
            let mut spoofed = valid.clone();
            spoofed[field] = json!(true);
            assert!(serde_json::from_value::<Command>(spoofed).is_err());
        }
    }
}
