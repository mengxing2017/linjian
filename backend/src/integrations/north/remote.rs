use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result, text},
    http::jsonapi::{Input, singleton},
};
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use serde_json::json;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MiniappLogin {
    code: String,
}
async fn mini(
    State(a): State<Application>,
    h: HeaderMap,
    input_document: Input,
) -> Result<Response> {
    let v = input_document.decode::<MiniappLogin>("sessions")?;
    if h.contains_key(header::ORIGIN) {
        return Err(Error::Forbidden);
    }
    let code = text(&v.code, 1, 256)?;
    let id = a.integrations.miniapp(&code).await?;
    let (token, user) = a
        .identity
        .0
        .session(id, crate::business::SessionKind::Miniapp, None)
        .await?;
    Ok(miniapp_session_response(token, user))
}
fn miniapp_session_response(token: String, user: Actor) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        singleton("sessions", "current", json!({"token":token,"user":user})),
    )
        .into_response()
}
pub fn router() -> Router<Application> {
    Router::new().route("/api/v1/auth/miniapp", post(mini))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn miniapp_session_matches_client_fixture_without_browser_secrets() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/miniapp-session.json"
        ))
        .unwrap();
        let mut value = fixture["data"]["attributes"]["user"].clone();
        // Actor's private browser fields must never escape into the response.
        value["csrf"] = json!("must-not-be-serialized");
        let user: Actor = serde_json::from_value(value).unwrap();
        let response = miniapp_session_response("fixture-only-miniapp-token".into(), user);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        let body = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        let actual: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(actual, fixture);
    }
}
