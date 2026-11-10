use crate::{
    business::{Actor, Result},
    schools::{
        domain::blocked_words::{OrderInput, WordInput, WordQuery},
        south::{adapter::postgres::Repository, port::blocked_words::Store},
    },
};
use serde_json::Value;
use uuid::Uuid;
pub struct Service<R: Store = Repository>(pub R);
impl Service<Repository> {
    pub fn from_pool(pool: sqlx::PgPool) -> Self {
        Self(Repository(pool))
    }
}
impl<R: Store> Service<R> {
    pub async fn blocked_list(&self, a: &Actor, s: Option<Uuid>, q: &WordQuery) -> Result<Value> {
        self.0.list(a, s, q).await
    }
    pub async fn blocked_save(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Option<Uuid>,
        v: WordInput,
    ) -> Result<Value> {
        self.0.save(a, s, id, v).await
    }
    pub async fn blocked_delete(&self, a: &Actor, s: Option<Uuid>, id: Uuid, r: i64) -> Result<()> {
        self.0.delete(a, s, id, r).await
    }
    pub async fn blocked_order(&self, a: &Actor, s: Option<Uuid>, v: OrderInput) -> Result<()> {
        self.0.order(a, s, v).await
    }
    pub async fn blocked_import_preview(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        name: &str,
        bytes: &[u8],
    ) -> Result<Value> {
        self.0.import_preview(a, s, name, bytes).await
    }
    pub async fn blocked_import_commit(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        digest: &str,
    ) -> Result<Value> {
        self.0.import_commit(a, s, id, digest).await
    }
}
