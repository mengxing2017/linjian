use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result, SessionKind, text},
    http::jsonapi::{Input, resource, singleton},
    onboarding::domain::{Command, Start},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
fn response(value: impl IntoResponse) -> Response {
    let mut value = value.into_response();
    value
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    value
}
async fn no_store(value: Response) -> Response {
    response(value)
}
async fn start(
    State(app): State<Application>,
    headers: HeaderMap,
    input: Input,
) -> Result<Response> {
    if headers.contains_key(header::ORIGIN) {
        return Err(Error::Forbidden);
    }
    let value: Start = input.decode("onboarding")?;
    let (app_id, openid) = app
        .integrations
        .wechat_identity(&text(&value.code, 1, 256)?)
        .await?;
    Ok(response(singleton(
        "onboarding",
        "current",
        app.onboarding.start(&app_id, &openid).await?,
    )))
}
async fn command(
    State(app): State<Application>,
    Path(action): Path<String>,
    headers: HeaderMap,
    input: Input,
) -> Result<Response> {
    if headers.contains_key(header::ORIGIN) {
        return Err(Error::Forbidden);
    }
    if !matches!(
        action.as_str(),
        "status" | "confirm" | "complete" | "cancel"
    ) {
        return Err(Error::NotFound);
    }
    let value: Command = input.decode("onboarding")?;
    let event_id = value.request_key;
    let result = app.onboarding.command(&action, value).await?;
    if action == "complete" && result["state"] == "ready" {
        let mut user: Actor =
            serde_json::from_value(result["user"].clone()).map_err(|_| Error::Unavailable)?;
        user.session_kind = SessionKind::Miniapp;
        app.activity
            .event(&user, user.bound_school_id, "login", event_id, None);
    }
    Ok(response(singleton("onboarding", "current", result)))
}
#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    query: String,
    #[serde(default = "first")]
    page: i64,
}
fn first() -> i64 {
    1
}
async fn schools(State(app): State<Application>, Query(search): Query<Search>) -> Result<Response> {
    Ok(response(resource(
        "onboarding-schools",
        app.onboarding.schools(&search.query, search.page).await?,
    )))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/auth/miniapp/onboarding", post(start))
        .route("/api/v1/auth/miniapp/onboarding/{action}", post(command))
        .route("/api/v1/onboarding/schools", get(schools))
        .layer(axum::extract::DefaultBodyLimit::max(16_384))
        .layer(axum::middleware::map_response(no_store))
}
