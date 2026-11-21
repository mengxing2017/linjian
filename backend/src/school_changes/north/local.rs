use crate::business::{Actor, Result, require};
use crate::school_changes::{
    domain::{Decision, Filters, Submit},
    south::port::Store,
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct Service<R = crate::school_changes::south::adapter::postgres::Repository>(pub R);
impl<R: Store> Service<R> {
    fn own(actor: &Actor) -> Result<()> {
        require(!actor.is_admin_browser(), "请使用本人小程序身份")
    }
    pub fn policy(&self, actor: &Actor) -> Result<Value> {
        Self::own(actor)?;
        Ok(self.0.policy())
    }
    pub async fn upload(&self, actor: &Actor, bytes: Vec<u8>) -> Result<Value> {
        Self::own(actor)?;
        self.0.upload(actor, bytes).await
    }
    pub async fn submit(&self, actor: &Actor, input: Submit) -> Result<Value> {
        Self::own(actor)?;
        self.0.submit(actor, input).await
    }
    pub async fn current(&self, actor: &Actor) -> Result<Option<Value>> {
        Self::own(actor)?;
        self.0.current(actor).await
    }
    pub async fn detail(&self, actor: &Actor, id: Uuid, platform: bool) -> Result<Value> {
        if platform {
            actor.platform()?;
        } else {
            Self::own(actor)?;
        }
        self.0.detail(actor, id, platform).await
    }
    pub async fn list(&self, actor: &Actor, filters: &Filters) -> Result<Vec<Value>> {
        actor.platform()?;
        filters.validate()?;
        self.0.list(actor, filters).await
    }
    pub async fn cancel(&self, actor: &Actor, id: Uuid, revision: i64) -> Result<Value> {
        Self::own(actor)?;
        self.0.close(actor, id, revision, None, false).await
    }
    pub async fn review(
        &self,
        actor: &Actor,
        id: Uuid,
        input: Decision,
        approve: bool,
    ) -> Result<Value> {
        actor.platform()?;
        crate::business::text(&input.reason, 1, 500)?;
        self.0
            .close(actor, id, input.revision, Some(input), approve)
            .await
    }
    pub async fn document(
        &self,
        actor: &Actor,
        id: Uuid,
        request: Option<Uuid>,
    ) -> Result<Vec<u8>> {
        if request.is_some() {
            actor.platform()?;
        } else {
            Self::own(actor)?;
        }
        self.0.document(actor, id, request).await
    }
    pub async fn cleanup(&self) -> Result<()> {
        self.0.cleanup().await
    }
    pub async fn purge_account_data(&self, account: Uuid) -> Result<()> {
        self.0.purge_account_data(account).await
    }
}
