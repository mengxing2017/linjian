use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    http::jsonapi::{Input, resource},
    legal_documents::domain::{Clear, Save},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
fn response(value: impl IntoResponse) -> Response {
    let mut response = value.into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}
async fn no_store(value: Response) -> Response {
    response(value)
}
async fn list(State(app): State<Application>) -> Result<Response> {
    Ok(response(resource(
        "legal-documents",
        app.legal_documents.read(None, None).await?,
    )))
}
async fn detail(State(app): State<Application>, Path(kind): Path<String>) -> Result<Response> {
    Ok(response(resource(
        "legal-documents",
        app.legal_documents.read(Some(&kind), None).await?,
    )))
}
async fn admin(
    State(app): State<Application>,
    actor: Actor,
    Path(kind): Path<String>,
) -> Result<Response> {
    Ok(response(resource(
        "legal-documents",
        app.legal_documents.read(Some(&kind), Some(&actor)).await?,
    )))
}
async fn save(
    State(app): State<Application>,
    actor: Actor,
    Path(kind): Path<String>,
    input: Input,
) -> Result<Response> {
    let value: Save = input.decode("legal-documents")?;
    Ok(response(resource(
        "legal-documents",
        app.legal_documents
            .save(&actor, &kind, value.revision, Some(value.content))
            .await?,
    )))
}
async fn clear(
    State(app): State<Application>,
    actor: Actor,
    Path(kind): Path<String>,
    input: Input,
) -> Result<Response> {
    let value: Clear = input.decode("legal-document-clears")?;
    Ok(response(resource(
        "legal-documents",
        app.legal_documents
            .save(&actor, &kind, value.revision, None)
            .await?,
    )))
}
#[derive(Deserialize)]
struct Page {
    #[serde(default = "first")]
    page: i64,
}
fn first() -> i64 {
    1
}
async fn history(
    State(app): State<Application>,
    actor: Actor,
    Path(kind): Path<String>,
    Query(page): Query<Page>,
) -> Result<Response> {
    Ok(response(resource(
        "legal-document-revisions",
        app.legal_documents
            .history(&actor, &kind, page.page)
            .await?,
    )))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/legal-documents", get(list))
        .route("/api/v1/legal-documents/{kind}", get(detail))
        .route(
            "/api/v1/platform/legal-documents/{kind}",
            get(admin).put(save),
        )
        .route("/api/v1/platform/legal-documents/{kind}/clear", post(clear))
        .route(
            "/api/v1/platform/legal-documents/{kind}/revisions",
            get(history),
        )
        .layer(axum::extract::DefaultBodyLimit::max(1_048_576))
        .layer(axum::middleware::map_response(no_store))
}
