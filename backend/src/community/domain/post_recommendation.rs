use crate::business::{Result, require, text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendPost {
    pub revision: i64,
    pub content_revision: i64,
    pub publication_snapshot_id: Uuid,
    pub reason: String,
    pub idempotency_key: Uuid,
}
impl RecommendPost {
    pub fn validate(&mut self) -> Result<()> {
        require(self.revision > 0 && self.content_revision > 0, "版本无效")?;
        require(!self.idempotency_key.is_nil(), "请求身份无效")?;
        self.reason = text(&self.reason, 1, 500)?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendationView {
    pub id: Uuid,
    pub school_id: Uuid,
    pub post_id: Uuid,
    pub recommendation_id: Option<Uuid>,
    pub trigger_type: String,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub actor_name: Option<String>,
    pub reason: Option<String>,
    pub terminated_reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendationPreview {
    pub id: Uuid,
    pub school_id: Uuid,
    pub school_name: String,
    pub title: String,
    pub display_label: String,
    pub summary: Option<String>,
    pub revision: i64,
    pub content_revision: i64,
    pub publication_snapshot_id: Option<Uuid>,
    pub authorized: bool,
    pub can_recommend: bool,
    pub unavailable_reason: Option<String>,
    pub next_eligible_at: Option<chrono::DateTime<chrono::Utc>>,
    pub latest: Option<RecommendationView>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recommendation_contract_and_input_validation() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/post-recommendation.json"
        ))
        .unwrap();
        let mut attributes = doc["data"]["attributes"].clone();
        attributes["id"] = doc["data"]["id"].clone();
        let view: RecommendationView = serde_json::from_value(attributes.clone()).unwrap();
        assert_eq!(serde_json::to_value(view).unwrap(), attributes);
        let preview: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/post-recommendation-preview.json"
        ))
        .unwrap();
        let mut attrs = preview["data"]["attributes"].clone();
        attrs["id"] = preview["data"]["id"].clone();
        let decoded: RecommendationPreview = serde_json::from_value(attrs.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), attrs);
        let input: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/post-recommendation-input.json"
        ))
        .unwrap();
        let mut command: RecommendPost =
            serde_json::from_value(input["data"]["attributes"].clone()).unwrap();
        command.validate().unwrap();
        command.reason = "   ".into();
        assert!(command.validate().is_err());
        command.reason = "😀".repeat(500);
        command.validate().unwrap();
        command.reason.push('字');
        assert!(command.validate().is_err());
    }
}
