use crate::schools::{domain::profile::SchoolQuery, south::adapter::postgres::Repository};
use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    http::jsonapi::{ApiResponse, Input, completed, resource},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use serde_json::Value;
use uuid::Uuid;
fn page(mut rows: Vec<Value>, q: &SchoolQuery) -> Result<ApiResponse> {
    let (n, _) = q.bounds()?;
    let more = rows.len() > n as usize;
    rows.truncate(n as usize);
    Ok(resource("schools", rows)
        .meta(serde_json::json!({"page":{"number":q.page.unwrap_or(1),"size":n,"has_more":more}})))
}
async fn directory(
    State(a): State<Application>,
    actor: Actor,
    Query(q): Query<SchoolQuery>,
) -> Result<ApiResponse> {
    page(
        Repository(a.pool).search_schools(&actor, &q, false).await?,
        &q,
    )
}
async fn platform_list(
    State(a): State<Application>,
    actor: Actor,
    Query(q): Query<SchoolQuery>,
) -> Result<ApiResponse> {
    page(
        Repository(a.pool).search_schools(&actor, &q, true).await?,
        &q,
    )
}
async fn detail(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    let row = Repository(a.pool).school_detail(&actor, s).await?;
    if actor.browser && actor.can(s, "operator") {
        Ok(resource("schools", row))
    } else {
        Ok(resource("schools", row.directory()))
    }
}
async fn branding(
    State(a): State<Application>,
    _actor: Actor,
    Path((s, slot)): Path<(Uuid, String)>,
) -> Result<axum::response::Response> {
    use axum::response::IntoResponse;
    let (mime, bytes) = Repository(a.pool).branding(s, &slot).await?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, mime),
            (
                axum::http::header::CACHE_CONTROL,
                "private, no-store".into(),
            ),
            (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        bytes,
    )
        .into_response())
}
async fn list(State(a): State<Application>, actor: Actor) -> Result<ApiResponse> {
    Ok(resource("schools", a.schools.list(&actor).await?))
}
async fn create(
    State(a): State<Application>,
    actor: Actor,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<Value>("schools")?;
    Ok(resource(
        "schools",
        Repository(a.pool).save_school(&actor, None, v).await?,
    ))
}
async fn update(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<Value>("schools")?;
    Ok(resource("schools", a.schools.update(&actor, s, v).await?))
}
async fn bind(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    a.schools.bind(&actor, s).await?;
    Ok(completed())
}
async fn members(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource("accounts", a.schools.members(&actor, s).await?))
}
async fn grant(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<Value>("accounts")?;
    a.schools
        .grant(&actor, s, id, v["role"].as_str().unwrap_or(""))
        .await?;
    Ok(completed())
}
async fn resources(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource("resources", a.schools.resources(&actor, s).await?))
}
async fn published_resources(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "resources",
        a.schools.published_resources(&actor, s).await?,
    ))
}
async fn click(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    a.schools.click(&actor, s, id).await?;
    Ok(completed())
}

async fn global_resources(State(a): State<Application>, actor: Actor) -> Result<ApiResponse> {
    Ok(resource(
        "resources",
        a.schools.global_resources(&actor).await?,
    ))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/platform/resources", get(global_resources))
        .route("/api/v1/platform/schools", get(platform_list))
        .route(
            "/api/v1/school-directory/{school}/branding/{slot}",
            get(branding),
        )
        .route("/api/v1/schools", get(list).post(create))
        .route("/api/v1/school-directory", get(directory))
        .route("/api/v1/schools/{school}", get(detail).put(update))
        .route("/api/v1/schools/{school}/bind", post(bind))
        .route("/api/v1/schools/{school}/members", get(members))
        .route("/api/v1/schools/{school}/members/{id}", put(grant))
        .route("/api/v1/schools/{school}/resources", get(resources))
        .route(
            "/api/v1/schools/{school}/published-resources",
            get(published_resources),
        )
        .route("/api/v1/schools/{school}/resources/{id}/click", post(click))
}
