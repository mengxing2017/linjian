//! Server-only deployment credential boundary; plaintext never becomes a read DTO.
use crate::source_access::domain::Result;
use uuid::Uuid;
pub trait CredentialStore {
    fn load_material(&self, id: Uuid) -> impl std::future::Future<Output = Result<String>> + Send;
}
