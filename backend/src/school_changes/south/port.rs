use crate::business::{Actor, Result};
use crate::school_changes::domain::{Decision, Filters, Submit};
use serde_json::Value;
use std::future::Future;
use uuid::Uuid;

pub trait Store: Clone + Send + Sync {
    fn policy(&self) -> Value;
    fn upload(&self, actor: &Actor, bytes: Vec<u8>) -> impl Future<Output = Result<Value>> + Send;
    fn submit(&self, actor: &Actor, input: Submit) -> impl Future<Output = Result<Value>> + Send;
    fn current(&self, actor: &Actor) -> impl Future<Output = Result<Option<Value>>> + Send;
    fn detail(
        &self,
        actor: &Actor,
        id: Uuid,
        platform: bool,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn list(
        &self,
        actor: &Actor,
        filters: &Filters,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;
    fn close(
        &self,
        actor: &Actor,
        id: Uuid,
        revision: i64,
        decision: Option<Decision>,
        approve: bool,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn document(
        &self,
        actor: &Actor,
        id: Uuid,
        request: Option<Uuid>,
    ) -> impl Future<Output = Result<Vec<u8>>> + Send;
    fn cleanup(&self) -> impl Future<Output = Result<()>> + Send;
    fn purge_account_data(&self, account: Uuid) -> impl Future<Output = Result<()>> + Send;
}
