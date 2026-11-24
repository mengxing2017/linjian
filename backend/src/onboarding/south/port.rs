use crate::{business::Result, onboarding::domain::Command};
use serde_json::Value;
pub trait Store: Clone + Send + Sync {
    fn start(&self, app: &str, openid: &str) -> impl Future<Output = Result<Value>> + Send;
    fn command(&self, action: &str, input: Command) -> impl Future<Output = Result<Value>> + Send;
    fn schools(&self, query: &str, page: i64) -> impl Future<Output = Result<Vec<Value>>> + Send;
}
