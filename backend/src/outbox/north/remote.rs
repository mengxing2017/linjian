use super::super::south::adapter::postgres::Repository;
use crate::{
    bootstrap::Application,
    business::{Actor, Page, Result},
    http::jsonapi::{ApiResponse, collection, completed},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use uuid::Uuid;
async fn list(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Query(page): Query<Page>,
) -> Result<ApiResponse> {
    let rows = Repository(a.pool).list(&actor, s, page.clone()).await?;
    Ok(collection("source-outbox", rows, &page))
}
async fn replay(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Repository(a.pool).replay(&actor, s, id).await?;
    Ok(completed())
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/source-outbox", get(list))
        .route(
            "/api/v1/schools/{school}/source-outbox/{id}/replays",
            post(replay),
        )
}
