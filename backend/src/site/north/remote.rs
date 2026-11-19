use crate::bootstrap::Application;
use crate::business::{Actor, Result};
use crate::http::jsonapi::{ApiResponse, Input, singleton};
use crate::site::domain::SiteSettingsInput;
use axum::{
    Router,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::Value;

const PUBLIC_CACHE: &str = "public, max-age=60, stale-while-revalidate=240";
const PRIVATE_CACHE: &str = "private, no-store";

fn cached(response: ApiResponse, value: &'static str) -> Response {
    let mut response = response.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static(value),
    );
    response
}

async fn public_read(State(app): State<Application>) -> Result<Response> {
    Ok(cached(
        singleton("site-settings", "current", app.site.public().await?),
        PUBLIC_CACHE,
    ))
}

async fn admin_read(State(app): State<Application>, actor: Actor) -> Result<Response> {
    Ok(cached(
        singleton("site-settings", "current", app.site.admin(&actor).await?),
        PRIVATE_CACHE,
    ))
}

async fn update(State(app): State<Application>, actor: Actor, input: Input) -> Result<Response> {
    let value: Value = input.decode("site-settings")?;
    let input = SiteSettingsInput::parse(value)?;
    Ok(cached(
        singleton(
            "site-settings",
            "current",
            app.site.update(&actor, input).await?,
        ),
        PRIVATE_CACHE,
    ))
}

pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/site-settings", get(public_read))
        .route(
            "/api/v1/platform/site-settings",
            get(admin_read).put(update),
        )
}
