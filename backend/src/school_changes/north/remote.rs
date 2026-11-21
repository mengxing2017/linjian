use crate::school_changes::domain::Filters;
use crate::{
    bootstrap::Application,
    business::{Actor, Result, Revision},
    http::jsonapi::{ApiResponse, collection, completed, resource},
};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use uuid::Uuid;

const REQUEST: &str = "miniapp-school-change-requests";
async fn policy(State(app): State<Application>, actor: Actor) -> Result<ApiResponse> {
    Ok(resource(
        "miniapp-school-change-policies",
        app.school_changes.policy(&actor)?,
    ))
}
async fn upload(State(app): State<Application>, actor: Actor, bytes: Bytes) -> Result<ApiResponse> {
    Ok(resource(
        "miniapp-school-change-documents",
        app.school_changes.upload(&actor, bytes.to_vec()).await?,
    ))
}
async fn submit(
    State(app): State<Application>,
    actor: Actor,
    input: crate::http::jsonapi::Input,
) -> Result<ApiResponse> {
    Ok(resource(
        REQUEST,
        app.school_changes
            .submit(&actor, input.decode(REQUEST)?)
            .await?,
    ))
}
async fn current(State(app): State<Application>, actor: Actor) -> Result<ApiResponse> {
    Ok(match app.school_changes.current(&actor).await? {
        Some(value) => resource(REQUEST, value),
        None => completed(),
    })
}
async fn detail(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        REQUEST,
        app.school_changes.detail(&actor, id, false).await?,
    ))
}
async fn platform_detail(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        REQUEST,
        app.school_changes.detail(&actor, id, true).await?,
    ))
}
async fn list(
    State(app): State<Application>,
    actor: Actor,
    Query(filters): Query<Filters>,
) -> Result<ApiResponse> {
    Ok(collection(
        REQUEST,
        app.school_changes.list(&actor, &filters).await?,
        &filters.page(),
    ))
}
async fn cancel(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input: crate::http::jsonapi::Input,
) -> Result<ApiResponse> {
    let input: Revision = input.decode("miniapp-school-change-cancellations")?;
    Ok(resource(
        REQUEST,
        app.school_changes
            .cancel(&actor, id, input.revision)
            .await?,
    ))
}
async fn approve(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input: crate::http::jsonapi::Input,
) -> Result<ApiResponse> {
    Ok(resource(
        REQUEST,
        app.school_changes
            .review(
                &actor,
                id,
                input.decode("miniapp-school-change-approvals")?,
                true,
            )
            .await?,
    ))
}
async fn reject(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input: crate::http::jsonapi::Input,
) -> Result<ApiResponse> {
    Ok(resource(
        REQUEST,
        app.school_changes
            .review(
                &actor,
                id,
                input.decode("miniapp-school-change-rejections")?,
                false,
            )
            .await?,
    ))
}
fn image(bytes: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response()
}
async fn document(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    Ok(image(app.school_changes.document(&actor, id, None).await?))
}
async fn platform_document(
    State(app): State<Application>,
    actor: Actor,
    Path((request, id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    Ok(image(
        app.school_changes
            .document(&actor, id, Some(request))
            .await?,
    ))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/miniapp/school-change-policy", get(policy))
        .route("/api/v1/miniapp/school-change-documents", post(upload))
        .route(
            "/api/v1/miniapp/school-change-documents/{id}",
            get(document),
        )
        .route("/api/v1/miniapp/school-change-requests", post(submit))
        .route(
            "/api/v1/miniapp/school-change-requests/current",
            get(current),
        )
        .route("/api/v1/miniapp/school-change-requests/{id}", get(detail))
        .route(
            "/api/v1/miniapp/school-change-requests/{id}/cancel",
            post(cancel),
        )
        .route("/api/v1/platform/school-change-requests", get(list))
        .route(
            "/api/v1/platform/school-change-requests/{id}",
            get(platform_detail),
        )
        .route(
            "/api/v1/platform/school-change-requests/{id}/approve",
            post(approve),
        )
        .route(
            "/api/v1/platform/school-change-requests/{id}/reject",
            post(reject),
        )
        .route(
            "/api/v1/platform/school-change-requests/{request}/documents/{id}",
            get(platform_document),
        )
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("private, no-store"),
                );
                response
            },
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn default_policy_is_always_enabled_and_matches_the_shared_fixture() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgresql://unused:unused@127.0.0.1/unused")
            .unwrap();
        let repository = crate::school_changes::south::adapter::postgres::Repository::new(
            pool,
            Default::default(),
        );
        let value = crate::school_changes::south::port::Store::policy(&repository);
        let response = resource("miniapp-school-change-policies", value).into_response();
        let bytes = axum::body::to_bytes(response.into_body(), 10_000)
            .await
            .unwrap();
        let actual: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/school-change-policy.json"
        ))
        .unwrap();
        assert_eq!(actual, expected);
    }
}
