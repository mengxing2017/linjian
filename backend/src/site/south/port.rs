use crate::business::{Actor, Result};
use crate::site::domain::{AdminSiteSettings, PublicSiteSettings, SiteSettingsInput};
use std::future::Future;

pub trait Store: Clone + Send + Sync + 'static {
    fn public(&self) -> impl Future<Output = Result<PublicSiteSettings>> + Send;
    fn logo(&self) -> impl Future<Output = Result<(String, Vec<u8>)>> + Send;
    fn admin(&self, a: &Actor) -> impl Future<Output = Result<AdminSiteSettings>> + Send;
    fn update(
        &self,
        a: &Actor,
        input: SiteSettingsInput,
    ) -> impl Future<Output = Result<AdminSiteSettings>> + Send;
}
