use crate::{
    bootstrap::Application,
    business::{Actor, Result},
    http::jsonapi::{ApiResponse, singleton},
};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    purpose: String,
    key: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lease {
    asset_ids: Vec<Uuid>,
}
use uuid::Uuid;
async fn upload(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Query(q): Query<Upload>,
    b: Bytes,
) -> Result<ApiResponse> {
    let value = a
        .media
        .put(&actor, s, &q.purpose, b.to_vec(), q.key)
        .await?;
    Ok(singleton(
        "media",
        value["id"].as_str().unwrap_or_default(),
        value.clone(),
    ))
}
async fn download(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let (mime, b) = a.media.get(&actor, s, id).await?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "private, no-store".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        b,
    )
        .into_response())
}
async fn lease(
    State(a): State<Application>,
    actor: Actor,
    Path((s, draft)): Path<(Uuid, Uuid)>,
    input: crate::http::jsonapi::Input,
) -> Result<ApiResponse> {
    let v: Lease = input.decode("media-draft-leases")?;
    Ok(crate::http::jsonapi::resource(
        "media-draft-leases",
        a.media.0.lease(&actor, s, draft, v.asset_ids).await?,
    ))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/media", post(upload))
        .route(
            "/api/v1/schools/{school}/media-draft-leases/{draft}",
            axum::routing::put(lease),
        )
        .route("/api/v1/schools/{school}/media/{id}", get(download))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
}

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn media_upload_query_accepts_purpose_and_uuid_key() {
        let uri = "/?purpose=post_image&key=01a201bc-0058-7006-9181-9e875955c7e4"
            .parse()
            .unwrap();
        let Query(query) = Query::<Upload>::try_from_uri(&uri).unwrap();
        assert_eq!(query.purpose, "post_image");
        assert_eq!(
            query.key.to_string(),
            "01a201bc-0058-7006-9181-9e875955c7e4"
        );
        for value in ["/?purpose=post_image", "/?purpose=post_image&key=invalid"] {
            assert!(Query::<Upload>::try_from_uri(&value.parse().unwrap()).is_err());
        }
    }
}
