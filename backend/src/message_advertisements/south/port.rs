use crate::{
    business::{Actor, Page, Result},
    message_advertisements::domain::{Command, Filters},
};
use serde_json::Value;
use std::future::Future;
use uuid::Uuid;

pub trait Store: Clone + Send + Sync {
    fn current(
        &self,
        actor: &Actor,
        school: Uuid,
    ) -> impl Future<Output = Result<Option<Value>>> + Send;
    fn history(
        &self,
        actor: &Actor,
        school: Uuid,
        advertisement: Uuid,
        page: &Page,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;
    fn detail(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        advertisement: Option<Uuid>,
        revision: Uuid,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn list(
        &self,
        actor: &Actor,
        filters: &Filters,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;
    fn write(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        advertisement: Option<Uuid>,
        revision: Option<Uuid>,
        command: Command,
    ) -> impl Future<Output = Result<Value>> + Send;
}
