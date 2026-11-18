use crate::{
    business::{Actor, Result},
    media::south::adapter::postgres::Repository,
};
use serde_json::Value;
use uuid::Uuid;
#[derive(Clone)]
pub struct Service(pub Repository);
impl Service {
    pub async fn put(
        &self,
        a: &Actor,
        s: Uuid,
        purpose: &str,
        b: Vec<u8>,
        key: Uuid,
    ) -> Result<Value> {
        self.0.upload(a, Some(s), purpose, b, key).await
    }
    pub async fn get(&self, a: &Actor, s: Uuid, id: Uuid) -> Result<(String, Vec<u8>)> {
        self.0.get(a, s, id).await
    }
}
