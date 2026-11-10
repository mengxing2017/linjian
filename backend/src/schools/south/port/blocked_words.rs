use crate::{
    business::{Actor, Result},
    schools::domain::blocked_words::{OrderInput, WordInput, WordQuery},
};
use serde_json::Value;
use uuid::Uuid;
pub trait Store: Clone + Send + Sync + 'static {
    fn list(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        q: &WordQuery,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn save(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Option<Uuid>,
        v: WordInput,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn delete(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        revision: i64,
    ) -> impl Future<Output = Result<()>> + Send;
    fn order(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        v: OrderInput,
    ) -> impl Future<Output = Result<()>> + Send;
    fn import_preview(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        name: &str,
        bytes: &[u8],
    ) -> impl Future<Output = Result<Value>> + Send;
    fn import_commit(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        digest: &str,
    ) -> impl Future<Output = Result<Value>> + Send;
}
