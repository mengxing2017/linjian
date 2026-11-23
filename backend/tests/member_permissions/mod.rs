use super::*;
use linjian_api::{
    business::{Error, SessionKind},
    schools::domain::permissions::{
        Action, CATALOG_VERSION, Effect, MemberQuery, Override, Recommendation, Save,
    },
};

fn configuration(role: &str, revision: &str, overrides: Vec<Override>) -> Save {
    Save {
        catalog_version: CATALOG_VERSION.into(),
        expected_revision: revision.into(),
        role: role.into(),
        overrides,
        reason: "按岗位调整权限".into(),
    }
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn permissions_are_versioned_scoped_atomic_and_live() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated URL required"),
    )
    .await
    .unwrap();
    let schema = format!("permissions_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root_id = app
        .identity
        .bootstrap(
            "permissions-root",
            "平台管理",
            "test-password-at-least-12",
            true,
        )
        .await
        .unwrap();
    let (root_token, root) = app
        .identity
        .0
        .session(
            root_id,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let school: Uuid = app.schools.create(&root, "权限测试校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&root, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let target_id = app
        .identity
        .bootstrap(
            "permissions-operator",
            "运营",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    let manager_id = app
        .identity
        .bootstrap(
            "permissions-admin",
            "学校管理",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    let unknown_id = app
        .identity
        .bootstrap(
            "permissions-unassigned",
            "其他后台账号",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    let catalog = app.schools.permission_catalog(&root, school).await.unwrap();
    let sql_catalog: Value = sqlx::query_scalar("SELECT forum_permission_catalog()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(catalog["permissions"], sql_catalog);
    assert_eq!(catalog["permissions"].as_array().unwrap().len(), 61);
    assert!(!root.has_permission(school, "unknown.permission"));
    let candidate_query = MemberQuery {
        search: Some("permissions-operator".into()),
        ..Default::default()
    };
    let candidates = app
        .schools
        .management_members(&root, school, &candidate_query, true)
        .await
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0]["id"], target_id.to_string());
    let allowed = Override {
        permission_key: "post.recommend".into(),
        effect: Effect::Allow,
    };
    let denied = Override {
        permission_key: "content.post.review".into(),
        effect: Effect::Deny,
    };
    let first = app
        .schools
        .save_authorization(
            &root,
            school,
            target_id,
            configuration("operator", "0", vec![allowed.clone(), denied.clone()]),
        )
        .await
        .unwrap();
    assert_eq!(first["revision"], "1");
    assert!(
        first["effective_permissions"]
            .as_array()
            .unwrap()
            .contains(&json!("post.recommend"))
    );
    assert!(
        !first["effective_permissions"]
            .as_array()
            .unwrap()
            .contains(&json!("content.post.review"))
    );
    let (operator_token, operator) = app
        .identity
        .0
        .session(
            target_id,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    assert!(operator.has_permission(school, "post.recommend"));
    assert!(!operator.has_permission(other, "post.recommend"));
    assert!(!operator.has_permission(school, "content.post.review"));
    assert!(
        app.schools
            .management_members(&operator, school, &MemberQuery::default(), true)
            .await
            .is_err()
    );
    assert!(
        app.schools
            .save_authorization(
                &operator,
                school,
                target_id,
                configuration("admin", "1", vec![])
            )
            .await
            .is_err()
    );
    let before: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM audit_events),(SELECT count(*) FROM forum_outbox)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(matches!(
        app.schools
            .save_authorization(
                &root,
                school,
                target_id,
                configuration("operator", "0", vec![])
            )
            .await,
        Err(Error::Conflict)
    ));
    assert!(
        app.schools
            .save_authorization(
                &root,
                school,
                target_id,
                configuration("operator", "1", vec![allowed.clone(), allowed])
            )
            .await
            .is_err()
    );
    let mut invalid = configuration("operator", "1", vec![]);
    invalid.catalog_version = "obsolete".into();
    assert!(matches!(
        app.schools
            .save_authorization(&root, school, target_id, invalid)
            .await,
        Err(Error::Conflict)
    ));
    let after: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM audit_events),(SELECT count(*) FROM forum_outbox)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after);
    app.schools
        .save_authorization(
            &root,
            school,
            manager_id,
            configuration("admin", "0", vec![]),
        )
        .await
        .unwrap();
    let (manager_token, manager) = app
        .identity
        .0
        .session(
            manager_id,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    assert!(
        app.schools
            .member_permissions(&manager, school, unknown_id)
            .await
            .is_err()
    );
    let recommended = app
        .schools
        .set_recommendation_permission(
            &manager,
            school,
            target_id,
            Recommendation {
                catalog_version: CATALOG_VERSION.into(),
                expected_revision: "1".into(),
                effect: "deny".into(),
                reason: "撤销人工推荐".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(recommended["revision"], "2");
    assert!(
        recommended["overrides"]
            .as_array()
            .unwrap()
            .contains(&json!({"permission_key":"content.post.review","effect":"deny"}))
    );
    assert!(
        app.schools
            .set_recommendation_permission(
                &manager,
                school,
                manager_id,
                Recommendation {
                    catalog_version: CATALOG_VERSION.into(),
                    expected_revision: "1".into(),
                    effect: "allow".into(),
                    reason: "禁止自授".into()
                }
            )
            .await
            .is_err()
    );
    let request = || configuration("operator", "2", vec![]);
    let (one, two) = tokio::join!(
        app.schools
            .save_authorization(&root, school, target_id, request()),
        app.schools
            .save_authorization(&root, school, target_id, request())
    );
    assert!(matches!(
        (&one, &two),
        (Ok(_), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(_))
    ));
    let removed = app
        .schools
        .authorization_action(
            &root,
            school,
            target_id,
            Action {
                catalog_version: CATALOG_VERSION.into(),
                expected_revision: "3".into(),
                action: "remove_management".into(),
                reason: "调离后台岗位".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(removed["role"], "member");
    assert_eq!(removed["revision"], "4");
    assert_eq!(removed["overrides"], json!([]));
    assert_eq!(removed["effective_permissions"], json!([]));
    let refreshed = app
        .identity
        .0
        .authenticate(&operator_token, SessionKind::AdminBrowser)
        .await
        .unwrap();
    assert!(!refreshed.has_permission(school, "content.post.read"));
    let mut transaction = pool.begin().await.unwrap();
    assert!(
        linjian_api::schools::south::adapter::postgres::permissions::authorize(
            &mut transaction,
            &operator,
            school,
            "content.post.read"
        )
        .await
        .is_err()
    );
    transaction.rollback().await.unwrap();
    let restored = app
        .schools
        .save_authorization(
            &root,
            school,
            target_id,
            configuration("operator", "4", vec![]),
        )
        .await
        .unwrap();
    assert_eq!(restored["overrides"], json!([]));
    assert!(
        !restored["effective_permissions"]
            .as_array()
            .unwrap()
            .contains(&json!("post.recommend"))
    );
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox, 0);
    let precise = |allowed: &[&str]| {
        linjian_api::schools::domain::permissions::catalog()
            .iter()
            .filter(|item| item.applicable_roles.iter().any(|role| role == "operator"))
            .map(|item| Override {
                permission_key: item.key.clone(),
                effect: if allowed.contains(&item.key.as_str()) {
                    Effect::Allow
                } else {
                    Effect::Deny
                },
            })
            .collect()
    };
    app.schools
        .save_authorization(
            &root,
            school,
            target_id,
            configuration(
                "operator",
                "5",
                precise(&["school.profile.read", "school.review_settings.update"]),
            ),
        )
        .await
        .unwrap();
    let policy_actor = app
        .identity
        .0
        .authenticate(&operator_token, SessionKind::AdminBrowser)
        .await
        .unwrap();
    let profiles = linjian_api::schools::south::adapter::postgres::Repository(pool.clone());
    let changed = profiles
        .save_school(
            &policy_actor,
            Some(school),
            json!({"revision":1,"config":{"post_audit_mode":"auto"}}),
        )
        .await
        .unwrap();
    assert_eq!(changed.revision, 2);
    assert!(
        profiles
            .save_school(
                &policy_actor,
                Some(school),
                json!({"revision":2,"config":{"post_audit_mode":"manual","unrelated":"forbidden"}})
            )
            .await
            .is_err()
    );
    assert_eq!(
        profiles
            .school_detail(&root, school)
            .await
            .unwrap()
            .revision,
        2
    );
    app.schools
        .save_authorization(
            &root,
            school,
            target_id,
            configuration(
                "operator",
                "6",
                precise(&["operation.banner.read", "operation.banner.create"]),
            ),
        )
        .await
        .unwrap();
    let creator = app
        .identity
        .0
        .authenticate(&operator_token, SessionKind::AdminBrowser)
        .await
        .unwrap();
    let resources = linjian_api::schools::north::resources::Service(profiles.clone());
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    assert!(
        resources
            .upload(
                &creator,
                Some(school),
                png.get_ref().clone(),
                Default::default()
            )
            .await
            .is_err()
    );
    let asset = resources
        .upload_scoped(
            &creator,
            Some(school),
            png.into_inner(),
            Default::default(),
            "operation.banner.create",
        )
        .await
        .unwrap();
    assert_eq!(
        resources
            .list_scoped(
                &creator,
                Some(school),
                Default::default(),
                "operation.banner.read"
            )
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        resources
            .list_scoped(
                &creator,
                Some(school),
                Default::default(),
                "resource.school.read"
            )
            .await
            .is_err()
    );
    use linjian_api::operations::{
        north::local::Service as Operations,
        south::{adapter::postgres::Repository as OperationStore, port::Command},
    };
    let operations = Operations(OperationStore(pool.clone(), Default::default()));
    let operation_input = |revision| {
        serde_json::from_value(json!({"revision":revision,"title":"授权创建的广告","content":"","image_ref":asset["image_ref"],"action":{"kind":"none"},"active":true,"sort_order":0,"starts_at":null,"ends_at":null,"popup_strategy":"daily","popup_interval_days":1})).unwrap()
    };
    let banner = operations
        .execute(
            &creator,
            Some(school),
            Command::Save {
                kind: "banner".into(),
                id: None,
                input: operation_input(0),
            },
        )
        .await
        .unwrap();
    assert!(
        operations
            .execute(
                &creator,
                Some(school),
                Command::Save {
                    kind: "banner".into(),
                    id: Some(banner["id"].as_str().unwrap().parse().unwrap()),
                    input: operation_input(1)
                }
            )
            .await
            .is_err()
    );
    let options = operations
        .execute(
            &creator,
            Some(school),
            Command::PostOptions {
                page: Default::default(),
                context: "operation.banner.create".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(options, json!([]));
    let router = linjian_api::http::app_router(app.clone());
    for path in [
        format!("/api/v1/schools/{school}/audit"),
        format!("/api/v2/schools/{school}/source-bindings"),
    ] {
        for (token, status) in [
            (&root_token, StatusCode::OK),
            (&manager_token, StatusCode::FORBIDDEN),
            (&operator_token, StatusCode::FORBIDDEN),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&path)
                        .header("Cookie", format!("linjian_session={token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status, "{path}");
        }
    }
    let body = json!({"data":{"type":"school-member-authorizations","attributes":{"role":"operator","catalog_version":CATALOG_VERSION,"expected_revision":"7","overrides":[],"reason":"HTTP授权"}}});
    for (origin, csrf, status) in [
        ("https://invalid.example", "wrong", StatusCode::FORBIDDEN),
        ("http://localhost:5173", "wrong", StatusCode::FORBIDDEN),
        ("http://localhost:5173", root.csrf.as_str(), StatusCode::OK),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/api/v1/schools/{school}/members/{target_id}/authorization"
                    ))
                    .header("Cookie", format!("linjian_session={root_token}"))
                    .header("Origin", origin)
                    .header("X-CSRF-Token", csrf)
                    .header("Content-Type", "application/vnd.api+json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let mut revocation = pool.begin().await.unwrap();
    linjian_api::schools::south::adapter::postgres::permissions::fence(
        &mut revocation,
        school,
        true,
    )
    .await
    .unwrap();
    let writer_pool = pool.clone();
    let stale_actor = operator.clone();
    let mut writer = tokio::spawn(async move {
        let mut transaction = writer_pool.begin().await.unwrap();
        linjian_api::schools::south::adapter::postgres::permissions::authorize(
            &mut transaction,
            &stale_actor,
            school,
            "content.post.read",
        )
        .await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut writer)
            .await
            .is_err()
    );
    sqlx::query(
        "UPDATE school_memberships SET role='revoked' WHERE school_id=$1 AND account_id=$2",
    )
    .bind(school)
    .bind(target_id)
    .execute(&mut *revocation)
    .await
    .unwrap();
    sqlx::query("UPDATE school_member_authorizations SET revision=revision+1 WHERE school_id=$1 AND account_id=$2").bind(school).bind(target_id).execute(&mut *revocation).await.unwrap();
    linjian_api::audit::record(
        &mut revocation,
        &root,
        school,
        "school.member.authorization.save",
        target_id,
        json!({"reason":"验证等待请求的授权栅栏"}),
    )
    .await
    .unwrap();
    revocation.commit().await.unwrap();
    assert!(writer.await.unwrap().is_err());
    sqlx::query("DELETE FROM sessions WHERE account_id=$1")
        .bind(root.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.schools
            .save_authorization(
                &root,
                school,
                target_id,
                configuration("operator", "8", vec![])
            )
            .await
            .is_err()
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
