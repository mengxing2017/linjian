//! Durable scheduling and network boundaries. Network calls never hold a database transaction.
use crate::source_access::domain::{Fault, Result};
use uuid::Uuid;

pub struct Delivery {
    pub id: Uuid,
    pub token: Uuid,
    pub origin: String,
    pub key_id: String,
    pub body: String,
    pub body_sha256: String,
    pub receipt_id: Option<Uuid>,
    pub attempts: i32,
    pub auth_failures: i32,
}
pub struct ReceiptResult {
    pub id: Uuid,
    pub status: String,
    pub summary: serde_json::Value,
}
pub type DeliveryResult = std::result::Result<ReceiptResult, (Fault, i64)>;
pub trait SourceTransport: Send + Sync {
    fn deliver(
        &self,
        delivery: &Delivery,
    ) -> impl std::future::Future<Output = DeliveryResult> + Send;
}
pub trait OutboxStore {
    fn prepare(&self) -> impl std::future::Future<Output = Result<bool>> + Send;
    fn claim(&self) -> impl std::future::Future<Output = Result<Option<Delivery>>> + Send;
    fn finish(
        &self,
        delivery: &Delivery,
        result: DeliveryResult,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}
