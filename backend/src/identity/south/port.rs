use crate::business::{Actor, Result, SessionKind};
use serde_json::Value;
use uuid::Uuid;
pub trait Store: Clone + Send + Sync + 'static {
    fn account(
        &self,
        username: &str,
    ) -> impl std::future::Future<Output = Result<Option<(Uuid, String)>>> + Send;
    fn throttle(&self, username: &str) -> impl std::future::Future<Output = Result<()>> + Send;
    fn failed(&self, username: &str) -> impl std::future::Future<Output = Result<()>> + Send;
    fn session(
        &self,
        id: Uuid,
        kind: SessionKind,
        origin: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(String, Actor)>> + Send;
    fn authenticate(
        &self,
        token: &str,
        kind: SessionKind,
    ) -> impl std::future::Future<Output = Result<Actor>> + Send;
    fn logout(&self, h: &str) -> impl std::future::Future<Output = Result<()>> + Send;
    fn create(
        &self,
        username: &str,
        name: &str,
        password: &str,
        admin: bool,
    ) -> impl std::future::Future<Output = Result<Uuid>> + Send;
    fn list(&self) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn update(
        &self,
        actor: Uuid,
        id: Uuid,
        name: &str,
        active: bool,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn reset_password(
        &self,
        id: Uuid,
        password: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn profile(
        &self,
        id: Uuid,
        name: &str,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
}
