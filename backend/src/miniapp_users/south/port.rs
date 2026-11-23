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

pub enum PhoneReservation {
    Completed(crate::miniapp_users::domain::phone::PhoneBinding),
    Reserved { openid: String },
}

pub trait PhoneProvider: Clone + Send + Sync {
    fn app_id(&self) -> &str;
    fn verify_phone(
        &self,
        code: &str,
        openid: &str,
    ) -> impl Future<Output = Result<crate::miniapp_users::domain::phone::VerifiedPhone>> + Send;
}

pub trait PhoneBindingStore: Clone + Send + Sync {
    fn phone_binding(
        &self,
        actor: &Actor,
    ) -> impl Future<Output = Result<crate::miniapp_users::domain::phone::PhoneBinding>> + Send;
    fn reserve_phone(
        &self,
        actor: &Actor,
        app_id: &str,
        input: &crate::miniapp_users::domain::phone::BindPhoneInput,
    ) -> impl Future<Output = Result<PhoneReservation>> + Send;
    fn finish_phone(
        &self,
        actor: &Actor,
        input: &crate::miniapp_users::domain::phone::BindPhoneInput,
        phone: &crate::miniapp_users::domain::phone::VerifiedPhone,
        ciphertext: &[u8],
        key_version: &str,
    ) -> impl Future<Output = Result<crate::miniapp_users::domain::phone::PhoneBinding>> + Send;
    fn fail_phone(&self, actor: &Actor, request: Uuid) -> impl Future<Output = Result<()>> + Send;
    fn revoke_phone(
        &self,
        actor: &Actor,
        revision: i64,
    ) -> impl Future<Output = Result<crate::miniapp_users::domain::phone::PhoneBinding>> + Send;
}
