use crate::{
    business::{Actor, Result},
    schools::domain::permissions::{MemberQuery, Save},
};
use serde_json::Value;
use std::future::Future;
use uuid::Uuid;

pub trait Store: Clone + Send + Sync + 'static {
    fn permission_snapshot(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn management_members(
        &self,
        actor: &Actor,
        school: Uuid,
        query: &MemberQuery,
        candidates: bool,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;
    fn save_authorization(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        input: Save,
        restricted: Option<&str>,
    ) -> impl Future<Output = Result<Value>> + Send;
}
