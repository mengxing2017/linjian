use crate::{
    business::{Actor, Result},
    schools::{
        domain::home_menu::{CatalogQuery, OrderInput, SettingsInput, TagInput},
        south::{adapter::postgres::Repository, port::home_menu::Store},
    },
};
use serde_json::Value;
use uuid::Uuid;
#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);
impl Service<Repository> {
    pub fn from_pool(pool: sqlx::PgPool) -> Self {
        Self(Repository(pool))
    }
}
impl<R: Store> Service<R> {
    pub async fn list(&self, a: &Actor, s: Uuid, public: bool) -> Result<Value> {
        a.authorize(s, if public { "member" } else { "operator" })?;
        self.0.list(a, s, public).await
    }
    pub async fn filtered(
        &self,
        a: &Actor,
        s: Uuid,
        q: &CatalogQuery,
        export: bool,
    ) -> Result<Value> {
        a.authorize(s, "operator")?;
        q.bounds()?;
        self.0.filtered(a, s, q, export).await
    }
    pub async fn save(&self, a: &Actor, s: Uuid, id: Option<Uuid>, v: TagInput) -> Result<Value> {
        a.authorize(s, "admin")?;
        v.validate()?;
        self.0.save(a, s, id, v).await
    }
    pub async fn delete(&self, a: &Actor, s: Uuid, id: Uuid, revision: i64) -> Result<()> {
        a.authorize(s, "admin")?;
        self.0.delete(a, s, id, revision).await
    }
    pub async fn order(&self, a: &Actor, s: Uuid, v: OrderInput) -> Result<()> {
        a.authorize(s, "admin")?;
        v.validate()?;
        self.0.order(a, s, v).await
    }
    pub async fn settings(&self, a: &Actor, s: Uuid) -> Result<Value> {
        a.authorize(s, "operator")?;
        self.0.settings(a, s).await
    }
    pub async fn save_settings(&self, a: &Actor, s: Uuid, v: SettingsInput) -> Result<Value> {
        a.authorize(s, "admin")?;
        v.validate()?;
        self.0.save_settings(a, s, v).await
    }
    pub async fn import_preview(
        &self,
        a: &Actor,
        s: Uuid,
        name: &str,
        bytes: &[u8],
    ) -> Result<Value> {
        a.authorize(s, "admin")?;
        self.0.import_preview(a, s, name, bytes).await
    }
    pub async fn import_commit(&self, a: &Actor, s: Uuid, id: Uuid, digest: &str) -> Result<Value> {
        a.authorize(s, "admin")?;
        self.0.import_commit(a, s, id, digest).await
    }
}
