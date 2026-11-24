use crate::business::{Actor, Result, require};
use crate::legal_documents::{
    domain,
    south::{adapter::postgres::Repository, port::Store},
};
use serde_json::Value;
#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);
impl<R: Store> Service<R> {
    pub async fn read(&self, kind: Option<&str>, actor: Option<&Actor>) -> Result<Value> {
        if let Some(actor) = actor {
            actor.platform()?;
        }
        if let Some(kind) = kind {
            domain::kind(kind)?;
        }
        self.0.read(kind, actor.is_some()).await
    }
    pub async fn history(&self, actor: &Actor, kind: &str, page: i64) -> Result<Value> {
        actor.platform()?;
        domain::kind(kind)?;
        require((1..=100_000).contains(&page), "分页不合法")?;
        self.0.history(kind, page).await
    }
    pub async fn save(
        &self,
        actor: &Actor,
        kind: &str,
        revision: i64,
        content: Option<String>,
    ) -> Result<Value> {
        actor.platform()?;
        domain::kind(kind)?;
        require(revision >= 0, "版本不合法")?;
        let content = content.map(|value| domain::content(&value)).transpose()?;
        self.0.save(actor, kind, revision, content).await
    }
}
