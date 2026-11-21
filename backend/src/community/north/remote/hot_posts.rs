use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    community::domain::hot_posts::HotPostsQuery,
    http::jsonapi::resource,
};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderValue, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use uuid::Uuid;

async fn hot_posts(
    State(application): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Query(query): Query<HotPostsQuery>,
) -> Result<Response> {
    let started = std::time::Instant::now();
    let result = application.community.hot_posts(&actor, school, query).await;
    let error_kind = match result.as_ref().err() {
        None => "none",
        Some(crate::business::Error::Forbidden) => "forbidden",
        Some(crate::business::Error::SchoolUnavailable) => "school_unavailable",
        Some(crate::business::Error::Invalid(_)) => "validation",
        Some(_) => "unavailable",
    };
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        item_count = result.as_ref().map(|value| value.items.len()).unwrap_or(0),
        succeeded = result.is_ok(),
        error_kind,
        "school hot posts read"
    );
    let result = result?;
    Ok(resource("hot-posts", result.items)
        .meta(serde_json::to_value(result.meta).map_err(|_| crate::business::Error::Unavailable)?)
        .into_response())
}

pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/hot-posts", get(hot_posts))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("private, no-store"),
                );
                response
            },
        ))
}
