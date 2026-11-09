use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    content_publication::{domain::Retry, south::adapter::postgres::Repository},
    http::jsonapi::{ApiResponse, Input, completed, singleton},
};
use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
};
use uuid::Uuid;
async fn preview(
    State(a): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Ok(singleton(
        "source-previews",
        id,
        Repository(a.pool).preview(&actor, school, id).await?,
    ))
}
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
    Router::new()
        .route(
            "/api/v1/schools/{school}/posts/{id}/source-preview",
            get(preview),
        )
        .route(
            "/api/v1/schools/{school}/source-outbox/{id}/preparation-retries",
            post(retry),
        )
}
