use crate::business::{Actor, Result};
use serde_json::Value;
pub trait Store: Clone + Send + Sync {
    fn read(&self, kind: Option<&str>, admin: bool) -> impl Future<Output = Result<Value>> + Send;
    fn history(&self, kind: &str, page: i64) -> impl Future<Output = Result<Value>> + Send;
    fn save(
        &self,
        actor: &Actor,
        kind: &str,
        revision: i64,
        content: Option<String>,
    ) -> impl Future<Output = Result<Value>> + Send;
}
