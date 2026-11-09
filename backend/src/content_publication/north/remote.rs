use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    content_publication::{domain::Retry, south::adapter::postgres::Repository},
    http::jsonapi::{ApiResponse, Input, completed},
};
use axum::{
    Router,
    extract::{Path, State},
    routing::post,
};
use uuid::Uuid;
async fn retry(
    State(a): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Repository(a.pool)
        .retry(
            &actor,
            school,
            id,
            input.decode::<Retry>("source-preparation-retries")?,
        )
        .await?;
    Ok(completed())
}
pub fn router() -> Router<Application> {
    Router::new().route(
        "/api/v1/schools/{school}/source-outbox/{id}/preparation-retries",
        post(retry),
    )
}
