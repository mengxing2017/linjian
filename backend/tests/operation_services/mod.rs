use super::*;
use linjian_api::{
    business::{Actor, Error, SessionKind},
    operations::{
        domain::services::*,
        north::local::Service,
        south::{adapter::postgres::Repository, port::Command},
    },
};
fn save(entity: &str, input: Value, id: Option<Uuid>) -> Command {
    Command::Services(ServiceCommand::Save {
        entity: entity.into(),
        id,
        input,
    })
}
fn decision(id: Uuid, revision: i64, decision: &str) -> Command {
    Command::Services(ServiceCommand::TargetDecision {
        id,
        input: DecisionInput {
            target_id: None,
            target_revision: None,
            revision,
            decision: decision.into(),
            reason: "隔离测试审核".into(),
        },
    })
}
async fn actor(app: &bootstrap::Application, name: &str) -> Actor {
    let account = app
        .identity
        .bootstrap(name, name, "test-password-at-least-12", true)
        .await
        .unwrap();
    app.identity
        .0
        .session(
            account,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap()
        .1
}
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn service_review_school_overrides_metrics_and_withdrawal_are_one_closed_loop() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("service_operations_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = actor(&app, "root").await;
    let reviewer = actor(&app, "reviewer").await;
    let school: Uuid = app.schools.create(&root, "服务测试学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&root, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let media = linjian_api::schools::north::resources::Service(
        linjian_api::schools::south::adapter::postgres::Repository(pool.clone()),
    );
    let outside: Uuid = app.schools.create(&root, "未授权目标学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let asset = media
        .upload(&root, None, png.into_inner(), Default::default())
        .await
        .unwrap();
    let ops = Service(Repository(pool.clone(), Default::default()));
    let target=ops.execute(&root,None,save("operation-service-targets",json!({"school_ids":[school,other],"revision":0,"name":"校园图书服务","provider":"示例合作方","purpose":"查询图书","disclosure":"访问第三方提供的服务","contact":"站内客服","complaint":"学校投诉入口","authorization":"内部授权材料，不可下发","qualification":"隔离测试资料，不代表正式许可","channels":[{"key":"copy","label":"复制服务链接","action":{"kind":"copy_link","url":"https://library.example.com/service"}},{"key":"qr","label":"服务二维码","action":{"kind":"qr_code","asset_id":asset["image_ref"]["id"]}}]}),None)).await.unwrap();
    let version: Uuid = target["versions"][0]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    ops.execute(&root, None, decision(version, 1, "submit"))
        .await
        .unwrap();
    assert!(
        ops.execute(&root, None, decision(version, 2, "approve"))
            .await
            .is_err()
    );
    ops.execute(&reviewer, None, decision(version, 2, "approve"))
        .await
        .unwrap();
    let input = json!({"revision":0,"title":"图书服务","subtitle":"图书查询","badge":"合作","image_ref":asset["image_ref"],"category_id":null,"cooperation_id":null,"action":{"kind":"target_channel","version_id":version,"channel":"copy"},"active":true,"featured":true,"sort_order":1,"starts_at":null,"ends_at":null});
    let shortcut = ops
        .execute(&root, None, save("home-shortcuts", input.clone(), None))
        .await
        .unwrap();
    let id: Uuid = shortcut["id"].as_str().unwrap().parse().unwrap();
    let other_catalog = ops
        .execute(
            &root,
            Some(outside),
            Command::Services(ServiceCommand::Catalog(CatalogQuery::default())),
        )
        .await
        .unwrap();
    assert_eq!(other_catalog["items"], json!([]));
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Resolve {
                id,
                version: Some(Uuid::new_v4()),
                channel: Some("copy".into())
            })
        )
        .await
        .is_err()
    );
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Resolve {
                id,
                version: Some(version),
                channel: Some("unknown".into())
            })
        )
        .await
        .is_err()
    );
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Resolve {
                id,
                version: Some(version),
                channel: Some("copy".into())
            })
        )
        .await
        .is_ok()
    );
    let qr_asset: Uuid = asset["image_ref"]["id"].as_str().unwrap().parse().unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Media {
                id,
                asset: qr_asset,
                version: Some(version),
                channel: Some("qr".into())
            })
        )
        .await
        .is_ok()
    );
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Media {
                id,
                asset: Uuid::new_v4(),
                version: Some(version),
                channel: Some("qr".into())
            })
        )
        .await
        .is_err()
    );
    let catalog = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Catalog(CatalogQuery::default())),
        )
        .await
        .unwrap();
    assert_eq!(catalog["featured"][0]["id"], shortcut["id"]);
    assert!(!catalog.to_string().contains("内部授权材料"));
    assert!(
        catalog["items"][0]["channels"][1]["action"]["image_url"]
            .as_str()
            .unwrap()
            .contains(&school.to_string())
    );
    assert!(
        ops.execute(&root, Some(other), save("home-shortcuts", input, None))
            .await
            .is_err()
    );
    let overlay = OverrideInput {
        revision: 0,
        token: catalog["token"].as_str().unwrap().into(),
        presentation: Presentation {
            active: Some(false),
            ..Default::default()
        },
    };
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Override { id, input: overlay }),
    )
    .await
    .unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Detail(id))
        )
        .await
        .is_err()
    );
    assert!(
        ops.execute(
            &root,
            Some(other),
            Command::Services(ServiceCommand::Detail(id))
        )
        .await
        .is_ok()
    );
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Reset { id, revision: 1 }),
    )
    .await
    .unwrap();
    let click = MetricInput {
        event_id: Uuid::new_v4(),
        resource_id: id,
        event_type: "click".into(),
        placement: "service_detail".into(),
        attempt_id: Some(Uuid::new_v4()),
        display_cycle: Uuid::new_v4(),
        occurred_at: chrono::Utc::now(),
        channel: Some("copy".into()),
        target_version_id: Some(version),
    };
    let result = OutcomeInput {
        event_id: Uuid::new_v4(),
        click_id: click.event_id,
        outcome: "copy_success".into(),
        stage: "clipboard_api".into(),
        attempt_id: click.attempt_id.unwrap(),
        occurred_at: chrono::Utc::now(),
    };
    assert!(matches!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Outcome(result.clone()))
        )
        .await,
        Err(Error::Policy("click_not_received"))
    ));
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Metric(click.clone())),
    )
    .await
    .unwrap();
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Metric(click.clone())),
    )
    .await
    .unwrap();
    let mut mismatch = click.clone();
    mismatch.channel = Some("qr".into());
    assert!(matches!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Metric(mismatch))
        )
        .await,
        Err(Error::Conflict)
    ));
    ops.execute(&reviewer, None, decision(version, 3, "withdraw"))
        .await
        .unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Detail(id))
        )
        .await
        .is_err()
    );
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Media {
                asset: asset["image_ref"]["id"].as_str().unwrap().parse().unwrap(),
                version: None,
                id,
                channel: Some("qr".into())
            })
        )
        .await
        .is_err()
    );
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Outcome(result.clone())),
    )
    .await
    .unwrap();
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Outcome(result)),
    )
    .await
    .unwrap();
    let mut reused_attempt = click.clone();
    reused_attempt.event_id = Uuid::new_v4();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Metric(reused_attempt))
        )
        .await
        .is_err()
    );
    let counts:(i64,i64)=sqlx::query_as("SELECT clicks,copy_success FROM operation_daily_metrics WHERE school_id=$1 AND resource_id=$2").bind(school).bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (1, 1));
    let partner = ops
        .execute(
            &root,
            None,
            save(
                "operation-partners",
                json!({"revision":0,"name":"图书合作方","contact":"仅平台联系人","active":true}),
                None,
            ),
        )
        .await
        .unwrap();
    let cooperation_input = json!({"revision":0,"partner_id":partner["id"],"name":"校园图书赞助","model":"sponsor","disclosure":"本入口为合作推广","channel_code":"library_school","school_ids":[school],"active":true,"starts_at":null,"ends_at":null});
    let cooperation = ops
        .execute(
            &root,
            None,
            save("operation-cooperations", cooperation_input.clone(), None),
        )
        .await
        .unwrap();
    let mut target_input = target["versions"][0]["config"].clone();
    target_input["school_ids"] = json!([school]);
    target_input["partner_id"] = partner["id"].clone();
    target_input["cooperation_id"] = cooperation["id"].clone();
    let sponsored = ops
        .execute(
            &root,
            None,
            save("operation-service-targets", target_input, None),
        )
        .await
        .unwrap();
    let sponsored_version: Uuid = sponsored["versions"][0]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    ops.execute(&root, None, decision(sponsored_version, 1, "submit"))
        .await
        .unwrap();
    ops.execute(&reviewer, None, decision(sponsored_version, 2, "approve"))
        .await
        .unwrap();
    let entry=ops.execute(&root,None,save("home-shortcuts",json!({"revision":0,"title":"赞助图书服务","subtitle":"","badge":"免费","image_ref":asset["image_ref"],"category_id":null,"cooperation_id":cooperation["id"],"action":{"kind":"target_channel","version_id":sponsored_version,"channel":"copy"},"active":true,"featured":true,"sort_order":1,"starts_at":null,"ends_at":null}),None)).await.unwrap();
    let entry_id: Uuid = entry["id"].as_str().unwrap().parse().unwrap();
    let resolved = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Detail(entry_id)),
        )
        .await
        .unwrap();
    assert_eq!(resolved["badge"], "合作推广");
    let link = resolved["channels"][0]["action"]["url"].as_str().unwrap();
    assert!(link.contains("linjian_channel=library_school"));
    assert!(link.contains(&format!("linjian_school={school}")));
    assert!(!resolved.to_string().contains("仅平台联系人"));
    assert!(
        ops.execute(
            &root,
            Some(other),
            Command::Services(ServiceCommand::Detail(entry_id))
        )
        .await
        .is_err()
    );
    let mut paused = cooperation_input;
    paused["revision"] = json!(1);
    paused["active"] = json!(false);
    ops.execute(
        &root,
        None,
        save(
            "operation-cooperations",
            paused,
            Some(cooperation["id"].as_str().unwrap().parse().unwrap()),
        ),
    )
    .await
    .unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Detail(entry_id))
        )
        .await
        .is_err()
    );
    let state: String =
        sqlx::query_scalar("SELECT state FROM operation_target_versions WHERE id=$1")
            .bind(sponsored_version)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, "withdrawn");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let (token, _) = app
        .identity
        .0
        .session(
            root.id,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let response = linjian_api::http::app_router(app.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/schools/{school}/operation-services"))
                .header("cookie", format!("linjian_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let document: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(document["data"]["type"], "operation-services");
    assert_eq!(document["data"]["id"], "current");
    assert!(document["data"]["attributes"].get("id").is_none());
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../api-contract/fixtures/operation-services.json"
    ))
    .unwrap();
    for key in fixture["data"]["attributes"].as_object().unwrap().keys() {
        assert!(
            document["data"]["attributes"].get(key).is_some(),
            "missing {key}"
        );
    }
    let snapshot = migration::export(&pool, &Default::default()).await.unwrap();
    let target_schema = format!("service_restore_{}", Uuid::new_v4().simple());
    let target_pool = isolated(&admin, &target_schema).await;
    database::migrate(&target_pool).await.unwrap();
    migration::import(&target_pool, &snapshot, false, &Default::default())
        .await
        .unwrap();
    let restored:(i64,i64)=sqlx::query_as("SELECT clicks,copy_success FROM operation_daily_metrics WHERE school_id=$1 AND resource_id=$2").bind(school).bind(id).fetch_one(&target_pool).await.unwrap();
    assert_eq!(restored, (1, 1));
    target_pool.close().await;
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query(&format!("DROP SCHEMA {target_schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn draft_preview_batch_atomicity_reports_and_official_marketing_are_scoped() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("service_batches_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = actor(&app, "batch-root").await;
    let school: Uuid = app.schools.create(&root, "批量操作学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&root, "其他报表学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let media = linjian_api::schools::north::resources::Service(
        linjian_api::schools::south::adapter::postgres::Repository(pool.clone()),
    );
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let platform_asset = media
        .upload(&root, None, png.get_ref().clone(), Default::default())
        .await
        .unwrap();
    let local_asset = media
        .upload(&root, Some(school), png.into_inner(), Default::default())
        .await
        .unwrap();
    let ops = Service(Repository(pool.clone(), Default::default()));
    let input = |image: Value, title: &str| json!({"revision":0,"title":title,"subtitle":"","badge":"","image_ref":image,"category_id":null,"cooperation_id":null,"action":{"kind":"none"},"active":true,"featured":true,"sort_order":1,"starts_at":null,"ends_at":null});
    let global = ops
        .execute(
            &root,
            None,
            save(
                "home-shortcuts",
                input(platform_asset["image_ref"].clone(), "平台服务"),
                None,
            ),
        )
        .await
        .unwrap();
    let first = ops
        .execute(
            &root,
            Some(school),
            save(
                "home-shortcuts",
                input(local_asset["image_ref"].clone(), "本校一"),
                None,
            ),
        )
        .await
        .unwrap();
    let second = ops
        .execute(
            &root,
            Some(school),
            save(
                "home-shortcuts",
                input(local_asset["image_ref"].clone(), "本校二"),
                None,
            ),
        )
        .await
        .unwrap();
    let preview = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Preview(CatalogQuery::default())),
        )
        .await
        .unwrap();
    assert_eq!(preview["preview"], true);
    let token = preview["token"].as_str().unwrap().to_owned();
    let draft = DraftPreviewInput {
        token: token.clone(),
        at: chrono::Utc::now(),
        resource_id: None,
        platform_default: false,
        draft: serde_json::from_value(input(local_asset["image_ref"].clone(), "未保存草稿"))
            .unwrap(),
    };
    let result = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::DraftPreview(draft)),
        )
        .await
        .unwrap();
    assert!(
        result["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["title"] == "未保存草稿")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM school_resources WHERE kind='shortcut'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM operation_metric_events")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let global_id = global["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let first_id = first["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let second_id = second["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let bad = BatchInput {
        token: token.clone(),
        items: vec![
            BatchItem {
                id: global_id,
                revision: 0,
                active: None,
            },
            BatchItem {
                id: first_id,
                revision: 1,
                active: None,
            },
            BatchItem {
                id: second_id,
                revision: 9,
                active: None,
            },
        ],
    };
    assert!(matches!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Batch {
                reorder: true,
                input: bad
            })
        )
        .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM school_shortcut_overrides")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT revision FROM school_resources WHERE id=$1")
            .bind(first_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    let good = BatchInput {
        token: token.clone(),
        items: vec![
            BatchItem {
                id: global_id,
                revision: 0,
                active: None,
            },
            BatchItem {
                id: first_id,
                revision: 1,
                active: None,
            },
            BatchItem {
                id: second_id,
                revision: 1,
                active: None,
            },
        ],
    };
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Batch {
            reorder: true,
            input: good,
        }),
    )
    .await
    .unwrap();
    let catalog = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Catalog(CatalogQuery::default())),
        )
        .await
        .unwrap();
    assert_eq!(catalog["items"][0]["id"], global["id"]);
    assert_ne!(catalog["token"], token);
    let overlay = OverrideInput {
        revision: 1,
        token: catalog["token"].as_str().unwrap().into(),
        presentation: Presentation {
            subtitle: Some(String::new()),
            ..Default::default()
        },
    };
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Override {
            id: global_id,
            input: overlay,
        }),
    )
    .await
    .unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Media {
                asset: platform_asset["image_ref"]["id"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap(),
                version: None,
                id: global_id,
                channel: None
            })
        )
        .await
        .is_ok()
    );
    let event = MetricInput {
        event_id: Uuid::new_v4(),
        resource_id: global_id,
        event_type: "view".into(),
        placement: "home_shortcut".into(),
        attempt_id: None,
        display_cycle: Uuid::new_v4(),
        occurred_at: chrono::Utc::now(),
        channel: None,
        target_version_id: None,
    };
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Metric(event.clone())),
    )
    .await
    .unwrap();
    let mut duplicate_cycle = event.clone();
    duplicate_cycle.event_id = Uuid::new_v4();
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::Metric(duplicate_cycle)),
    )
    .await
    .unwrap();
    let day = chrono::Utc::now().date_naive();
    let start = day.and_hms_opt(0, 0, 0).unwrap().and_utc() - chrono::Duration::hours(8);
    let report = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Report(ReportInput {
                starts_at: start,
                ends_at: start + chrono::Duration::days(2),
                ..Default::default()
            })),
        )
        .await
        .unwrap();
    assert_eq!(report["report"]["rows"][0]["views"], 1);
    assert_eq!(report["report"]["coverage"]["partial"], true);
    let filtered = ops
        .execute(
            &root,
            None,
            Command::Services(ServiceCommand::Report(ReportInput {
                starts_at: start,
                ends_at: start + chrono::Duration::days(2),
                school_ids: vec![school],
                placement: Some("service_detail".into()),
                ..Default::default()
            })),
        )
        .await
        .unwrap();
    assert_eq!(filtered["report"]["rows"], json!([]));
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Report(ReportInput {
                starts_at: start,
                ends_at: start + chrono::Duration::days(2),
                school_ids: vec![other],
                ..Default::default()
            }))
        )
        .await
        .is_err()
    );
    let report_id = report["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    let beyond = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::ReadReport {
                id: report_id,
                export: false,
                page: linjian_api::business::Page {
                    page: Some(2),
                    page_size: Some(100),
                    ..Default::default()
                },
            }),
        )
        .await
        .unwrap();
    assert_eq!(beyond["report"]["rows"], json!([]));
    assert_eq!(beyond["report"]["pagination"]["total"], 1);
    assert!(
        ops.execute(
            &root,
            Some(other),
            Command::Services(ServiceCommand::ReadReport {
                id: report_id,
                export: true,
                page: Default::default()
            })
        )
        .await
        .is_err()
    );
    sqlx::query(
        "UPDATE operation_report_snapshots SET expires_at=now()-interval '1 minute' WHERE id=$1",
    )
    .bind(report_id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::ReadReport {
                id: report_id,
                export: true,
                page: Default::default()
            })
        )
        .await
        .is_err()
    );
    sqlx::query(
        "UPDATE operation_metric_events SET created_at=now()-interval '9 days' WHERE school_id=$1",
    )
    .bind(school)
    .execute(&pool)
    .await
    .unwrap();
    ops.0.cleanup_metrics().await.unwrap();
    let retained: i64 = sqlx::query_scalar(
        "SELECT sum(views)::bigint FROM operation_daily_metrics WHERE school_id=$1",
    )
    .bind(school)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retained, 1);
    let slot=ops.execute(&root,None,save("marketing-slots",json!({"revision":0,"name":"学校官方单元","ad_unit_id":"adunit-test","format":"rewarded","reward_scene":"service_theme","qualification":"测试资格材料","active":true}),None)).await.unwrap();
    let slot_id = slot["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    assert!(
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Marketing)
        )
        .await
        .unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    ops.execute(
        &root,
        Some(school),
        Command::Services(ServiceCommand::MarketingSetting {
            id: slot_id,
            revision: 0,
            enabled: true,
        }),
    )
    .await
    .unwrap();
    let marketing = ops
        .execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::Marketing),
        )
        .await
        .unwrap();
    assert_eq!(marketing["items"][0]["id"], slot["id"]);
    assert!(!marketing.to_string().contains("测试资格材料"));
    let marketing_event = json!({"event_id":Uuid::new_v4(),"slot_id":slot_id,"event_type":"client_completed","occurred_at":chrono::Utc::now()});
    for _ in 0..2 {
        ops.execute(
            &root,
            Some(school),
            Command::Services(ServiceCommand::MarketingEvent(
                serde_json::from_value(marketing_event.clone()).unwrap(),
            )),
        )
        .await
        .unwrap();
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM marketing_events")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
