use crate::{
    bootstrap::Application,
    business::{Actor, Page, Result},
    http::jsonapi::{ApiResponse, Input, collection, resource},
    schools::domain::permissions::{Action, MemberQuery, Recommendation, Save},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use uuid::Uuid;

async fn catalog(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "school-permission-catalogs",
        app.schools.permission_catalog(&actor, school).await?,
    ))
}
async fn mine(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "school-member-permissions",
        app.schools
            .member_permissions(&actor, school, actor.id)
            .await?,
    ))
}
async fn detail(
    State(app): State<Application>,
    actor: Actor,
    Path((school, target)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Ok(resource(
        "school-member-permissions",
        app.schools
            .member_permissions(&actor, school, target)
            .await?,
    ))
}
async fn list(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Query(query): Query<MemberQuery>,
) -> Result<ApiResponse> {
    members(app, actor, school, query, false).await
}
async fn candidates(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Query(query): Query<MemberQuery>,
) -> Result<ApiResponse> {
    members(app, actor, school, query, true).await
}
async fn members(
    app: Application,
    actor: Actor,
    school: Uuid,
    query: MemberQuery,
    candidates: bool,
) -> Result<ApiResponse> {
    let rows = app
        .schools
        .management_members(&actor, school, &query, candidates)
        .await?;
    Ok(collection(
        if candidates {
            "school-member-candidates"
        } else {
            "school-management-members"
        },
        rows,
        &Page {
            page: query.page,
            page_size: query.page_size,
            ..Default::default()
        },
    ))
}
async fn save(
    State(app): State<Application>,
    actor: Actor,
    Path((school, target)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let command: Save = input.decode("school-member-authorizations")?;
    Ok(resource(
        "school-member-permissions",
        app.schools
            .save_authorization(&actor, school, target, command)
            .await?,
    ))
}
async fn action(
    State(app): State<Application>,
    actor: Actor,
    Path((school, target)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let command: Action = input.decode("school-member-authorization-actions")?;
    Ok(resource(
        "school-member-permissions",
        app.schools
            .authorization_action(&actor, school, target, command)
            .await?,
    ))
}
async fn recommendation(
    State(app): State<Application>,
    actor: Actor,
    Path((school, target)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let command: Recommendation = input.decode("post-recommendation-capabilities")?;
    Ok(resource(
        "school-member-permissions",
        app.schools
            .set_recommendation_permission(&actor, school, target, command)
            .await?,
    ))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/permission-catalog", get(catalog))
        .route("/api/v1/schools/{school}/permissions/me", get(mine))
        .route("/api/v1/schools/{school}/members", get(list))
        .route(
            "/api/v1/schools/{school}/member-candidates",
            get(candidates),
        )
        .route(
            "/api/v1/schools/{school}/members/{target}/permissions",
            get(detail),
        )
        .route(
            "/api/v1/schools/{school}/members/{target}/authorization",
            put(save),
        )
        .route(
            "/api/v1/schools/{school}/members/{target}/authorization-actions",
            post(action),
        )
        .route(
            "/api/v1/schools/{school}/members/{target}/recommendation-capability",
            put(recommendation),
        )
}
