use crate::{
    business::Actor,
    miniapp_users::{
        domain::registration::{RegistrationSettings, RegistrationSettingsInput},
        south::{adapter::postgres::Repository, port::RegistrationSettingsStore},
    },
};
use crate::{
    business::{Error, Result, require},
    miniapp_users::south::port::RegistrationStore,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct RegistrationSettingsService<Store: RegistrationSettingsStore = Repository>(pub Store);

impl<Store: RegistrationSettingsStore> RegistrationSettingsService<Store> {
    fn authorize(actor: &Actor) -> Result<()> {
        if !actor.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        actor.platform()
    }

    pub async fn read(&self, actor: &Actor) -> Result<RegistrationSettings> {
        Self::authorize(actor)?;
        self.0.registration_settings().await
    }

    pub async fn update(
        &self,
        actor: &Actor,
        input: RegistrationSettingsInput,
    ) -> Result<RegistrationSettings> {
        Self::authorize(actor)?;
        input.validate()?;
        self.0.update_registration_settings(actor, input).await
    }
}
pub async fn register(store: &impl RegistrationStore, app: &str, openid: &str) -> Result<Uuid> {
    require(
        !app.trim().is_empty() && app.len() <= 128,
        "小程序 AppID 无效",
    )?;
    if openid.is_empty()
        || openid.len() > 256
        || openid.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(Error::Unavailable);
    }
    store.register(app, openid).await
}
