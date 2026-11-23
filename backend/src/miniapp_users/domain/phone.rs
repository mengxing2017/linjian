use crate::business::{Actor, Error, Result, SessionKind};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const NOTICE_VERSION: &str = "linjian-phone-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhoneBinding {
    pub id: Uuid,
    pub status: String,
    pub masked_phone: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub revision: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindPhoneInput {
    pub code: String,
    pub request_id: Uuid,
    pub revision: i64,
    pub notice_version: String,
}

impl BindPhoneInput {
    pub fn validate(&self) -> Result<()> {
        if self.code.is_empty()
            || self.code.len() > 512
            || self.code.chars().any(char::is_whitespace)
            || self.code.chars().any(char::is_control)
            || self.request_id.is_nil()
            || self.revision < 0
            || self.notice_version != NOTICE_VERSION
        {
            return Err(Error::Phone("phone_authorization_invalid"));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokePhoneInput {
    pub revision: i64,
}

pub struct VerifiedPhone {
    pub(crate) number: String,
    pub(crate) last4: String,
    pub(crate) app_id: String,
    pub(crate) verified_at: DateTime<Utc>,
}

impl VerifiedPhone {
    pub fn from_wechat(
        value: &serde_json::Value,
        app_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Self> {
        let invalid = || Error::Phone("phone_authorization_invalid");
        if value
            .get("errcode")
            .is_some_and(|code| code.as_i64() != Some(0))
        {
            return Err(invalid());
        }
        let info = &value["phone_info"];
        if info["watermark"]["appid"].as_str() != Some(app_id) {
            return Err(invalid());
        }
        if info["countryCode"].as_str() != Some("86") {
            return Err(Error::Phone("phone_region_unsupported"));
        }
        let number = info["purePhoneNumber"].as_str().ok_or_else(invalid)?;
        if number.len() != 11
            || !number.starts_with('1')
            || !number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(invalid());
        }
        let formatted = info["phoneNumber"].as_str().ok_or_else(invalid)?;
        if formatted != number
            && formatted != format!("86{number}")
            && formatted != format!("+86{number}")
        {
            return Err(invalid());
        }
        let timestamp = info["watermark"]["timestamp"]
            .as_i64()
            .ok_or_else(invalid)?;
        let verified_at = DateTime::from_timestamp(timestamp, 0).ok_or_else(invalid)?;
        let age = now.signed_duration_since(verified_at).num_seconds();
        if !(-60..=360).contains(&age) {
            return Err(invalid());
        }
        Ok(Self {
            number: number.into(),
            last4: number[7..].into(),
            app_id: app_id.into(),
            verified_at,
        })
    }
}

pub fn require_phone(actor: &Actor, verified: bool) -> Result<()> {
    if actor.session_kind == SessionKind::Miniapp && !verified {
        Err(Error::Phone("phone_verification_required"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn response(now: DateTime<Utc>) -> serde_json::Value {
        json!({"errcode":0,"phone_info":{"phoneNumber":"13800138000","purePhoneNumber":"13800138000","countryCode":"86","watermark":{"appid":"phone-app","timestamp":now.timestamp()}}})
    }

    #[test]
    fn validates_phone_watermark_region_time_and_business_response() {
        let now = Utc::now();
        assert_eq!(
            VerifiedPhone::from_wechat(&response(now), "phone-app", now)
                .unwrap()
                .last4,
            "8000"
        );
        for path in ["appid", "timestamp"] {
            let mut value = response(now);
            value["phone_info"]["watermark"][path] = json!("invalid");
            assert!(VerifiedPhone::from_wechat(&value, "phone-app", now).is_err());
        }
        for bad in [json!("0"), json!(40029), json!(null)] {
            let mut value = response(now);
            value["errcode"] = bad;
            assert!(VerifiedPhone::from_wechat(&value, "phone-app", now).is_err());
        }
        let mut value = response(now);
        value["phone_info"]["countryCode"] = json!("1");
        assert!(matches!(
            VerifiedPhone::from_wechat(&value, "phone-app", now),
            Err(Error::Phone("phone_region_unsupported"))
        ));
        assert!(
            VerifiedPhone::from_wechat(
                &response(now - chrono::Duration::minutes(10)),
                "phone-app",
                now
            )
            .is_err()
        );
        assert!(
            VerifiedPhone::from_wechat(
                &response(now + chrono::Duration::minutes(10)),
                "phone-app",
                now
            )
            .is_err()
        );
        let mut value = response(now);
        value["phone_info"]["purePhoneNumber"] = json!("1😀2345678");
        assert!(VerifiedPhone::from_wechat(&value, "phone-app", now).is_err());
    }

    #[test]
    fn accepts_only_the_authorization_contract_not_client_supplied_identity() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/miniapp-phone-binding-input.json"
        ))
        .unwrap();
        let mut value = fixture["data"]["attributes"].clone();
        assert!(
            serde_json::from_value::<BindPhoneInput>(value.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for key in ["mobile", "openid", "app_id", "verified"] {
            value[key] = json!("untrusted");
            assert!(serde_json::from_value::<BindPhoneInput>(value.clone()).is_err());
            value.as_object_mut().unwrap().remove(key);
        }
        value["notice_version"] = json!("old");
        assert!(
            serde_json::from_value::<BindPhoneInput>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }

    #[test]
    fn binding_wire_shapes_match_contract_fixtures() {
        for raw in [
            include_str!("../../../../api-contract/fixtures/miniapp-phone-binding.json"),
            include_str!("../../../../api-contract/fixtures/miniapp-phone-binding-unverified.json"),
            include_str!("../../../../api-contract/fixtures/miniapp-phone-binding-revoked.json"),
        ] {
            let fixture: serde_json::Value = serde_json::from_str(raw).unwrap();
            let mut value = fixture["data"]["attributes"].clone();
            value["id"] = fixture["data"]["id"].clone();
            let binding: PhoneBinding = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(binding).unwrap(), value);
        }
    }

    #[test]
    fn browser_exemption_is_not_inherited_by_a_dual_identity_miniapp_session() {
        let mut actor: Actor = serde_json::from_value(json!({"id":Uuid::nil(),"username":"root","display_name":"管理员","platform_admin":true,"bound_school_id":null,"grants":[]})).unwrap();
        actor.session_kind = SessionKind::Miniapp;
        assert!(matches!(
            require_phone(&actor, false),
            Err(Error::Phone("phone_verification_required"))
        ));
        assert!(require_phone(&actor, true).is_ok());
        assert!(actor.permission(Uuid::nil(), "content.post.edit").is_err());
        actor.session_kind = SessionKind::AdminBrowser;
        assert!(require_phone(&actor, false).is_ok());
        actor.platform_admin = false;
        assert!(actor.permission(Uuid::nil(), "content.post.edit").is_err());
    }
}
