use crate::{
    bootstrap::Application,
    business::{Actor, Error, Result, require},
    http::jsonapi::{Input, resource, singleton},
    statistics::{domain::*, north::local::csv},
};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
async fn school(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Query(q): Query<crate::statistics::domain::Query>,
) -> Result<Response> {
    Ok(resource(
        "school-statistics",
        a.statistics.report(&actor, Some(s), q).await?,
    )
    .into_response())
}
async fn platform(
    State(a): State<Application>,
    actor: Actor,
    Query(q): Query<crate::statistics::domain::Query>,
) -> Result<Response> {
    let mut report = a.statistics.report(&actor, None, q).await?;
    report.schools.clear();
    report.school_ids.clear();
    report.school_names.clear();
    Ok(resource("platform-statistics", report).into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportQuery {
    kind: Option<String>,
}
fn export_response(report: &Report, kind: &str) -> Result<Response> {
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"linjian-statistics-{}.csv\"",
                    report.id
                ),
            ),
        ],
        csv(report, kind)?,
    )
        .into_response())
}
async fn school_export(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    Query(q): Query<ExportQuery>,
) -> Result<Response> {
    actor.permission(s, "statistics.school.export")?;
    export_response(
        &a.statistics.snapshot(&actor, Some(s), id).await?,
        q.kind.as_deref().unwrap_or("periods"),
    )
}
async fn platform_export(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    Query(q): Query<ExportQuery>,
) -> Result<Response> {
    export_response(
        &a.statistics.snapshot(&actor, None, id).await?,
        q.kind.as_deref().unwrap_or("periods"),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchoolsQuery {
    page: Option<usize>,
    size: Option<usize>,
    sort: Option<String>,
    descending: Option<bool>,
}
async fn schools(
    State(a): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    Query(q): Query<SchoolsQuery>,
) -> Result<Response> {
    let mut r = a.statistics.snapshot(&actor, None, id).await?;
    let p = q.page.unwrap_or(1);
    let size = q.size.unwrap_or(20);
    require(
        (1..=100000).contains(&p) && (1..=100).contains(&size),
        "分页参数无效",
    )?;
    let sort = q.sort.as_deref().unwrap_or("posts");
    require(
        matches!(
            sort,
            "name"
                | "posts"
                | "comments"
                | "participants"
                | "registrations"
                | "cumulative_registrations"
                | "active_users"
        ),
        "排序指标无效",
    )?;
    r.schools.sort_by(|a, b| {
        let n = |r: &SchoolRow| match sort {
            "comments" => r.metrics.comments,
            "participants" => r.metrics.participants,
            "registrations" => r.metrics.registrations,
            "cumulative_registrations" => r.metrics.cumulative_registrations,
            "active_users" => r.metrics.active_users,
            _ => r.metrics.posts,
        };
        let cmp = if sort == "name" {
            a.name.cmp(&b.name)
        } else {
            n(a).cmp(&n(b))
        };
        let cmp = if q.descending.unwrap_or(true) {
            cmp.reverse()
        } else {
            cmp
        };
        cmp.then(a.id.cmp(&b.id))
    });
    let total = r.schools.len();
    let rows: Vec<_> = r
        .schools
        .into_iter()
        .skip((p - 1) * size)
        .take(size)
        .collect();
    Ok(resource("school-statistics-rows",rows).meta(json!({"page":{"number":p,"size":size,"has_more":p*size<total},"total":total,"snapshot_id":id})).into_response())
}
async fn activity(
    State(a): State<Application>,
    actor: Actor,
    h: HeaderMap,
    input: Input,
) -> Result<Response> {
    if actor.is_admin_browser() || h.contains_key(header::ORIGIN) {
        return Err(Error::Forbidden);
    }
    let v: ActivityInput = input.decode("miniapp-activity-events")?;
    v.validate()?;
    a.statistics.validate_activity(&actor, &v).await?;
    a.activity.event(
        &actor,
        Some(v.school_id),
        &v.kind,
        Some(v.event_id),
        v.post_id,
    );
    Ok((
        StatusCode::ACCEPTED,
        singleton(
            "miniapp-activity-events",
            v.event_id,
            json!({"accepted":true}),
        ),
    )
        .into_response())
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v1/schools/{school}/statistics", get(school))
        .route("/api/v1/platform/statistics", get(platform))
        .route(
            "/api/v1/schools/{school}/statistics/snapshots/{id}/export",
            get(school_export),
        )
        .route(
            "/api/v1/platform/statistics/snapshots/{id}/export",
            get(platform_export),
        )
        .route(
            "/api/v1/platform/statistics/snapshots/{id}/schools",
            get(schools),
        )
        .route("/api/v1/miniapp/activity-events", post(activity))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("private, no-store"),
                );
                r
            },
        ))
}
