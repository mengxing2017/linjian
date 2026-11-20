use crate::{
    business::{Actor, Result},
    community::domain::post_seed::*,
};
use serde_json::Value;
use uuid::Uuid;

pub trait Store: Clone + Send + Sync + 'static {
    fn preview(
        &self,
        actor: &Actor,
        school: Uuid,
        input: PreviewInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn commit(
        &self,
        actor: &Actor,
        school: Uuid,
        batch: Uuid,
        input: CommitInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn revoke(
        &self,
        actor: &Actor,
        school: Uuid,
        batch: Uuid,
        input: RevokeInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn read(
        &self,
        actor: &Actor,
        school: Uuid,
        kind: &str,
        batch: Option<Uuid>,
        query: SeedQuery,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
}
