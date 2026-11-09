//! Source-delivery application orchestration.
use crate::{
    business::{Actor, Page, Result},
    outbox::{
        domain::ReplayInput,
        south::{
            adapter::{delivery::HttpTransport, postgres::Repository},
            port::{OutboxStore, SourceTransport},
        },
    },
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct Service(pub PgPool);
impl Service {
    pub async fn list(&self, actor: &Actor, school: Uuid, page: Page) -> Result<Vec<Value>> {
        Repository(self.0.clone()).list(actor, school, page).await
    }
    pub async fn detail(&self, actor: &Actor, school: Uuid, id: Uuid) -> Result<Value> {
        Repository(self.0.clone()).detail(actor, school, id).await
    }
    pub async fn replay(
        &self,
        actor: &Actor,
        school: Uuid,
        id: Uuid,
        input: ReplayInput,
    ) -> Result<()> {
        Repository(self.0.clone())
            .replay(actor, school, id, input)
            .await
    }
}
#[derive(Clone)]
pub struct Worker {
    store: Repository,
}
impl Worker {
    pub fn from_env(pool: PgPool) -> Result<Option<Self>> {
        Ok(Some(Self {
            store: Repository(pool),
        }))
    }
    pub async fn run_once(&self) -> crate::source_access::domain::Result<bool> {
        self.run_with(&HttpTransport).await
    }
    pub async fn run_with(
        &self,
        transport: &impl SourceTransport,
    ) -> crate::source_access::domain::Result<bool> {
        let prepared = self.store.prepare().await?;
        let Some(delivery) = self.store.claim().await? else {
            return Ok(prepared);
        };
        let result = transport.deliver(&delivery).await;
        self.store.finish(&delivery, result).await?;
        Ok(true)
    }
}
