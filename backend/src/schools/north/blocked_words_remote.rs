use crate::{
    bootstrap::Application,
    business::{Actor, Result, Revision},
    http::jsonapi::{ApiResponse, Input, completed, resource},
    schools::{
        domain::blocked_words::{OrderInput, WordInput, WordQuery},
        north::blocked_words::Service,
    },
};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

async fn school_list(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Query(q): Query<WordQuery>,
) -> Result<ApiResponse> {
    Ok(resource(
        "blocked-word-catalog",
        Service::from_pool(a.pool)
            .blocked_list(&actor, Some(s), &q)
            .await?,
    ))
}
async fn platform_list(
    State(a): State<Application>,
    actor: Actor,
    Query(q): Query<WordQuery>,
) -> Result<ApiResponse> {
    Ok(resource(
        "blocked-word-catalog",
        Service::from_pool(a.pool)
            .blocked_list(&actor, None, &q)
            .await?,
    ))
}
async fn school_create(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "blocked-words",
        Service::from_pool(a.pool)
            .blocked_save(
                &actor,
                Some(s),
                None,
                input.decode::<WordInput>("blocked-words")?,
            )
            .await?,
    ))
}
async fn platform_create(
    State(a): State<Application>,
    actor: Actor,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "blocked-words",
        Service::from_pool(a.pool)
            .blocked_save(
                &actor,
                None,
                None,
                input.decode::<WordInput>("blocked-words")?,
            )
            .await?,
    ))
}
async fn school_update(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "blocked-words",
        Service::from_pool(a.pool)
            .blocked_save(
                &actor,
                Some(s),
                Some(id),
                input.decode::<WordInput>("blocked-words")?,
            )
            .await?,
    ))
}
async fn platform_update(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "blocked-words",
        Service::from_pool(a.pool)
            .blocked_save(
                &actor,
                None,
                Some(id),
                input.decode::<WordInput>("blocked-words")?,
            )
            .await?,
    ))
}
async fn school_delete(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Service::from_pool(a.pool)
        .blocked_delete(
            &actor,
            Some(s),
            id,
            input.decode::<Revision>("blocked-words")?.revision,
        )
        .await?;
    Ok(completed())
}
async fn platform_delete(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Service::from_pool(a.pool)
        .blocked_delete(
            &actor,
            None,
            id,
            input.decode::<Revision>("blocked-words")?.revision,
        )
        .await?;
    Ok(completed())
}
async fn school_order(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Service::from_pool(a.pool)
        .blocked_order(
            &actor,
            Some(s),
            input.decode::<OrderInput>("blocked-words")?,
        )
        .await?;
    Ok(completed())
}
async fn platform_order(
    State(a): State<Application>,
    actor: Actor,
    input: Input,
) -> Result<ApiResponse> {
    Service::from_pool(a.pool)
        .blocked_order(&actor, None, input.decode::<OrderInput>("blocked-words")?)
        .await?;
    Ok(completed())
}
pub fn router() -> Router<Application> {
    Router::new()
        .route(
            "/api/v1/schools/{school}/blocked-word-imports",
            post(school_import).layer(DefaultBodyLimit::max(2 * 1024 * 1024 + 65536)),
        )
        .route(
            "/api/v1/platform/blocked-word-imports",
            post(platform_import).layer(DefaultBodyLimit::max(2 * 1024 * 1024 + 65536)),
        )
        .route(
            "/api/v1/schools/{school}/blocked-word-imports/{id}/commit",
            post(school_commit),
        )
        .route(
            "/api/v1/platform/blocked-word-imports/{id}/commit",
            post(platform_commit),
        )
        .route(
            "/api/v1/schools/{school}/blocked-words",
            get(school_list).post(school_create),
        )
        .route(
            "/api/v1/schools/{school}/blocked-words/order",
            post(school_order),
        )
        .route(
            "/api/v1/schools/{school}/blocked-words/{id}",
            axum::routing::put(school_update).delete(school_delete),
        )
        .route(
            "/api/v1/platform/blocked-words",
            get(platform_list).post(platform_create),
        )
        .route("/api/v1/platform/blocked-words/order", post(platform_order))
        .route(
            "/api/v1/platform/blocked-words/{id}",
            axum::routing::put(platform_update).delete(platform_delete),
        )
}

async fn upload(mut multipart: Multipart) -> Result<(String, axum::body::Bytes)> {
    use crate::business::{Error, require};
    let field = multipart
        .next_field()
        .await
        .map_err(|_| Error::Invalid("文件格式错误".into()))?
        .ok_or_else(|| Error::Invalid("请选择文件".into()))?;
    require(field.name() == Some("file"), "需要 file 文件字段")?;
    let name = field
        .file_name()
        .ok_or_else(|| Error::Invalid("文件名缺失".into()))?
        .to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| Error::Invalid("文件读取失败或超过限制".into()))?;
    require(
        multipart
            .next_field()
            .await
            .map_err(|_| Error::Invalid("文件格式错误".into()))?
            .is_none(),
        "仅允许一个文件",
    )?;
    Ok((name, bytes))
}
async fn school_import(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    multipart: Multipart,
) -> Result<ApiResponse> {
    let (name, bytes) = upload(multipart).await?;
    Ok(resource(
        "blocked-word-imports",
        Service::from_pool(a.pool)
            .blocked_import_preview(&actor, Some(s), &name, &bytes)
            .await?,
    ))
}
async fn platform_import(
    State(a): State<Application>,
    actor: Actor,
    multipart: Multipart,
) -> Result<ApiResponse> {
    let (name, bytes) = upload(multipart).await?;
    Ok(resource(
        "blocked-word-imports",
        Service::from_pool(a.pool)
            .blocked_import_preview(&actor, None, &name, &bytes)
            .await?,
    ))
}
#[derive(Deserialize)]
struct Commit {
    digest: String,
}
async fn school_commit(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let v = input.decode::<Commit>("blocked-word-imports")?;
    Ok(resource(
        "blocked-word-imports",
        Service::from_pool(a.pool)
            .blocked_import_commit(&actor, Some(s), id, &v.digest)
            .await?,
    ))
}
async fn platform_commit(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    let v = input.decode::<Commit>("blocked-word-imports")?;
    Ok(resource(
        "blocked-word-imports",
        Service::from_pool(a.pool)
            .blocked_import_commit(&actor, None, id, &v.digest)
            .await?,
    ))
}
