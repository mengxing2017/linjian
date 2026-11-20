use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PostInteractionState {
    pub id: Uuid,
    pub school_id: Uuid,
    pub audit_status: String,
    pub visibility: String,
    pub like_count: i64,
    pub comment_count: i64,
    pub liked: bool,
    pub collected: bool,
    pub author_followed: bool,
    pub can_interact: bool,
    pub can_share: bool,
}

impl PostInteractionState {
    pub fn set_capabilities(&mut self, active_member: bool) {
        let allowed =
            active_member && self.audit_status == "approved" && self.visibility == "visible";
        self.can_interact = allowed;
        self.can_share = allowed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serialization_matches_the_contract_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/post-interaction-state.json"
        ))
        .unwrap();
        let mut value = fixture["data"]["attributes"].clone();
        value["id"] = fixture["data"]["id"].clone();
        let state: PostInteractionState = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(state).unwrap(), value);
    }

    #[test]
    fn capabilities_require_both_membership_and_publication() {
        let mut state = PostInteractionState {
            id: Uuid::nil(),
            school_id: Uuid::nil(),
            audit_status: "approved".into(),
            visibility: "visible".into(),
            like_count: 0,
            comment_count: 0,
            liked: false,
            collected: false,
            author_followed: false,
            can_interact: false,
            can_share: false,
        };
        state.set_capabilities(true);
        assert!(state.can_interact && state.can_share);
        state.set_capabilities(false);
        assert!(!state.can_interact && !state.can_share);
        for (audit, visibility) in [
            ("pending", "visible"),
            ("rejected", "visible"),
            ("approved", "hidden"),
            ("approved", "offline"),
            ("approved", "deleted"),
        ] {
            state.audit_status = audit.into();
            state.visibility = visibility.into();
            state.set_capabilities(true);
            assert!(!state.can_interact && !state.can_share);
        }
    }
}
