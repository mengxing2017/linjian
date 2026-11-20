use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result},
    community::domain::post_seed::*,
    http::jsonapi::{Input, collection, resource},
};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use uuid::Uuid;

fn private(response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response
}
async fn preview(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<Response> {
    Ok(private(resource(
        "post-seed-batches",
        app.post_seeds
            .preview(
                &actor,
                school,
                input.decode::<PreviewInput>("post-seed-batches")?,
            )
            .await?,
    )))
}
async fn commit(
    State(app): State<Application>,
    actor: Actor,
    Path((school, batch)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<Response> {
    Ok(private(resource(
        "post-seed-batches",
        app.post_seeds
            .commit(
                &actor,
                school,
                batch,
                input.decode::<CommitInput>("post-seed-batches")?,
            )
            .await?,
    )))
}
async fn revoke(
    State(app): State<Application>,
    actor: Actor,
    Path((school, batch)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<Response> {
    Ok(private(resource(
        "post-seed-batches",
        app.post_seeds
            .revoke(
                &actor,
                school,
                batch,
                input.decode::<RevokeInput>("post-seed-batches")?,
            )
            .await?,
    )))
}
async fn detail(
    State(app): State<Application>,
    actor: Actor,
    Path((school, batch)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    Ok(private(resource(
        "post-seed-batches",
        app.post_seeds
            .read(&actor, school, "batches", Some(batch), SeedQuery::default())
            .await?,
    )))
}
async fn read(
    app: Application,
    actor: Actor,
    school: Uuid,
    kind: &str,
    batch: Option<Uuid>,
    query: SeedQuery,
) -> Result<Response> {
    let page = query.page();
    let value = app
        .post_seeds
        .read(&actor, school, kind, batch, query)
        .await?;
    let values = value.as_array().cloned().ok_or(Error::Unavailable)?;
    Ok(private(collection(
        &format!("post-seed-{kind}"),
        values,
        &page,
    )))
}
async fn list(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Query(query): Query<SeedQuery>,
) -> Result<Response> {
    read(app, actor, school, "batches", None, query).await
}
async fn authors(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Query(query): Query<SeedQuery>,
) -> Result<Response> {
    read(app, actor, school, "authors", None, query).await
}
async fn avatars(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<Response> {
    read(app, actor, school, "avatars", None, SeedQuery::default()).await
}
async fn items(
    State(app): State<Application>,
    actor: Actor,
    Path((school, batch)): Path<(Uuid, Uuid)>,
    Query(query): Query<SeedQuery>,
) -> Result<Response> {
    read(app, actor, school, "items", Some(batch), query).await
}
async fn avatar(
    State(app): State<Application>,
    actor: Actor,
    Path((school, slot)): Path<(Uuid, u8)>,
) -> Result<Response> {
    app.post_seeds
        .read(&actor, school, "avatars", None, SeedQuery::default())
        .await?;
    let bytes = crate::community::south::adapter::postgres::post_seed::avatar_png(slot)?;
    Ok(private((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        bytes,
    )))
}
async fn author_avatar(
    State(app): State<Application>,
    actor: Actor,
    Path((school, author)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let value = app
        .post_seeds
        .read(
            &actor,
            school,
            "author-avatar",
            Some(author),
            SeedQuery::default(),
        )
        .await?;
    let id = crate::business::uuid(&value, "id")?;
    let (mime, bytes) = app.media.0.read(id).await?;
    Ok(private((
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime)],
        bytes,
    )))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route(
            "/api/v1/schools/{school}/post-seed-batches/preview",
            post(preview),
        )
        .route("/api/v1/schools/{school}/post-seed-batches", get(list))
        .route(
            "/api/v1/schools/{school}/post-seed-batches/{batch}",
            get(detail),
        )
        .route(
            "/api/v1/schools/{school}/post-seed-batches/{batch}/commit",
            post(commit),
        )
        .route(
            "/api/v1/schools/{school}/post-seed-batches/{batch}/revoke",
            post(revoke),
        )
        .route(
            "/api/v1/schools/{school}/post-seed-batches/{batch}/items",
            get(items),
        )
        .route("/api/v1/schools/{school}/post-seed-authors", get(authors))
        .route(
            "/api/v1/schools/{school}/post-seed-authors/{author}/avatar",
            get(author_avatar),
        )
        .route("/api/v1/schools/{school}/post-seed-avatars", get(avatars))
        .route(
            "/api/v1/schools/{school}/post-seed-avatars/{slot}",
            get(avatar),
        )
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
}
