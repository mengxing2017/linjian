use crate::{
    business::Result,
    content_publication::{
        domain::{Failure, Policy},
        south::{
            adapter::postgres::Repository,
            port::{MiniappLinkProvider, PreparationStore},
        },
    },
};
use sqlx::PgPool;
#[derive(Clone)]
pub struct Worker {
    pub store: Repository,
}
impl Worker {
    pub fn new(pool: PgPool) -> Result<Self> {
        Policy::from_env()?;
        Ok(Self {
            store: Repository(pool),
        })
    }
    pub async fn run_with(&self, provider: &impl MiniappLinkProvider) -> Result<bool> {
        let Some(work) = self.store.claim().await? else {
            return Ok(false);
        };
        let result = if work.policy.app_id.is_empty() {
            Err(Failure::blocked("miniapp_not_configured"))
        } else if let Some(link) = self.store.cached_link(&work).await? {
            Ok(link)
        } else if self.store.reserve_link(&work).await? {
            provider.generate(&work).await
        } else {
            Err(Failure::transient("link_generation_in_progress"))
        };
        self.store.finish(&work, result).await?;
        Ok(true)
    }
}
