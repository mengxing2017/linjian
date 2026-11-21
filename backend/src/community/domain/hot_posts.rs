use crate::business::{Result, require};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct HotPostPolicy {
    pub window: Duration,
    pub comment_weight: i64,
    pub offset_hours: f64,
    pub exponent: f64,
    pub minimum_engagement: i64,
}

impl Default for HotPostPolicy {
    fn default() -> Self {
        Self {
            window: Duration::days(7),
            comment_weight: 2,
            offset_hours: 2.0,
            exponent: 0.6,
            minimum_engagement: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotPostsQuery {
    pub limit: u8,
}

impl Default for HotPostsQuery {
    fn default() -> Self {
        Self { limit: 10 }
    }
}

impl HotPostsQuery {
    pub fn validate(&self) -> Result<()> {
        require((1..=20).contains(&self.limit), "热榜条数必须为 1–20")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HotPostSummary {
    pub id: Uuid,
    pub school_id: Uuid,
    pub rank: i64,
    pub title: String,
    pub like_count: i64,
    pub comment_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HotPostsMeta {
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub ranked_at: DateTime<Utc>,
    pub validated_at: DateTime<Utc>,
    pub limit: u8,
}

#[derive(Debug)]
pub struct HotPostsResult {
    pub items: Vec<HotPostSummary>,
    pub meta: HotPostsMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_posts_policy_and_limits_are_explicit() {
        let policy = HotPostPolicy::default();
        assert_eq!(policy.window, Duration::days(7));
        assert_eq!(policy.comment_weight, 2);
        assert_eq!(policy.minimum_engagement, 3);
        assert_eq!(policy.offset_hours, 2.0);
        assert_eq!(policy.exponent, 0.6);
        assert_eq!(HotPostsQuery::default().limit, 10);
        for limit in [1, 10, 20] {
            assert!(HotPostsQuery { limit }.validate().is_ok());
        }
        for limit in [0, 21, 255] {
            assert!(HotPostsQuery { limit }.validate().is_err());
        }
        assert!(serde_json::from_str::<HotPostsQuery>(r#"{"limit":10,"sort":"browse"}"#).is_err());
    }
}
