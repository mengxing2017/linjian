use crate::{
    business::{Error, Result, require},
    miniapp_users::south::port::RegistrationStore,
};
use uuid::Uuid;
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
