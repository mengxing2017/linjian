use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result},
    community::domain::{PostInput, post_import::CommitPostImport, post_management::*},
    http::jsonapi::{ApiResponse, Input, collection, completed, resource, singleton},
};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde_json::json;
use uuid::Uuid;
fn private(response: ApiResponse) -> Response {
    let mut r = response.into_response();
    r.headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    r
}
async fn list(
    State(app): State<Application>,
    a: Actor,
    Path(s): Path<Uuid>,
    Query(f): Query<PostManagementFilters>,
) -> Result<Response> {
    if let Err(error) = f.validate() {
        let message = match error {
            Error::Invalid(message) => message,
            _ => "查询条件无效".into(),
        };
        return Ok(crate::http::error::response(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_request",
            &message,
        ));
    }
    let page = f.pagination();
    let result = app.community.management_list(&a, s, f).await?;
    let values = result
        .items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::Unavailable)?;
    Ok(private(
        collection("post-management", values, &page)
            .meta(json!({"order_revision":result.order_revision})),
    ))
}
async fn detail(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    Ok(private(resource(
        "post-management",
        app.community.management_detail(&a, s, id).await?,
    )))
}
async fn create(
    State(app): State<Application>,
    a: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "post-management",
        app.community
            .management_write(&a, s, None, input.decode::<PostInput>("post-management")?)
            .await?,
    ))
}
async fn update(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "post-management",
        app.community
            .management_write(
                &a,
                s,
                Some(id),
                input.decode::<PostInput>("post-management")?,
            )
            .await?,
    ))
}
async fn pin(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "post-management",
        app.community
            .management_pin(&a, s, id, input.decode::<PinPostInput>("post-management")?)
            .await?,
    ))
}
async fn status(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "post-management",
        app.community
            .management_status(
                &a,
                s,
                id,
                input.decode::<SetPostStatusInput>("post-management")?,
            )
            .await?,
    ))
}
async fn delete(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Response> {
    let Some(revision) = headers
        .get(header::IF_MATCH)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix('"'))
        .and_then(|v| v.strip_suffix('"'))
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
    else {
        return Ok(crate::http::error::response(
            axum::http::StatusCode::BAD_REQUEST,
            "invalid_request",
            "删除需要 If-Match 版本号",
        ));
    };
    app.community.management_delete(&a, s, id, revision).await?;
    Ok(completed().into_response())
}
async fn reorder(
    State(app): State<Application>,
    a: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    let revision = app
        .community
        .management_reorder(
            &a,
            s,
            input.decode::<ReorderPostsInput>("post-order-changes")?,
        )
        .await?;
    Ok(singleton(
        "post-order-changes",
        s,
        json!({"order_revision":revision}),
    ))
}
async fn preview(
    State(app): State<Application>,
    a: Actor,
    Path(s): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Response> {
    a.platform()?;
    let field = multipart
        .next_field()
        .await
        .map_err(|_| Error::Invalid("导入文件格式错误".into()))?
        .ok_or_else(|| Error::Invalid("请选择导入文件".into()))?;
    if field.name() != Some("file") {
        return Err(Error::Invalid("需要 file 文件字段".into()));
    }
    let name = field
        .file_name()
        .ok_or_else(|| Error::Invalid("文件名缺失".into()))?
        .to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| Error::Invalid("导入文件超过限制或读取失败".into()))?;
    if multipart
        .next_field()
        .await
        .map_err(|_| Error::Invalid("导入文件格式错误".into()))?
        .is_some()
    {
        return Err(Error::Invalid("仅允许一个文件".into()));
    }
    Ok(private(resource(
        "post-imports",
        app.community
            .preview_post_import(&a, s, name, bytes.to_vec())
            .await?,
    )))
}
async fn import_detail(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    Ok(private(resource(
        "post-imports",
        app.community.post_import_detail(&a, s, id).await?,
    )))
}
async fn commit(
    State(app): State<Application>,
    a: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<Response> {
    Ok(private(resource(
        "post-imports",
        app.community
            .commit_post_import(&a, s, id, input.decode::<CommitPostImport>("post-imports")?)
            .await?,
    )))
}
pub(super) fn router() -> Router<Application> {
    Router::new()
        .route(
            "/api/v1/schools/{school}/post-management",
            get(list).post(create),
        )
        .route(
            "/api/v1/schools/{school}/post-management/order-changes",
            post(reorder),
        )
        .route(
            "/api/v1/schools/{school}/post-management/{id}",
            get(detail).put(update).delete(delete),
        )
        .route(
            "/api/v1/schools/{school}/post-management/{id}/pin",
            put(pin),
        )
        .route(
            "/api/v1/schools/{school}/post-management/{id}/status",
            put(status),
        )
        .route(
            "/api/v1/schools/{school}/post-imports",
            post(preview).layer(DefaultBodyLimit::max(5 * 1024 * 1024 + 65536)),
        )
        .route(
            "/api/v1/schools/{school}/post-imports/{id}",
            get(import_detail),
        )
        .route(
            "/api/v1/schools/{school}/post-imports/{id}/commit",
            post(commit),
        )
}
