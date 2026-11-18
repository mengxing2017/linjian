mod time;
use crate::business::{Result, require};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use time::*;
use uuid::Uuid;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub period: Option<String>,
    pub anchor_date: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub granularity: Option<String>,
    pub comparison: Option<String>,
    pub school_ids: Option<String>,
    pub school_status: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Window {
    pub key: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub actual_end_at: DateTime<Utc>,
    pub state: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub posts: Option<i64>,
    pub comments: Option<i64>,
    pub post_authors: Option<i64>,
    pub comment_authors: Option<i64>,
    pub participants: Option<i64>,
    pub registrations: Option<i64>,
    pub cumulative_registrations: Option<i64>,
    pub new_participants: Option<i64>,
    pub active_users: Option<i64>,
    pub registered_participants: Option<i64>,
    pub daily_posts: Option<f64>,
    pub daily_comments: Option<f64>,
    pub comment_post_ratio: Option<f64>,
    pub new_participation_rate: Option<f64>,
    pub participation_rate: Option<f64>,
}
impl Metrics {
    pub fn ratios(&mut self) {
        fn ratio(a: Option<i64>, b: Option<i64>, scale: f64) -> Option<f64> {
            a.zip(b)
                .filter(|(_, b)| *b > 0)
                .map(|(a, b)| (a as f64 / b as f64 * scale * 10.).round() / 10.)
        }
        self.comment_post_ratio = ratio(self.comments, self.posts, 1.);
        self.new_participation_rate = ratio(self.new_participants, self.registrations, 100.);
        self.participation_rate = ratio(self.registered_participants, self.active_users, 100.);
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Coverage {
    pub content_from: Option<DateTime<Utc>>,
    pub registration_from: Option<DateTime<Utc>>,
    pub activity_from: Option<DateTime<Utc>>,
    pub activity_through: Option<DateTime<Utc>>,
    pub content: bool,
    pub registration: bool,
    pub registration_assignment: bool,
    pub registration_base: bool,
    pub activity: bool,
    pub reasons: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Bucket {
    #[serde(flatten)]
    pub window: Window,
    pub metrics: Metrics,
    pub coverage: Coverage,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Change {
    pub current: Option<i64>,
    pub previous: Option<i64>,
    pub delta: Option<i64>,
    pub percent: Option<f64>,
    pub state: String,
}
pub fn change(current: Option<i64>, previous: Option<i64>) -> Change {
    let delta = current.zip(previous).map(|(a, b)| a - b);
    let state = match (current, previous) {
        (Some(a), Some(0)) if a > 0 => "new",
        (Some(0), Some(0)) => "unchanged",
        (Some(_), Some(_)) => "comparable",
        _ => "unavailable",
    };
    let percent = current
        .zip(previous)
        .filter(|(_, b)| *b > 0)
        .map(|(a, b)| ((a - b) as f64 / b as f64 * 1000.).round() / 10.);
    Change {
        current,
        previous,
        delta,
        percent,
        state: state.into(),
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub kind: String,
    pub window: Option<Window>,
    pub reason: Option<String>,
    pub metrics: Option<Metrics>,
    pub changes: BTreeMap<String, Change>,
    pub buckets: Vec<Bucket>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SchoolRow {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub metrics: Metrics,
    pub coverage: Coverage,
    pub changes: BTreeMap<String, Change>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: Uuid,
    pub school_id: Option<Uuid>,
    pub school_ids: Vec<Uuid>,
    pub school_names: Vec<String>,
    pub full_site: bool,
    pub timezone: String,
    pub period: String,
    pub granularity: String,
    pub window: Window,
    pub as_of: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub definition_version: i32,
    pub summary: Metrics,
    pub coverage: Coverage,
    pub buckets: Vec<Bucket>,
    pub comparisons: Vec<Comparison>,
    pub current_state: serde_json::Value,
    pub registration_groups: serde_json::Value,
    pub schools: Vec<SchoolRow>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityInput {
    pub event_id: Uuid,
    pub kind: String,
    pub school_id: Uuid,
    pub post_id: Option<Uuid>,
}
impl ActivityInput {
    pub fn validate(&self) -> Result<()> {
        require(
            matches!(self.kind.as_str(), "feed_open" | "post_open"),
            "活跃行为类型无效",
        )?;
        require(
            (self.kind == "post_open") == self.post_id.is_some(),
            "访问目标无效",
        )
    }
}
#[derive(Clone)]
pub struct ActivityEvent {
    pub post_id: Option<Uuid>,
    pub event_id: Uuid,
    pub account_id: Uuid,
    pub school_id: Option<Uuid>,
    pub kind: String,
    pub occurred_at: DateTime<Utc>,
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    #[test]
    fn shared_fixtures_roundtrip() {
        for (source, kind) in [
            (
                include_str!("../../../../api-contract/fixtures/school-statistics.json"),
                "school-statistics",
            ),
            (
                include_str!("../../../../api-contract/fixtures/platform-statistics.json"),
                "platform-statistics",
            ),
            (
                include_str!("../../../../api-contract/fixtures/statistics-unavailable.json"),
                "school-statistics",
            ),
        ] {
            let doc: serde_json::Value = serde_json::from_str(source).unwrap();
            let mut attributes = doc["data"]["attributes"].clone();
            attributes["id"] = doc["data"]["id"].clone();
            let r: Report = serde_json::from_value(attributes.clone()).unwrap();
            assert_eq!(serde_json::to_value(r).unwrap(), attributes);
            assert_eq!(doc["data"]["type"], kind);
        }
    }
    #[test]
    fn zero_and_unavailable_changes_are_distinct() {
        assert_eq!(change(Some(2), Some(0)).state, "new");
        assert_eq!(change(Some(0), Some(0)).state, "unchanged");
        assert_eq!(change(Some(0), Some(2)).percent, Some(-100.));
        assert_eq!(change(None, Some(2)).state, "unavailable");
    }
}
