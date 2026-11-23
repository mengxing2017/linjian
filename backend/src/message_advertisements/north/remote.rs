use crate::{
    bootstrap::Application,
    business::{Actor, Page, Result},
    http::jsonapi::{ApiResponse, Input, collection, completed, resource},
    message_advertisements::domain::{Command, Filters},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use uuid::Uuid;

const CONFIGURATION: &str = "school-message-advertisements";
const REVISION: &str = "school-message-advertisement-revisions";

async fn current(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(
        match app.message_advertisements.current(&actor, school).await? {
            Some(value) => resource(CONFIGURATION, value),
            None => completed(),
        },
    )
}
async fn create(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                Some(school),
                None,
                None,
                Command::Create(input.decode(CONFIGURATION)?),
            )
            .await?,
    ))
}
async fn draft(
    State(app): State<Application>,
    actor: Actor,
    Path((school, advertisement)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                Some(school),
                Some(advertisement),
                None,
                Command::Draft(input.decode("school-message-advertisement-drafts")?),
            )
            .await?,
    ))
}
async fn remove_school(
    State(app): State<Application>,
    actor: Actor,
    Path((school, advertisement)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                Some(school),
                Some(advertisement),
                None,
                Command::Delete(input.decode(CONFIGURATION)?),
            )
            .await?,
    ))
}
async fn submit(
    State(app): State<Application>,
    actor: Actor,
    Path((school, advertisement, revision)): Path<(Uuid, Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                Some(school),
                Some(advertisement),
                Some(revision),
                Command::Submit(input.decode("school-message-advertisement-submissions")?),
            )
            .await?,
    ))
}
async fn withdraw(
    State(app): State<Application>,
    actor: Actor,
    Path((school, advertisement, revision)): Path<(Uuid, Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                Some(school),
                Some(advertisement),
                Some(revision),
                Command::Withdraw(input.decode("school-message-advertisement-withdrawals")?),
            )
            .await?,
    ))
}
async fn history(
    State(app): State<Application>,
    actor: Actor,
    Path((school, advertisement)): Path<(Uuid, Uuid)>,
    Query(page): Query<Page>,
) -> Result<ApiResponse> {
    Ok(collection(
        REVISION,
        app.message_advertisements
            .history(&actor, school, advertisement, &page)
            .await?,
        &page,
    ))
}
async fn detail(
    State(app): State<Application>,
    actor: Actor,
    Path((school, advertisement, revision)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Ok(resource(
        REVISION,
        app.message_advertisements
            .detail(&actor, Some(school), Some(advertisement), revision)
            .await?,
    ))
}
async fn platform_list(
    State(app): State<Application>,
    actor: Actor,
    Query(filters): Query<Filters>,
) -> Result<ApiResponse> {
    Ok(collection(
        REVISION,
        app.message_advertisements.list(&actor, &filters).await?,
        &filters.page,
    ))
}
async fn platform_detail(
    State(app): State<Application>,
    actor: Actor,
    Path(revision): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        REVISION,
        app.message_advertisements
            .detail(&actor, None, None, revision)
            .await?,
    ))
}
async fn decide(
    State(app): State<Application>,
    actor: Actor,
    Path(revision): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                None,
                None,
                Some(revision),
                Command::Decide(input.decode("school-message-advertisement-decisions")?),
            )
            .await?,
    ))
}
async fn remove_platform(
    State(app): State<Application>,
    actor: Actor,
    Path(advertisement): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        CONFIGURATION,
        app.message_advertisements
            .write(
                &actor,
                None,
                Some(advertisement),
                None,
                Command::Remove(input.decode("school-message-advertisement-removals")?),
            )
            .await?,
    ))
}

pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/message-advertisements/current", get(current))
        .route("/api/v1/schools/{school}/message-advertisements", post(create))
        .route("/api/v1/schools/{school}/message-advertisements/{advertisement}/draft", put(draft))
        .route("/api/v1/schools/{school}/message-advertisements/{advertisement}", delete(remove_school))
        .route("/api/v1/schools/{school}/message-advertisements/{advertisement}/revisions", get(history))
        .route("/api/v1/schools/{school}/message-advertisements/{advertisement}/revisions/{revision}", get(detail))
        .route("/api/v1/schools/{school}/message-advertisements/{advertisement}/revisions/{revision}/submissions", post(submit))
        .route("/api/v1/schools/{school}/message-advertisements/{advertisement}/revisions/{revision}/withdrawals", post(withdraw))
        .route("/api/v1/platform/message-advertisement-revisions", get(platform_list))
        .route("/api/v1/platform/message-advertisement-revisions/{revision}", get(platform_detail))
        .route("/api/v1/platform/message-advertisement-revisions/{revision}/decisions", post(decide))
        .route("/api/v1/platform/message-advertisements/{advertisement}/removals", post(remove_platform))
}
