use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct NotificationSummary {
    pub id: Uuid,
    pub school_id: Uuid,
    pub unread_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn summary_serialization_matches_the_contract_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/notification-summary.json"
        ))
        .unwrap();
        let school: Uuid = fixture["data"]["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(
            serde_json::to_value(NotificationSummary {
                id: school,
                school_id: school,
                unread_count: 103,
            })
            .unwrap(),
            json!({"id": school, "school_id": school, "unread_count": fixture["data"]["attributes"]["unread_count"]})
        );
    }
}
