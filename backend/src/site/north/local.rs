use super::super::south::adapter::postgres::Repository;
use crate::business::{Actor, Result};
use crate::site::domain::{AdminSiteSettings, PublicSiteSettings, SiteSettingsInput};
use crate::site::south::port::Store;

#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);

impl<R: Store> Service<R> {
    pub async fn public(&self) -> Result<PublicSiteSettings> {
        self.0.public().await
    }

    pub async fn admin(&self, a: &Actor) -> Result<AdminSiteSettings> {
        a.platform()?;
        self.0.admin(a).await
    }

    pub async fn update(&self, a: &Actor, input: SiteSettingsInput) -> Result<AdminSiteSettings> {
        a.platform()?;
        crate::business::require(input.revision >= 0, "站点配置版本无效")?;
        self.0.update(a, input).await
    }
}
