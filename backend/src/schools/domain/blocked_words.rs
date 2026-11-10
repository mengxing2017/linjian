use crate::business::{Result, require};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct WordInput {
    pub word: String,
    #[serde(default)]
    pub remark: String,
    pub active: bool,
    #[serde(default)]
    pub revision: i64,
}
impl WordInput {
    pub fn validate(&mut self) -> Result<()> {
        self.word = self.word.trim().to_owned();
        self.remark = self.remark.trim().to_owned();
        require(
            (1..=100).contains(&self.word.chars().count()),
            "屏蔽词须为 1 至 100 个字符",
        )?;
        require(
            self.remark.chars().count() <= 255,
            "备注不能超过 255 个字符",
        )
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderInput {
    pub ids: Vec<Uuid>,
    pub order_revision: i64,
}
#[derive(Default, Deserialize)]
pub struct WordQuery {
    pub global: Option<bool>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub ordering: Option<bool>,
}
