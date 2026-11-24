use crate::{
    business::{Result, require},
    onboarding::{
        domain::Command,
        south::{adapter::postgres::Repository, port::Store},
    },
};
use serde_json::Value;
#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);
impl<R: Store> Service<R> {
    pub async fn start(&self, app: &str, openid: &str) -> Result<Value> {
        self.0.start(app, openid).await
    }
    pub async fn command(&self, action: &str, input: Command) -> Result<Value> {
        require(
            matches!(action, "status" | "confirm" | "complete" | "cancel"),
            "未知入驻操作",
        )?;
        input.validate()?;
        self.0.command(action, input).await
    }
    pub async fn schools(&self, query: &str, page: i64) -> Result<Vec<Value>> {
        require(
            query.chars().count() <= 100 && (1..=10000).contains(&page),
            "学校搜索条件不合法",
        )?;
        self.0.schools(query, page).await
    }
}
