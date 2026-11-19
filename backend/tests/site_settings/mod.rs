use super::*;

fn input(revision: i64, site_name: Option<&str>) -> linjian_api::site::domain::SiteSettingsInput {
    serde_json::from_value(json!({
        "revision": revision,
        "site_name": site_name,
        "title_subtitle": null,
        "login_intro": null,
        "icp_number": null,
        "police_number": null,
        "police_url": null,
        "copyright_owner": null,
        "copyright_start_year": null,
        "copyright_end_mode": null,
        "copyright_end_year": null,
        "copyright_notice": null
    }))
    .unwrap()
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn site_settings_are_public_atomic_and_platform_only() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("site_settings_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());

    let initial = app.site.public().await.unwrap();
    assert_eq!(initial.revision, 0);
    assert_eq!(initial.site_name, None);

    let root = app
        .identity
        .bootstrap("site-root", "站点管理员", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(
            root,
            linjian_api::business::SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();

    let school: Uuid = app.schools.create(&actor, "站点校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let school_admin = Uuid::new_v4();
    sqlx::query("INSERT INTO accounts(id,username,display_name,password_hash,platform_admin) VALUES($1,'site-school-admin','学校管理员','unused',false)")
        .bind(school_admin)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'admin')")
        .bind(school)
        .bind(school_admin)
        .execute(&pool)
        .await
        .unwrap();
    let non_platform: linjian_api::business::Actor = serde_json::from_value(json!({
        "id":school_admin,"username":"site-school-admin","display_name":"学校管理员",
        "platform_admin":false,"bound_school_id":null,
        "grants":[{"school_id":school,"role":"admin"}]
    }))
    .unwrap();
    assert!(app.site.admin(&non_platform).await.is_err());
    assert!(
        app.site
            .update(&non_platform, input(0, Some("不应保存")))
            .await
            .is_err()
    );

    let saved = app
        .site
        .update(&actor, input(0, Some("示例校园")))
        .await
        .unwrap();
    assert_eq!(saved.revision, 1);
    let public = app.site.public().await.unwrap();
    assert_eq!(public.site_name.as_deref(), Some("示例校园"));
    assert_eq!(public.revision, 1);

    let repeated = app
        .site
        .update(&actor, input(1, Some("示例校园")))
        .await
        .unwrap();
    assert_eq!(repeated.revision, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_events WHERE action='site.settings.update'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );

    assert!(
        matches!(
            app.site
                .update(&actor, input(0, Some("旧版本")))
                .await
                .unwrap_err(),
            linjian_api::business::Error::Conflict
        ),
        "stale revisions must not overwrite the current value"
    );

    let cleared = app.site.update(&actor, input(2, None)).await.unwrap();
    assert_eq!(cleared.revision, 3);
    assert_eq!(app.site.public().await.unwrap().site_name, None);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0,
        "site settings must never create source events"
    );

    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
