use crate::business::{Result, require, text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseState {
    Open,
    Resolved,
    Dismissed,
}
impl CaseState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Dismissed => "dismissed",
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseKind {
    Report,
    Feedback,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CaseRecord {
    pub id: Uuid,
    pub school_id: Uuid,
    pub account_id: Uuid,
    pub kind: CaseKind,
    pub post_id: Option<Uuid>,
    pub content: String,
    pub contact: String,
    pub images: Vec<String>,
    pub state: CaseState,
    pub resolution: String,
    pub revision: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CaseManagementView {
    #[serde(flatten)]
    pub record: CaseRecord,
    pub title: String,
    pub reporter_name: String,
    pub school_name: String,
    pub handled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub handled_by: Option<Uuid>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveCaseInput {
    pub revision: i64,
    pub state: CaseState,
    pub resolution: String,
}
impl ResolveCaseInput {
    pub fn validate(&mut self) -> Result<()> {
        require(self.revision > 0, "版本号无效")?;
        require(self.state != CaseState::Open, "处理状态无效")?;
        self.resolution = text(&self.resolution, 1, 2000)?;
        Ok(())
    }
}
