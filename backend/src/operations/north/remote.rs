use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result, Revision},
    http::jsonapi::{ApiResponse, Input, collection, completed, resource},
    operations::{
        domain::*,
        north::local::Service,
        south::{adapter::postgres::Repository, port::Command},
    },
};
use axum::{
    Router,
    extract::{ConnectInfo, OriginalUri, Path, Query as Params, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::{collections::HashMap, net::SocketAddr};
use uuid::Uuid;
type Keys = Path<HashMap<String, String>>;
fn key(p: &HashMap<String, String>, k: &str) -> Result<Uuid> {
    p.get(k).and_then(|v| v.parse().ok()).ok_or(Error::NotFound)
}
fn scope(p: &HashMap<String, String>) -> Result<Option<Uuid>> {
    p.get("school").map(|_| key(p, "school")).transpose()
}
fn kind(uri: &axum::http::Uri) -> Result<(&'static str, &'static str)> {
    if uri.path().split('/').any(|s| s == "group-qrcodes") {
        Ok(("qrcode", "group-qrcodes"))
    } else if uri.path().split('/').any(|s| s == "home-banners") {
        Ok(("banner", "home-banners"))
    } else {
        Err(Error::NotFound)
    }
}
fn service(a: Application) -> Service {
    Service(Repository(a.pool))
}
async fn list(
    State(a): State<Application>,
    actor: Actor,
    Path(p): Keys,
    OriginalUri(uri): OriginalUri,
    Params(q): Params<Query>,
) -> Result<ApiResponse> {
    let (k, t) = kind(&uri)?;
    let page = q.page();
    let v = service(a)
        .execute(&actor, scope(&p)?, Command::List { kind: k.into(), q })
        .await?;
    Ok(collection(
        t,
        serde_json::from_value(v).map_err(|_| Error::Unavailable)?,
        &page,
    ))
}
async fn one(
    State(a): State<Application>,
    actor: Actor,
    Path(p): Keys,
    OriginalUri(uri): OriginalUri,
) -> Result<ApiResponse> {
    let (k, t) = kind(&uri)?;
    Ok(resource(
        t,
        service(a)
            .execute(
                &actor,
                scope(&p)?,
                Command::Get {
                    kind: k.into(),
                    id: key(&p, "id")?,
                },
            )
            .await?,
    ))
}
async fn save(
    State(a): State<Application>,
    actor: Actor,
    Path(p): Keys,
    OriginalUri(uri): OriginalUri,
    input: Input,
) -> Result<ApiResponse> {
    let (k, t) = kind(&uri)?;
    Ok(resource(
        t,
        service(a)
            .execute(
                &actor,
                scope(&p)?,
                Command::Save {
                    kind: k.into(),
                    id: p.get("id").map(|_| key(&p, "id")).transpose()?,
                    input: input.decode(t)?,
                },
            )
            .await?,
    ))
}
async fn delete(
    State(a): State<Application>,
    actor: Actor,
    Path(p): Keys,
    OriginalUri(uri): OriginalUri,
    input: Input,
) -> Result<ApiResponse> {
    let (k, t) = kind(&uri)?;
    service(a)
        .execute(
            &actor,
            scope(&p)?,
            Command::Delete {
                kind: k.into(),
                id: key(&p, "id")?,
                revision: input.decode::<Revision>(t)?.revision,
            },
        )
        .await?;
    Ok(completed())
}
async fn customer(State(a): State<Application>, actor: Actor) -> Result<ApiResponse> {
    Ok(resource(
        "customer-config",
        service(a).execute(&actor, None, Command::Customer).await?,
    ))
}
async fn save_customer(
    State(a): State<Application>,
    actor: Actor,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "customer-config",
        service(a)
            .execute(
                &actor,
                None,
                Command::SaveCustomer(input.decode("customer-config")?),
            )
            .await?,
    ))
}
async fn feedback_list(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Params(q): Params<Query>,
) -> Result<ApiResponse> {
    let page = q.page();
    let v = service(a)
        .execute(&actor, Some(s), Command::FeedbackList(q))
        .await?;
    Ok(collection(
        "feedback",
        serde_json::from_value(v).map_err(|_| Error::Unavailable)?,
        &page,
    ))
}
async fn feedback_get(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    Ok(resource(
        "feedback",
        service(a)
            .execute(&actor, Some(s), Command::FeedbackGet(id))
            .await?,
    ))
}
async fn feedback_delete(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    service(a)
        .execute(
            &actor,
            Some(s),
            Command::FeedbackDelete {
                id,
                revision: input.decode::<Revision>("feedback")?.revision,
            },
        )
        .await?;
    Ok(completed())
}
async fn submit(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    connect: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    input: Input,
) -> Result<ApiResponse> {
    let trusted = std::env::var("LINJIAN_TRUSTED_PROXY_IPS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect::<Vec<_>>();
    let ip = client_ip(
        connect.map(|c| c.0.0.ip()),
        headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        &trusted,
    )
    .map(|ip| ip.to_string());
    Ok(resource(
        "feedback",
        service(a)
            .execute(
                &actor,
                Some(s),
                Command::Submit {
                    input: input.decode("feedback")?,
                    ip,
                },
            )
            .await?,
    ))
}
async fn display(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "operations-display",
        service(a)
            .execute(&actor, Some(s), Command::Display)
            .await?,
    ))
}
async fn event(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "banner-events",
        service(a)
            .execute(
                &actor,
                Some(s),
                Command::Event(input.decode("banner-events")?),
            )
            .await?,
    ))
}
fn binary(v: serde_json::Value) -> Result<Response> {
    let bytes: Vec<u8> =
        serde_json::from_value(v["bytes"].clone()).map_err(|_| Error::Unavailable)?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                v["mime"]
                    .as_str()
                    .unwrap_or("application/octet-stream")
                    .to_owned(),
            ),
            (header::CACHE_CONTROL, "private, no-store".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        bytes,
    )
        .into_response())
}
async fn customer_media(
    State(a): State<Application>,
    actor: Actor,
    Path((s, k)): Path<(Uuid, String)>,
) -> Result<Response> {
    binary(
        service(a)
            .execute(&actor, Some(s), Command::CustomerMedia(k))
            .await?,
    )
}
async fn feedback_media(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id, media)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Response> {
    binary(
        service(a)
            .execute(&actor, Some(s), Command::FeedbackMedia { case: id, media })
            .await?,
    )
}
async fn resource_media(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    binary(
        service(a)
            .execute(&actor, Some(s), Command::ResourceMedia(id))
            .await?,
    )
}
pub fn router() -> Router<Application> {
    let mut router = Router::new();
    for root in ["/api/v1/schools/{school}", "/api/v1/platform"] {
        for k in ["group-qrcodes", "home-banners"] {
            router = router
                .route(&format!("{root}/{k}"), get(list).post(save))
                .route(
                    &format!("{root}/{k}/{{id}}"),
                    get(one).patch(save).delete(delete),
                );
        }
    }
    router
        .route(
            "/api/v1/platform/customer-config",
            get(customer).patch(save_customer),
        )
        .route(
            "/api/v1/schools/{school}/feedback",
            get(feedback_list).post(submit),
        )
        .route(
            "/api/v1/schools/{school}/feedback/{id}",
            get(feedback_get).delete(feedback_delete),
        )
        .route("/api/v1/schools/{school}/operations-display", get(display))
        .route("/api/v1/schools/{school}/banner-events", post(event))
        .route(
            "/api/v1/schools/{school}/customer-images/{kind}",
            get(customer_media),
        )
        .route(
            "/api/v1/schools/{school}/operations-images/{id}",
            get(resource_media),
        )
        .route(
            "/api/v1/schools/{school}/feedback/{id}/images/{media}",
            get(feedback_media),
        )
}
