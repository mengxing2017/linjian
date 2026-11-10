use crate::business::{Actor, Result};
use serde_json::Value;
use uuid::Uuid;
pub trait Store: Clone + Send + Sync + 'static {
    fn list(&self, a: &Actor) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn bind(&self, a: &Actor, school: Uuid)
    -> impl std::future::Future<Output = Result<()>> + Send;
    fn members(&self, school: Uuid)
    -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn grant(
        &self,
        a: &Actor,
        school: Uuid,
        id: Uuid,
        role: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn resources(
        &self,
        school: Uuid,
        operator: bool,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn click(&self, school: Uuid, id: Uuid)
    -> impl std::future::Future<Output = Result<()>> + Send;
}

pub mod resources;

pub mod home_menu;
