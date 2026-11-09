use super::local::Service;
use crate::{
    bootstrap::Application,
    business::{Actor, Page, Result},
    http::jsonapi::{ApiResponse, Input, collection, completed, resource},
    outbox::domain::ReplayInput,
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
    let rows = Service(a.pool).list(&actor, s, page.clone()).await?;
    Ok(collection("source-outbox", rows, &page))
}
async fn detail(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Ok(resource(
        "source-outbox",
        Service(a.pool).detail(&actor, s, id).await?,
    ))
}
async fn replay(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Service(a.pool)
        .replay(
            &actor,
            s,
            id,
            input.decode::<ReplayInput>("source-outbox-replays")?,
        )
        .await?;
    Ok(completed())
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/source-outbox", get(list))
        .route("/api/v1/schools/{school}/source-outbox/{id}", get(detail))
        .route(
            "/api/v1/schools/{school}/source-outbox/{id}/replays",
            post(replay),
        )
}
