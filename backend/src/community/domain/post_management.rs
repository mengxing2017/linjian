use super::review::PostReview;
use crate::business::{Page, Result, require, text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Serialize)]
pub struct PostManagementView {
    #[serde(flatten)]
    pub post: PostReview,
    pub is_top: bool,
    pub cover_image: Option<String>,
}
#[derive(Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostManagementFilters {
    #[serde(rename = "page[number]")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]")]
    pub page_size: Option<i64>,
    pub search: Option<String>,
    pub audit_status: Option<String>,
    pub visibility: Option<String>,
    pub is_top: Option<bool>,
    pub tag: Option<String>,
    pub created_from: Option<chrono::DateTime<chrono::Utc>>,
    pub created_to: Option<chrono::DateTime<chrono::Utc>>,
}
impl PostManagementFilters {
    pub fn pagination(&self) -> Page {
        Page {
            page: self.page,
            page_size: self.page_size,
            ..Default::default()
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.pagination().bounds()?;
        text(self.search.as_deref().unwrap_or(""), 0, 200)?;
        text(self.tag.as_deref().unwrap_or(""), 0, 50)?;
        require(
            self.audit_status
                .as_deref()
                .is_none_or(|s| matches!(s, "" | "pending" | "approved" | "rejected")),
            "审核状态无效",
        )?;
        require(
            self.visibility
                .as_deref()
                .is_none_or(|s| matches!(s, "" | "visible" | "hidden" | "offline")),
            "帖子状态无效",
        )?;
        require(
            !matches!((self.created_from,self.created_to),(Some(a),Some(b)) if a>b),
            "时间范围无效",
        )
    }
}
pub struct PostManagementPage {
    pub items: Vec<PostManagementView>,
    pub order_revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinPostInput {
    pub revision: i64,
    pub is_top: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostVisibility {
    Visible,
    Hidden,
    Offline,
}
impl PostVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
            Self::Offline => "offline",
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetPostStatusInput {
    pub revision: i64,
    pub visibility: PostVisibility,
    #[serde(default)]
    pub reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderPostRow {
    pub id: Uuid,
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderPostsInput {
    pub order_revision: i64,
    pub is_top: bool,
    pub page: i64,
    pub page_size: i64,
    pub original: Vec<ReorderPostRow>,
    pub ordered_ids: Vec<Uuid>,
}
impl ReorderPostsInput {
    pub fn validate(&self) -> Result<()> {
        require(self.order_revision > 0, "排序版本无效")?;
        Page {
            page: Some(self.page),
            page_size: Some(self.page_size),
            ..Default::default()
        }
        .bounds()?;
        require(
            !self.original.is_empty()
                && self.original.len() <= 100
                && self.original.len() == self.ordered_ids.len(),
            "排序条目无效",
        )?;
        let original: std::collections::HashSet<_> = self.original.iter().map(|v| v.id).collect();
        let ordered: std::collections::HashSet<_> = self.ordered_ids.iter().copied().collect();
        require(
            original.len() == self.original.len()
                && ordered.len() == self.ordered_ids.len()
                && original == ordered
                && self.original.iter().all(|v| v.revision > 0),
            "排序条目重复或不匹配",
        )
    }
}
