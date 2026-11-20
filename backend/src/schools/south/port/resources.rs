use crate::{
    business::{Actor, Result},
    schools::domain::ResourceInput,
};
use serde_json::Value;
use std::future::Future;
use uuid::Uuid;
#[derive(Clone)]
pub enum Command {
    Preview(ResourceInput),
    Confirm { preview_id: Uuid, digest: String },
}
pub trait Store: Clone + Send + Sync + 'static {
    fn command(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        v: Command,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn upload(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        bytes: Vec<u8>,
        context: &str,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn media_list(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        page: crate::business::Page,
        context: &str,
    ) -> impl Future<Output = Result<Vec<Value>>> + Send;
    fn media(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        published: bool,
        context: &str,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> impl Future<Output = Result<(String, Vec<u8>)>> + Send;
}
