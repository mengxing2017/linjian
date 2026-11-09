use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    http::jsonapi::{ApiResponse, Input, completed, resource, singleton},
    miniapp_users::{
        domain::*,
        south::{adapter::postgres::Repository, port::ProfileStore},
    },
};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query as Params, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use uuid::Uuid;
fn collection(mut rows: Vec<UserView>, q: &Query) -> Result<ApiResponse> {
    let (size, _) = q.bounds()?;
    let more = rows.len() > size as usize;
    rows.truncate(size as usize);
    Ok(resource("miniapp-users", rows).meta(
        serde_json::json!({"page":{"number":q.page.unwrap_or(1),"size":size,"has_more":more}}),
    ))
}
async fn list(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Params(q): Params<Query>,
) -> Result<ApiResponse> {
    crate::business::require(q.mobile.is_none(), "手机号筛选请使用搜索接口")?;
    collection(Repository(a.pool).list(&actor, Some(s), &q).await?, &q)
}
async fn platform_list(
    State(a): State<Application>,
    actor: Actor,
    Params(q): Params<Query>,
) -> Result<ApiResponse> {
    crate::business::require(q.mobile.is_none(), "手机号筛选请使用搜索接口")?;
    collection(Repository(a.pool).list(&actor, None, &q).await?, &q)
}
async fn search(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    v: Input,
) -> Result<ApiResponse> {
    let q: Query = v.decode("miniapp-user-searches")?;
    collection(Repository(a.pool).list(&actor, Some(s), &q).await?, &q)
}
async fn platform_search(
    State(a): State<Application>,
    actor: Actor,
    v: Input,
) -> Result<ApiResponse> {
    let q: Query = v.decode("miniapp-user-searches")?;
    collection(Repository(a.pool).list(&actor, None, &q).await?, &q)
}
async fn detail(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Ok(resource(
        "miniapp-users",
        Repository(a.pool).detail(&actor, Some(s), id).await?,
    ))
}
async fn platform_detail(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "miniapp-users",
        Repository(a.pool).detail(&actor, None, id).await?,
    ))
}
async fn status(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    v: Input,
) -> Result<ApiResponse> {
    Repository(a.pool)
        .status(&actor, Some(s), id, v.decode("miniapp-user-statuses")?)
        .await?;
    Ok(completed())
}
async fn platform_status(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    v: Input,
) -> Result<ApiResponse> {
    Repository(a.pool)
        .status(&actor, None, id, v.decode("miniapp-user-statuses")?)
        .await?;
    Ok(completed())
}
async fn profile(State(a): State<Application>, actor: Actor) -> Result<ApiResponse> {
    crate::business::require(!actor.browser, "请使用小程序身份")?;
    Ok(resource(
        "miniapp-profiles",
        Repository(a.pool).own(actor.id).await?,
    ))
}
async fn edit_profile(State(a): State<Application>, actor: Actor, v: Input) -> Result<ApiResponse> {
    Ok(resource(
        "miniapp-profiles",
        Repository(a.pool)
            .profile(&actor, v.decode("miniapp-profiles")?)
            .await?,
    ))
}
async fn mobile(State(a): State<Application>, actor: Actor, v: Input) -> Result<ApiResponse> {
    Ok(resource(
        "miniapp-profiles",
        Repository(a.pool)
            .mobile(&actor, actor.id, v.decode("miniapp-user-mobiles")?, false)
            .await?,
    ))
}
async fn platform_mobile(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    v: Input,
) -> Result<ApiResponse> {
    Repository(a.pool)
        .mobile(&actor, id, v.decode("miniapp-user-mobiles")?, true)
        .await?;
    Ok(completed())
}
async fn school(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    v: Input,
) -> Result<ApiResponse> {
    Repository(a.pool)
        .school(&actor, id, v.decode("miniapp-user-schools")?)
        .await?;
    Ok(completed())
}
async fn upload(State(a): State<Application>, actor: Actor, b: Bytes) -> Result<ApiResponse> {
    let id = Repository(a.pool).upload(&actor, b.to_vec()).await?;
    Ok(singleton(
        "miniapp-avatar-assets",
        id,
        serde_json::json!({"url":format!("/api/v1/miniapp/avatar-assets/{id}")}),
    ))
}
async fn avatar(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let (mime, bytes) = Repository(a.pool).avatar(&actor, id).await?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        bytes,
    )
        .into_response())
}
pub fn router() -> Router<Application> {
    Router::new()
        .route(
            "/api/v1/schools/{school}/miniapp-users/searches",
            post(search),
        )
        .route(
            "/api/v1/platform/miniapp-users/searches",
            post(platform_search),
        )
        .route("/api/v1/schools/{school}/miniapp-users", get(list))
        .route("/api/v1/schools/{school}/miniapp-users/{id}", get(detail))
        .route(
            "/api/v1/schools/{school}/miniapp-users/{id}/status",
            put(status),
        )
        .route("/api/v1/platform/miniapp-users", get(platform_list))
        .route("/api/v1/platform/miniapp-users/{id}", get(platform_detail))
        .route(
            "/api/v1/platform/miniapp-users/{id}/status",
            put(platform_status),
        )
        .route(
            "/api/v1/platform/miniapp-users/{id}/mobile",
            put(platform_mobile),
        )
        .route("/api/v1/platform/miniapp-users/{id}/school", put(school))
        .route("/api/v1/miniapp/profile", get(profile).put(edit_profile))
        .route("/api/v1/miniapp/profile/mobile", put(mobile))
        .route("/api/v1/miniapp/avatar-assets", post(upload))
        .route("/api/v1/miniapp/avatar-assets/{id}", get(avatar))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut().insert(
                    header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("private, no-store"),
                );
                r
            },
        ))
}
