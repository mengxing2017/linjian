use crate::{
    business::Actor,
    miniapp_users::domain::registration::{RegistrationSettings, RegistrationSettingsInput},
};
use crate::{business::Result, miniapp_users::domain::UserView};
use std::future::Future;
use uuid::Uuid;

pub trait NicknameRandom: Send {
    fn next_number(&mut self) -> Result<u32>;
}

pub trait RegistrationSettingsStore: Clone + Send + Sync {
    fn registration_settings(&self) -> impl Future<Output = Result<RegistrationSettings>> + Send;
    fn update_registration_settings(
        &self,
        actor: &Actor,
        input: RegistrationSettingsInput,
    ) -> impl Future<Output = Result<RegistrationSettings>> + Send;
}
/// Registration accepts only an identity already verified by the WeChat adapter.
pub trait RegistrationStore: Clone + Send + Sync {
    fn register(&self, app: &str, openid: &str) -> impl Future<Output = Result<Uuid>> + Send;
}
pub trait ProfileStore: Clone + Send + Sync {
    fn own(&self, id: Uuid) -> impl Future<Output = Result<UserView>> + Send;
}
