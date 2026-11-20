use crate::business::{Result, require, text};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AVATAR_VERSION: &str = "linjian-student-avatars-v1";
pub const AVATAR_COUNT: u8 = 30;
pub const MAX_ROWS: usize = 50;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorInput {
    pub key: String,
    pub existing_id: Option<Uuid>,
    pub nickname: Option<String>,
    pub avatar_id: Option<u8>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeedRow {
    pub author_key: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub display_published_at: DateTime<Utc>,
}
impl SeedRow {
    pub fn post(&self) -> super::PostInput {
        super::PostInput {
            title: self.title.clone(),
            content: self.content.clone(),
            tags: self.tags.clone(),
            images: vec![],
            candidate_tag_ids: vec![],
            revision: None,
        }
    }
    pub fn validate(&self, school: Uuid, now: DateTime<Utc>) -> Result<()> {
        self.post().validate(school)?;
        require(self.display_published_at <= now, "发布时间不能晚于当前时间")
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewInput {
    pub name: String,
    pub authors: Vec<AuthorInput>,
    pub rows: Vec<SeedRow>,
}
impl PreviewInput {
    pub fn validate(&self) -> Result<()> {
        text(&self.name, 1, 100)?;
        require(
            (1..=MAX_ROWS).contains(&self.rows.len()),
            "每批须为 1～50 篇帖子",
        )?;
        require(
            (1..=MAX_ROWS).contains(&self.authors.len()),
            "作者数量须为 1～50",
        )?;
        let mut keys = std::collections::HashSet::new();
        for author in &self.authors {
            text(&author.key, 1, 100)?;
            require(keys.insert(&author.key), "作者标识重复")?;
            require(
                author.avatar_id.is_none_or(|id| id < AVATAR_COUNT),
                "内置头像无效",
            )?;
            if let Some(name) = &author.nickname {
                text(name, 1, 100)?;
            }
        }
        for row in &self.rows {
            require(keys.contains(&row.author_key), "帖子作者标识不存在")?;
        }
        require(
            self.authors
                .iter()
                .all(|author| self.rows.iter().any(|row| row.author_key == author.key)),
            "作者必须分配给帖子",
        )
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreviewAuthor {
    pub key: String,
    pub id: Uuid,
    pub existing: bool,
    pub nickname: String,
    pub avatar_id: u8,
    pub revision: Option<i64>,
    pub avatar_url: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RowError {
    pub row: usize,
    pub message: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreviewPayload {
    pub authors: Vec<PreviewAuthor>,
    pub rows: Vec<SeedRow>,
    pub errors: Vec<RowError>,
    pub audit_mode: String,
    pub avatar_version: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatchView {
    pub id: Uuid,
    pub school_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub state: String,
    pub revision: i64,
    pub preview_digest: String,
    pub preview_payload: PreviewPayload,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub committed_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<Uuid>,
    pub revoke_reason: Option<String>,
    pub post_count: i64,
    pub interaction_count: i64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitInput {
    pub revision: i64,
    pub digest: String,
    pub idempotency_key: Uuid,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeInput {
    pub revision: i64,
    pub reason: String,
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedQuery {
    #[serde(rename = "page[number]")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]")]
    pub page_size: Option<i64>,
    pub search: Option<String>,
    pub state: Option<String>,
}
impl SeedQuery {
    pub fn page(&self) -> crate::business::Page {
        crate::business::Page {
            page: self.page,
            page_size: self.page_size,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seed_input_is_bounded_and_time_requires_offset() {
        assert!(serde_json::from_value::<SeedRow>(serde_json::json!({"author_key":"a","title":"标题","content":"正文","display_published_at":"2026-11-19T10:00:00"})).is_err());
        let mut input = PreviewInput {
            name: "批次".into(),
            authors: vec![AuthorInput {
                key: "a".into(),
                existing_id: None,
                nickname: None,
                avatar_id: None,
            }],
            rows: vec![],
        };
        assert!(input.validate().is_err());
        let row = SeedRow {
            author_key: "a".into(),
            title: "标题".into(),
            content: "正文".into(),
            tags: vec![],
            display_published_at: Utc::now(),
        };
        input.rows = vec![row.clone(); 50];
        input.authors[0].avatar_id = Some(AVATAR_COUNT - 1);
        input.validate().unwrap();
        input.authors[0].avatar_id = Some(AVATAR_COUNT);
        assert!(input.validate().is_err());
        input.authors[0].avatar_id = None;
        input.rows.push(row.clone());
        assert!(input.validate().is_err());
        assert!(
            row.validate(Uuid::nil(), Utc::now() - chrono::Duration::days(1))
                .is_err()
        );
    }
}
