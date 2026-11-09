use super::super::south::adapter::postgres::Repository;
use crate::business::{Actor, Result, require};
use crate::schools::south::port::Store;
use serde_json::Value;
use uuid::Uuid;
#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);
impl<R: Store> Service<R> {
    pub async fn grant(&self, a: &Actor, school: Uuid, id: Uuid, role: &str) -> Result<()> {
        a.platform()?;
        require(
            ["member", "operator", "admin", "revoked"].contains(&role),
            "角色无效",
        )?;
        self.0.grant(a, school, id, role).await
    }
    pub async fn resources(&self, a: &Actor, school: Uuid) -> Result<Vec<Value>> {
        a.authorize(school, "member")?;
        self.0.resources(school, a.can(school, "operator")).await
    }
    pub async fn published_resources(&self, a: &Actor, school: Uuid) -> Result<Vec<Value>> {
        a.authorize(school, "member")?;
        self.0.resources(school, false).await
    }
}

impl<R: Store> Service<R> {
    pub async fn list(&self, a: &Actor) -> Result<Vec<Value>> {
        self.0.list(a).await
    }
    pub async fn bind(&self, a: &Actor, s: Uuid) -> Result<()> {
        self.0.bind(a, s).await
    }
    pub async fn members(&self, a: &Actor, s: Uuid) -> Result<Vec<Value>> {
        a.authorize(s, "admin")?;
        self.0.members(s).await
    }
    pub async fn click(&self, a: &Actor, s: Uuid, id: Uuid) -> Result<()> {
        a.authorize(s, "member")?;
        self.0.click(s, id).await
    }
}

impl<R: Store> Service<R> {
    pub async fn global_resources(&self, a: &Actor) -> Result<Vec<Value>> {
        a.platform()?;
        self.0.resources(Uuid::nil(), true).await
    }
}

impl Service<Repository> {
    pub async fn create(&self, a: &Actor, name: &str) -> Result<Value> {
        serde_json::to_value(
            self.0
                .save_school(a, None, serde_json::json!({"name":name}))
                .await?,
        )
        .map_err(|_| crate::business::Error::Unavailable)
    }
    pub async fn update(&self, a: &Actor, id: Uuid, v: Value) -> Result<Value> {
        serde_json::to_value(self.0.save_school(a, Some(id), v).await?)
            .map_err(|_| crate::business::Error::Unavailable)
    }
}
