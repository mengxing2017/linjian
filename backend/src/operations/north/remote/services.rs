use super::*;
use crate::business::Page;
use crate::operations::domain::services::*;
use serde::Deserialize;
use serde_json::Value;

fn entity(uri: &axum::http::Uri) -> Result<&str> {
    uri.path()
        .split('/')
        .find(|part| {
            matches!(
                *part,
                "home-shortcuts"
                    | "operation-service-categories"
                    | "operation-service-targets"
                    | "operation-partners"
                    | "operation-cooperations"
                    | "marketing-slots"
            )
        })
        .ok_or(Error::NotFound)
}
async fn run(
    app: Application,
    actor: Actor,
    scope: Option<Uuid>,
    command: ServiceCommand,
    kind: &str,
) -> Result<ApiResponse> {
    Ok(resource(
        kind,
        service(app)
            .execute(&actor, scope, Command::Services(command))
            .await?,
    ))
}
async fn list(
    State(app): State<Application>,
    actor: Actor,
    Path(keys): Keys,
    OriginalUri(uri): OriginalUri,
    Params(query): Params<CatalogQuery>,
) -> Result<ApiResponse> {
    let entity = entity(&uri)?;
    run(
        app,
        actor,
        scope(&keys)?,
        ServiceCommand::ManageList {
            entity: entity.into(),
            query,
        },
        entity,
    )
    .await
}
async fn manage_detail(
    State(app): State<Application>,
    actor: Actor,
    Path(keys): Keys,
    OriginalUri(uri): OriginalUri,
) -> Result<ApiResponse> {
    let entity = entity(&uri)?;
    run(
        app,
        actor,
        scope(&keys)?,
        ServiceCommand::ManageDetail {
            entity: entity.into(),
            id: key(&keys, "id")?,
        },
        entity,
    )
    .await
}
async fn options(
    State(app): State<Application>,
    actor: Actor,
    Path(keys): Keys,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        scope(&keys)?,
        ServiceCommand::Options,
        "operation-service-target-options",
    )
    .await
}
async fn save(
    State(app): State<Application>,
    actor: Actor,
    Path(keys): Keys,
    OriginalUri(uri): OriginalUri,
    input: Input,
) -> Result<ApiResponse> {
    let entity = entity(&uri)?;
    run(
        app,
        actor,
        scope(&keys)?,
        ServiceCommand::Save {
            entity: entity.into(),
            id: keys.get("id").map(|_| key(&keys, "id")).transpose()?,
            input: input.decode::<Value>(entity)?,
        },
        entity,
    )
    .await
}
async fn remove(
    State(app): State<Application>,
    actor: Actor,
    Path(keys): Keys,
    OriginalUri(uri): OriginalUri,
    input: Input,
) -> Result<ApiResponse> {
    let entity = entity(&uri)?;
    service(app)
        .execute(
            &actor,
            scope(&keys)?,
            Command::Services(ServiceCommand::Delete {
                entity: entity.into(),
                id: key(&keys, "id")?,
                revision: input.decode::<Revision>(entity)?.revision,
            }),
        )
        .await?;
    Ok(completed())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionRequest {
    revision: i64,
    version_id: Uuid,
    version_revision: i64,
    reason: String,
}
async fn decision(
    State(app): State<Application>,
    actor: Actor,
    Path((target, action)): Path<(Uuid, String)>,
    input: Input,
) -> Result<ApiResponse> {
    let request: DecisionRequest = input.decode("operation-target-reviews")?;
    run(
        app,
        actor,
        None,
        ServiceCommand::TargetDecision {
            id: request.version_id,
            input: DecisionInput {
                target_id: Some(target),
                target_revision: Some(request.revision),
                revision: request.version_revision,
                decision: action,
                reason: request.reason,
            },
        },
        "operation-target-reviews",
    )
    .await
}
async fn catalog(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Params(query): Params<CatalogQuery>,
) -> Result<ApiResponse> {
    if query.at.is_some() {
        return Err(Error::Invalid("成员接口不接受预览时间".into()));
    }
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Catalog(query),
        "operation-services",
    )
    .await
}
async fn preview(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    Params(mut query): Params<CatalogQuery>,
) -> Result<ApiResponse> {
    query.at = Some(query.at.unwrap_or_else(chrono::Utc::now));
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Preview(query),
        "home-shortcut-previews",
    )
    .await
}
async fn draft_preview(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::DraftPreview(input.decode("home-shortcut-previews")?),
        "home-shortcut-previews",
    )
    .await
}
async fn detail(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Detail(id),
        "operation-services",
    )
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveInput {
    version: Option<Uuid>,
    channel: Option<String>,
}
async fn resolve(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
    Params(query): Params<ResolveInput>,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Resolve {
            id,
            version: query.version,
            channel: query.channel,
        },
        "operation-service-resolutions",
    )
    .await
}
async fn overlay(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Override {
            id,
            input: input.decode("home-shortcut-overrides")?,
        },
        "home-shortcut-overrides",
    )
    .await
}
async fn reset(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Reset {
            id,
            revision: input
                .decode::<Revision>("home-shortcut-overrides")?
                .revision,
        },
        "home-shortcut-overrides",
    )
    .await
}
async fn batch(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    OriginalUri(uri): OriginalUri,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Batch {
            reorder: uri.path().ends_with("/order"),
            input: input.decode("shortcut-batches")?,
        },
        "shortcut-batches",
    )
    .await
}
async fn metric(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Metric(input.decode("operation-events")?),
        "operation-events",
    )
    .await
}
async fn outcome(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Outcome(input.decode("operation-action-events")?),
        "operation-action-events",
    )
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaQuery {
    version: Option<Uuid>,
    channel: Option<String>,
}
async fn media(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id, asset)): Path<(Uuid, Uuid, Uuid)>,
    Params(query): Params<MediaQuery>,
) -> Result<Response> {
    binary(
        service(app)
            .execute(
                &actor,
                Some(school),
                Command::Services(ServiceCommand::Media {
                    id,
                    asset,
                    version: query.version,
                    channel: query.channel,
                }),
            )
            .await?,
    )
}
async fn report(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Report(input.decode("operation-reports")?),
        "operation-reports",
    )
    .await
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportPage {
    #[serde(rename = "page[number]")]
    number: Option<i64>,
    #[serde(rename = "page[size]")]
    size: Option<i64>,
}
impl ReportPage {
    fn page(self) -> Page {
        Page {
            page: self.number,
            page_size: self.size,
            ..Default::default()
        }
    }
}
async fn read_report(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
    Params(page): Params<ReportPage>,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::ReadReport {
            id,
            export: false,
            page: page.page(),
        },
        "operation-reports",
    )
    .await
}
async fn export(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let result = service(app)
        .execute(
            &actor,
            Some(school),
            Command::Services(ServiceCommand::ReadReport {
                id,
                export: true,
                page: Page::default(),
            }),
        )
        .await?;
    report_csv(result)
}
fn report_csv(result: Value) -> Result<Response> {
    let mut csv = csv::Writer::from_writer(Vec::new());
    let columns = [
        "day",
        "school_id",
        "resource_id",
        "channel",
        "target_version",
        "title",
        "provider",
        "cooperation_id",
        "placement",
        "views",
        "clicks",
        "api_success",
        "cancel",
        "fail",
        "copy_success",
        "qr_displayed",
        "webview_loaded",
        "webview_failed",
        "unreported",
        "ctr",
        "success_rate",
        "cancel_rate",
        "webview_load_rate",
    ];
    csv.write_record(columns).map_err(|_| Error::Unavailable)?;
    for row in result["report"]["rows"]
        .as_array()
        .ok_or(Error::Unavailable)?
    {
        let fields: Vec<String> = columns
            .iter()
            .map(|key| {
                let cell = if ["title", "provider", "cooperation_id", "placement"].contains(key) {
                    &row["dimensions"][*key]
                } else {
                    &row[*key]
                };
                let value = cell.as_str().map(str::to_owned).unwrap_or_else(|| {
                    if cell.is_null() {
                        String::new()
                    } else {
                        cell.to_string()
                    }
                });
                if value
                    .trim_start()
                    .starts_with(['=', '+', '-', '@', '\t', '\r'])
                {
                    format!("'{value}")
                } else {
                    value
                }
            })
            .collect();
        csv.write_record(fields).map_err(|_| Error::Unavailable)?;
    }
    let bytes = csv.into_inner().map_err(|_| Error::Unavailable)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=linjian-operation-report.csv",
            ),
        ],
        bytes,
    )
        .into_response())
}
async fn marketing(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::Marketing,
        "school-marketing",
    )
    .await
}
async fn marketing_event(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::MarketingEvent(input.decode("marketing-events")?),
        "marketing-events",
    )
    .await
}
async fn marketing_settings(
    State(app): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::MarketingSettings,
        "school-marketing",
    )
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarketingSetting {
    revision: i64,
    enabled: bool,
}
async fn marketing_setting(
    State(app): State<Application>,
    actor: Actor,
    Path((school, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let value: MarketingSetting = input.decode("school-marketing")?;
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::MarketingSetting {
            id,
            revision: value.revision,
            enabled: value.enabled,
        },
        "school-marketing",
    )
    .await
}
async fn platform_draft_preview(
    State(app): State<Application>,
    actor: Actor,
    input: Input,
) -> Result<ApiResponse> {
    actor.platform()?;
    let mut input: Value = input.decode("home-shortcut-previews")?;
    let school = crate::business::uuid(&input, "school_id")?;
    input
        .as_object_mut()
        .ok_or_else(|| Error::Invalid("预览结构无效".into()))?
        .remove("school_id");
    let draft: DraftPreviewInput = decode(&input)?;
    crate::business::require(draft.platform_default, "平台预览只接受平台默认草稿")?;
    run(
        app,
        actor,
        Some(school),
        ServiceCommand::DraftPreview(draft),
        "home-shortcut-previews",
    )
    .await
}
async fn platform_report(
    State(app): State<Application>,
    actor: Actor,
    input: Input,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        None,
        ServiceCommand::Report(input.decode("operation-reports")?),
        "operation-reports",
    )
    .await
}
async fn platform_read_report(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
    Params(page): Params<ReportPage>,
) -> Result<ApiResponse> {
    run(
        app,
        actor,
        None,
        ServiceCommand::ReadReport {
            id,
            export: false,
            page: page.page(),
        },
        "operation-reports",
    )
    .await
}
async fn platform_export(
    State(app): State<Application>,
    actor: Actor,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let result = service(app)
        .execute(
            &actor,
            None,
            Command::Services(ServiceCommand::ReadReport {
                id,
                export: true,
                page: Page::default(),
            }),
        )
        .await?;
    report_csv(result)
}
pub(super) fn router() -> Router<Application> {
    let mut router = Router::new();
    for root in ["/api/v1/platform", "/api/v1/schools/{school}"] {
        router = router.route(
            &format!("{root}/operation-service-target-options"),
            get(options),
        );
        for entity in [
            "home-shortcuts",
            "operation-service-categories",
            "operation-service-targets",
            "operation-partners",
            "operation-cooperations",
            "marketing-slots",
        ] {
            router = router
                .route(&format!("{root}/{entity}"), get(list).post(save))
                .route(
                    &format!("{root}/{entity}/{{id}}"),
                    get(manage_detail).patch(save).delete(remove),
                );
        }
    }
    router
        .route(
            "/api/v1/platform/home-shortcuts/preview",
            post(platform_draft_preview),
        )
        .route("/api/v1/platform/operation-reports", post(platform_report))
        .route(
            "/api/v1/platform/operation-reports/{id}",
            get(platform_read_report),
        )
        .route(
            "/api/v1/platform/operation-reports/{id}/export",
            get(platform_export),
        )
        .route(
            "/api/v1/platform/operation-service-targets/{target}/{action}",
            post(decision),
        )
        .route("/api/v1/schools/{school}/operation-services", get(catalog))
        .route(
            "/api/v1/schools/{school}/operation-services/{id}",
            get(detail),
        )
        .route(
            "/api/v1/schools/{school}/operation-services/{id}/resolve",
            get(resolve),
        )
        .route(
            "/api/v1/schools/{school}/operation-services/{id}/media/{asset}",
            get(media),
        )
        .route(
            "/api/v1/schools/{school}/home-shortcuts/preview",
            get(preview).post(draft_preview),
        )
        .route(
            "/api/v1/schools/{school}/home-shortcut-overrides/{id}",
            axum::routing::patch(overlay).delete(reset),
        )
        .route("/api/v1/schools/{school}/home-shortcuts/order", post(batch))
        .route(
            "/api/v1/schools/{school}/home-shortcuts/batch-status",
            post(batch),
        )
        .route("/api/v1/schools/{school}/operation-events", post(metric))
        .route(
            "/api/v1/schools/{school}/operation-action-events",
            post(outcome),
        )
        .route("/api/v1/schools/{school}/operation-reports", post(report))
        .route(
            "/api/v1/schools/{school}/operation-reports/{id}",
            get(read_report),
        )
        .route(
            "/api/v1/schools/{school}/operation-reports/{id}/export",
            get(export),
        )
        .route("/api/v1/schools/{school}/marketing", get(marketing))
        .route(
            "/api/v1/schools/{school}/marketing-settings",
            get(marketing_settings),
        )
        .route(
            "/api/v1/schools/{school}/marketing-events",
            post(marketing_event),
        )
        .route(
            "/api/v1/schools/{school}/marketing/{id}",
            axum::routing::patch(marketing_setting),
        )
}

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn service_resolve_and_media_queries_accept_version_and_channel() {
        let uri = "/?version=01a201bc-0058-7006-9181-9e875955c7e4&channel=wechat"
            .parse()
            .unwrap();
        let Params(resolve) = Params::<ResolveInput>::try_from_uri(&uri).unwrap();
        let Params(media) = Params::<MediaQuery>::try_from_uri(&uri).unwrap();
        assert!(resolve.version.is_some());
        assert_eq!(resolve.version, media.version);
        assert_eq!(resolve.channel.as_deref(), Some("wechat"));
        assert_eq!(resolve.channel, media.channel);
        let empty = "/".parse().unwrap();
        assert!(Params::<ResolveInput>::try_from_uri(&empty).is_ok());
        assert!(Params::<MediaQuery>::try_from_uri(&empty).is_ok());
        let invalid = "/?version=invalid".parse().unwrap();
        assert!(Params::<ResolveInput>::try_from_uri(&invalid).is_err());
        assert!(Params::<MediaQuery>::try_from_uri(&invalid).is_err());
    }

    #[test]
    fn report_list_queries_accept_pagination() {
        let uri = "/?page%5Bnumber%5D=2&page%5Bsize%5D=10".parse().unwrap();
        let Params(query) = Params::<ReportPage>::try_from_uri(&uri).unwrap();
        assert_eq!(query.page().bounds().unwrap(), (10, 10));
        let Params(query) = Params::<ReportPage>::try_from_uri(&"/".parse().unwrap()).unwrap();
        assert_eq!(query.page().bounds().unwrap(), (30, 0));
        let invalid = "/?page[number]=invalid".parse().unwrap();
        assert!(Params::<ReportPage>::try_from_uri(&invalid).is_err());
    }
}
