use super::local::{AccountUpdate, Login, NewAccount};
use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result, SessionKind},
    http::jsonapi::{ApiResponse, Input, completed, resource, singleton},
};
use axum::{
    Router,
    extract::{FromRequestParts, Path, State},
    http::{HeaderMap, header, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde_json::{Value, json};
use uuid::Uuid;
impl FromRequestParts<Application> for Actor {
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &Application) -> Result<Self> {
        let (token, kind) = credentials(&parts.headers)?;
        let mut actor = state.identity.authenticate(token, kind).await?;
        let segments: Vec<_> = parts
            .uri
            .path()
            .trim_start_matches('/')
            .split('/')
            .collect();
        if segments.get(0..3) == Some(&["api", "v1", "schools"][..])
            && let Some(id) = segments.get(3).and_then(|id| id.parse::<Uuid>().ok())
        {
            let entity = segments.len() == 4;
            let artwork = segments.get(4) == Some(&"resource-artwork");
            if !(actor.is_admin_browser() && actor.platform_admin && (entity || artwork)) {
                crate::schools::south::adapter::postgres::Repository(state.pool.clone())
                    .require_active(id)
                    .await?;
            }
        }
        actor.request_id = parts
            .headers
            .get("x-request-id")
            .and_then(|x| x.to_str().ok())
            .unwrap_or("")
            .chars()
            .take(64)
            .collect();
        if actor.is_admin_browser() && !matches!(parts.method.as_str(), "GET" | "HEAD" | "OPTIONS")
        {
            let origin = check_origin(&parts.headers, state)?;
            if actor.login_origin.as_deref() != Some(origin.as_str()) {
                return Err(Error::Forbidden);
            }
            if parts
                .headers
                .get("x-csrf-token")
                .and_then(|v| v.to_str().ok())
                != Some(actor.csrf.as_str())
            {
                return Err(Error::Forbidden);
            }
        }
        Ok(actor)
    }
}
fn credentials(headers: &HeaderMap) -> Result<(&str, SessionKind)> {
    let mut cookies = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|value| value.trim().strip_prefix("linjian_session="));
    let cookie = cookies.next();
    if cookies.next().is_some() || headers.get_all(header::AUTHORIZATION).iter().count() > 1 {
        return Err(Error::Unauthorized);
    }
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        if cookie.is_some() {
            return Err(Error::Unauthorized);
        }
        let token = value
            .to_str()
            .ok()
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|token| !token.is_empty() && !token.chars().any(char::is_whitespace))
            .ok_or(Error::Unauthorized)?;
        Ok((token, SessionKind::Miniapp))
    } else {
        Ok((
            cookie
                .filter(|token| !token.is_empty())
                .ok_or(Error::Unauthorized)?,
            SessionKind::AdminBrowser,
        ))
    }
}

pub fn check_origin(h: &HeaderMap, a: &Application) -> Result<String> {
    if h.get_all(header::ORIGIN).iter().count() != 1 {
        return Err(Error::Forbidden);
    }
    let origin = h
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(Error::Forbidden)?;
    if !a
        .options
        .endpoints
        .allows_admin(origin, a.options.insecure_local_auth)
    {
        return Err(Error::Forbidden);
    }
    crate::config::origins::CanonicalOrigin::parse(origin, a.options.insecure_local_auth)
        .map(|origin| origin.as_str().to_owned())
        .map_err(|_| Error::Forbidden)
}

pub fn cookie(token: &str, secure: bool, max_age: u32) -> String {
    format!(
        "linjian_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    )
}
async fn login(
    State(a): State<Application>,
    headers: HeaderMap,
    input_document: Input,
) -> Result<Response> {
    let input = input_document.decode::<Login>("sessions")?;
    let origin = check_origin(&headers, &a)?;
    let (t, user) = a.identity.login(input, &origin).await?;
    let c = cookie(&t, !a.options.insecure_local_auth, 28800);
    Ok((
        [
            (header::SET_COOKIE, c),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
        singleton("sessions", "current", json!({"user":user,"csrf":user.csrf})),
    )
        .into_response())
}
async fn session(State(_a): State<Application>, actor: Actor) -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store")],
        singleton(
            "sessions",
            "current",
            json!({"user":actor,"csrf":actor.csrf}),
        ),
    )
}
async fn logout(State(a): State<Application>, actor: Actor) -> Result<Response> {
    a.identity.logout(&actor).await?;
    if !actor.is_admin_browser() {
        return Ok(completed().into_response());
    }
    Ok((
        [(
            header::SET_COOKIE,
            cookie("", !a.options.insecure_local_auth, 0),
        )],
        completed(),
    )
        .into_response())
}
async fn list(State(a): State<Application>, actor: Actor) -> Result<ApiResponse> {
    Ok(resource("accounts", a.identity.list(&actor).await?))
}
async fn create(
    State(a): State<Application>,
    actor: Actor,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<NewAccount>("accounts")?;
    Ok(resource("accounts", a.identity.create(&actor, v).await?))
}
async fn update(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<AccountUpdate>("accounts")?;
    a.identity.update(&actor, id, v).await?;
    Ok(completed())
}
async fn reset(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<Login>("accounts")?;
    a.identity.reset(&actor, id, &v.password).await?;
    Ok(completed())
}
async fn profile(
    State(a): State<Application>,
    actor: Actor,
    input_document: Input,
) -> Result<ApiResponse> {
    let v = input_document.decode::<Value>("accounts")?;
    Ok(singleton(
        "accounts",
        actor.id,
        a.identity
            .profile(&actor, v["display_name"].as_str().unwrap_or(""))
            .await?,
    ))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/auth/sessions", post(login))
        .route("/api/v1/auth/session", get(session).delete(logout))
        .route("/api/v1/accounts", get(list).post(create))
        .route("/api/v1/accounts/{id}", put(update))
        .route("/api/v1/accounts/{id}/password", post(reset))
        .route("/api/v1/profile", put(profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_do_not_downgrade_or_mix_identities() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "linjian_session=valid".parse().unwrap());
        assert_eq!(credentials(&headers).unwrap().1, SessionKind::AdminBrowser);
        headers.insert(header::AUTHORIZATION, "Basic invalid".parse().unwrap());
        assert!(credentials(&headers).is_err());
        headers.insert(header::AUTHORIZATION, "Bearer token".parse().unwrap());
        assert!(credentials(&headers).is_err());
        headers.remove(header::COOKIE);
        assert_eq!(
            credentials(&headers).unwrap(),
            ("token", SessionKind::Miniapp)
        );
        headers.append(header::AUTHORIZATION, "Bearer other".parse().unwrap());
        assert!(credentials(&headers).is_err());
    }
}
