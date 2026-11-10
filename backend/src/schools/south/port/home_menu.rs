use crate::{
    business::{Actor, Result},
    schools::domain::home_menu::{CatalogQuery, OrderInput, SettingsInput, TagInput},
};
use serde_json::Value;
use uuid::Uuid;
pub trait Store: Clone + Send + Sync + 'static {
    fn list(
        &self,
        a: &Actor,
        s: Uuid,
        public: bool,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn filtered(
        &self,
        a: &Actor,
        s: Uuid,
        q: &CatalogQuery,
        export: bool,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn save(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        v: TagInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn delete(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn order(
        &self,
        a: &Actor,
        s: Uuid,
        v: OrderInput,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn settings(
        &self,
        a: &Actor,
        s: Uuid,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn save_settings(
        &self,
        a: &Actor,
        s: Uuid,
        v: SettingsInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn import_preview(
        &self,
        a: &Actor,
        s: Uuid,
        name: &str,
        bytes: &[u8],
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn import_commit(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        digest: &str,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
}
