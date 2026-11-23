use crate::{
    business::{Actor, Page, Result},
    message_advertisements::{
        domain::{Command, Filters},
        south::port::Store,
    },
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct Service<R = crate::message_advertisements::south::adapter::postgres::Repository>(pub R);

impl<R: Store> Service<R> {
    pub async fn current(&self, actor: &Actor, school: Uuid) -> Result<Option<Value>> {
        self.0.current(actor, school).await
    }
    pub async fn history(
        &self,
        actor: &Actor,
        school: Uuid,
        advertisement: Uuid,
        page: &Page,
    ) -> Result<Vec<Value>> {
        page.bounds()?;
        self.0.history(actor, school, advertisement, page).await
    }
    pub async fn detail(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        advertisement: Option<Uuid>,
        revision: Uuid,
    ) -> Result<Value> {
        self.0.detail(actor, school, advertisement, revision).await
    }
    pub async fn list(&self, actor: &Actor, filters: &Filters) -> Result<Vec<Value>> {
        actor.platform()?;
        filters.validate()?;
        self.0.list(actor, filters).await
    }
    pub async fn write(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        advertisement: Option<Uuid>,
        revision: Option<Uuid>,
        mut command: Command,
    ) -> Result<Value> {
        command.validate()?;
        if command.platform() {
            actor.platform()?;
        }
        self.0
            .write(actor, school, advertisement, revision, command)
            .await
    }
}
