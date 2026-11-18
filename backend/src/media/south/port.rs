//! Filesystem capability boundary. Keys are asset UUIDs, never user paths.
use crate::business::Result;
use uuid::Uuid;
pub trait ContentStore: Clone + Send + Sync {
    fn put_new(
        &self,
        id: Uuid,
        bytes: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn read_verified(
        &self,
        store: Uuid,
        id: Uuid,
        size: i64,
        hash: String,
    ) -> impl std::future::Future<Output = Result<Vec<u8>>> + Send;
    fn remove(&self, store: Uuid, id: Uuid)
    -> impl std::future::Future<Output = Result<()>> + Send;
}
impl ContentStore for super::adapter::filesystem::Files {
    async fn put_new(&self, id: Uuid, bytes: Vec<u8>) -> Result<()> {
        self.write(id, bytes).await
    }
    async fn read_verified(&self, s: Uuid, id: Uuid, size: i64, hash: String) -> Result<Vec<u8>> {
        self.read(s, id, size, hash).await
    }
    async fn remove(&self, s: Uuid, id: Uuid) -> Result<()> {
        self.delete(s, id).await
    }
}
