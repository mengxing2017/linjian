use crate::{business::Result, miniapp_users::domain::UserView};
use std::future::Future;
use uuid::Uuid;
/// Registration accepts only an identity already verified by the WeChat adapter.
pub trait RegistrationStore: Clone + Send + Sync {
    fn register(&self, app: &str, openid: &str) -> impl Future<Output = Result<Uuid>> + Send;
}
pub trait ProfileStore: Clone + Send + Sync {
    fn own(&self, id: Uuid) -> impl Future<Output = Result<UserView>> + Send;
}
