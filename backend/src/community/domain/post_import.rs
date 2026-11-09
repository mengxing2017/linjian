use super::PostInput;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const MAX_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_ROWS: usize = 200;
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportRow {
    pub row: usize,
    pub post: PostInput,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportRowError {
    pub row: usize,
    pub message: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PostImportPreview {
    pub id: Uuid,
    pub school_id: Uuid,
    pub actor_id: Uuid,
    pub revision: i64,
    pub digest: String,
    pub rows: Vec<ImportRow>,
    pub errors: Vec<ImportRowError>,
    pub state: String,
    pub result: Vec<Uuid>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitPostImport {
    pub revision: i64,
    pub digest: String,
    pub idempotency_key: Uuid,
}
