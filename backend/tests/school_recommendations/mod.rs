use super::*;
use linjian_api::{
    business::SessionKind,
    schools::domain::school_recommendations::{OnboardingQuery, Query, Scene},
};

fn location(scene: Scene) -> Query {
    Query {
        latitude: 23.1,
        longitude: 113.3,
        coordinate_system: "gcj02".into(),
        scene,
    }
}

async fn call(
    router: &axum::Router,
    path: &str,
    input: Value,
    token: Option<&str>,
    origin: bool,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(path)
        .method("POST")
        .header("content-type", "application/vnd.api+json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    if origin {
        request = request.header("origin", "http://localhost:5173");
    }
    let response = router
        .clone()
        .oneshot(
            request
                .body(Body::from(
                    json!({"data":{"type":"school-recommendation-queries","attributes":input}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let status = response.status();
    let document =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    (status, document)
}

async fn counts(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_array((SELECT count(*) FROM school_memberships),(SELECT count(*) FROM audit_events),(SELECT count(*) FROM forum_outbox),(SELECT count(*) FROM miniapp_school_change_requests),(SELECT count(*) FROM accounts WHERE bound_school_id IS NOT NULL))").fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn school_recommendations_are_nearest_five_read_only_and_permission_scoped() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("school_recommendations_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let router = linjian_api::http::app_router(app.clone());
    let user = linjian_api::miniapp_users::north::local::register(
        &linjian_api::miniapp_users::south::adapter::postgres::Repository(pool.clone()),
        "school-recommendations",
        "user",
    )
    .await
    .unwrap();
    let (token, actor) = app
        .identity
        .0
        .session(user, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let before = counts(&pool).await;
    let mut schools = Vec::new();
    for amount in [0, 1, 4, 5, 6, 35] {
        while schools.len() < amount {
            let index = schools.len();
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO schools(id,name,latitude,longitude) VALUES($1,$2,$3,$4)")
                .bind(id)
                .bind(format!("学校{:03}", 100 - index))
                .bind(23.1 + index as f64 * 0.01)
                .bind(113.3)
                .execute(&pool)
                .await
                .unwrap();
            schools.push(id);
        }
        let rows = app
            .schools
            .school_recommendations(&actor, &location(Scene::Binding))
            .await
            .unwrap();
        assert_eq!(rows.len(), amount.min(5));
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.school["id"], schools[index].to_string());
            assert!(row.distance_m.is_finite() && row.distance_m >= 0.0);
            assert!(row.school.get("config").is_none());
            assert!(row.school.get("logo_asset_id").is_none());
        }
        for pair in rows.windows(2) {
            assert!(pair[0].distance_m <= pair[1].distance_m);
        }
    }
    assert_eq!(counts(&pool).await, before);
    let input =
        json!({"latitude":23.1,"longitude":113.3,"coordinate_system":"gcj02","scene":"binding"});
    assert_eq!(
        call(
            &router,
            "/api/v1/school-recommendations",
            input.clone(),
            None,
            false
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, document) = call(
        &router,
        "/api/v1/school-recommendations",
        input.clone(),
        Some(&token),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(document["data"].as_array().unwrap().len(), 5);
    assert_eq!(document["meta"]["limit"], 5);
    assert!(document["meta"].get("latitude").is_none());
    let directory = app.onboarding.schools("学校066", 1).await.unwrap();
    assert_eq!(directory[0]["id"], schools[34].to_string());
    sqlx::query("UPDATE schools SET latitude=23.1 WHERE id=$1")
        .bind(schools[1])
        .execute(&pool)
        .await
        .unwrap();
    let tied = app
        .schools
        .school_recommendations(&actor, &location(Scene::Binding))
        .await
        .unwrap();
    let mut expected = [schools[0], schools[1]];
    expected.sort();
    assert_eq!(tied[0].school["id"], expected[0].to_string());
    assert_eq!(tied[1].school["id"], expected[1].to_string());
    assert!(tied[0].distance_m < 0.001);
    app.schools.bind(&actor, schools[34]).await.unwrap();
    assert!(
        app.schools
            .school_recommendations(&actor, &location(Scene::Binding))
            .await
            .is_err()
    );
    let rows = app
        .schools
        .school_recommendations(&actor, &location(Scene::Browsing))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].school["id"], schools[34].to_string());
    sqlx::query("UPDATE school_memberships SET role='revoked' WHERE account_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.schools
            .school_recommendations(&actor, &location(Scene::Browsing))
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query("UPDATE miniapp_users SET status='banned' WHERE account_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.schools
            .school_recommendations(&actor, &location(Scene::Browsing))
            .await
            .is_err()
    );
    sqlx::query("UPDATE miniapp_users SET status='normal' WHERE account_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM sessions WHERE account_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.schools
            .school_recommendations(&actor, &location(Scene::Browsing))
            .await
            .is_err()
    );

    let attempt = app
        .onboarding
        .start("school-recommendations", "new-user")
        .await
        .unwrap();
    let ticket = attempt["ticket"].as_str().unwrap();
    let query = OnboardingQuery {
        ticket: ticket.into(),
        latitude: 23.1,
        longitude: 113.3,
        coordinate_system: "gcj02".into(),
    };
    let before = counts(&pool).await;
    assert_eq!(
        app.schools
            .onboarding_school_recommendations(&query)
            .await
            .unwrap()
            .len(),
        5
    );
    assert_eq!(counts(&pool).await, before);
    let ticket_input =
        json!({"latitude":23.1,"longitude":113.3,"coordinate_system":"gcj02","ticket":ticket});
    assert_eq!(
        call(
            &router,
            "/api/v1/onboarding/school-recommendations",
            ticket_input.clone(),
            None,
            true
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &router,
            "/api/v1/onboarding/school-recommendations",
            ticket_input,
            None,
            false
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE miniapp_onboarding_attempts SET cancelled=true WHERE token_hash=$1")
        .bind(linjian_api::business::hash(ticket))
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.schools
            .onboarding_school_recommendations(&query)
            .await
            .is_err()
    );
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(schools[0])
        .execute(&pool)
        .await
        .unwrap();
    let another = app
        .onboarding
        .start("school-recommendations", "another-user")
        .await
        .unwrap();
    let query = OnboardingQuery {
        ticket: another["ticket"].as_str().unwrap().into(),
        ..query
    };
    assert!(
        app.schools
            .onboarding_school_recommendations(&query)
            .await
            .unwrap()
            .iter()
            .all(|row| row.school["id"] != schools[0].to_string())
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn school_coordinates_upgrade_preserves_legacy_rows_and_checks_edits() {
    use linjian_api::business::Error;
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    for already_applied in [false, true] {
        let schema = format!("school_coordinates_upgrade_{}", Uuid::new_v4().simple());
        let pool = isolated(&admin, &schema).await;
        let previous = sqlx::migrate::Migrator {
            migrations: std::borrow::Cow::Owned(
                database::MIGRATOR
                    .iter()
                    .filter(|migration| migration.version < if already_applied { 16 } else { 14 })
                    .cloned()
                    .collect(),
            ),
            ..sqlx::migrate::Migrator::DEFAULT
        };
        previous.run(&pool).await.unwrap();
        let school = Uuid::new_v4();
        if already_applied {
            sqlx::query(
                "INSERT INTO schools(id,name,latitude,longitude) VALUES($1,'旧学校',23.1,113.3)",
            )
            .bind(school)
            .execute(&pool)
            .await
            .unwrap();
        } else {
            sqlx::query("INSERT INTO schools(id,name) VALUES($1,'旧学校')")
                .bind(school)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO schools(id,name,active) VALUES(gen_random_uuid(),'隐藏旧学校',false)")
                .execute(&pool).await.unwrap();
            assert!(
                sqlx::raw_sql(include_str!(
                    "../../migrations/0014_school_coordinates_required.sql"
                ))
                .execute(&pool)
                .await
                .is_err()
            );
        }
        database::migrate(&pool).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let recorded: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=14)")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(recorded, already_applied);
        let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
        let root = app
            .identity
            .bootstrap("legacy-root", "平台", "test-password-long-enough", true)
            .await
            .unwrap();
        let (_, actor) = app
            .identity
            .0
            .session(
                root,
                SessionKind::AdminBrowser,
                Some("http://localhost:5173"),
            )
            .await
            .unwrap();
        let detail = app.schools.0.school_detail(&actor, school).await.unwrap();
        assert_eq!(detail.latitude, already_applied.then_some(23.1));
        assert_eq!(detail.longitude, already_applied.then_some(113.3));
        let list = app.schools.0.school_list(&actor).await.unwrap();
        assert_eq!(list.len(), if already_applied { 1 } else { 2 });
        let result = app
            .schools
            .0
            .save_school(
                &actor,
                Some(school),
                json!({"revision":detail.revision,"name":"编辑旧学校"}),
            )
            .await;
        if already_applied {
            assert!(result.is_ok());
        } else {
            assert!(
                matches!(result, Err(Error::Invalid(message)) if message.contains("经纬度必填"))
            );
            assert!(
                sqlx::query("UPDATE schools SET name='直接编辑' WHERE id=$1")
                    .bind(school)
                    .execute(&pool)
                    .await
                    .is_err()
            );
            assert!(
                sqlx::query("UPDATE schools SET name='编辑隐藏学校' WHERE NOT active")
                    .execute(&pool)
                    .await
                    .is_err()
            );
            let unchanged = app.schools.0.school_detail(&actor, school).await.unwrap();
            assert_eq!(unchanged.name, "旧学校");
            assert_eq!(unchanged.revision, detail.revision);
            let saved = app.schools.0.save_school(&actor, Some(school),
                json!({"revision":detail.revision,"name":"编辑旧学校","latitude":23.1,"longitude":113.3}))
                .await.unwrap();
            assert_eq!(saved.latitude, Some(23.1));
            assert_eq!(saved.longitude, Some(113.3));
        }
        assert!(
            sqlx::query(
                "INSERT INTO schools(id,name) VALUES(gen_random_uuid(),'缺少坐标的新学校')"
            )
            .execute(&pool)
            .await
            .is_err()
        );
        if already_applied {
            sqlx::query("UPDATE _sqlx_migrations SET checksum='\\x00'::bytea WHERE version=14")
                .execute(&pool)
                .await
                .unwrap();
            assert!(matches!(
                database::migrate(&pool).await,
                Err(sqlx::migrate::MigrateError::VersionMismatch(14))
            ));
        }
        pool.close().await;
        sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
            .execute(&admin)
            .await
            .unwrap();
    }
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn school_coordinates_are_required_without_breaking_partial_updates() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("school_coordinates_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap(
            "coordinates-root",
            "平台",
            "test-password-long-enough",
            true,
        )
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(
            root,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    for input in [
        json!({"name":"空学校"}),
        json!({"name":"缺经度","latitude":0}),
        json!({"name":"清空","latitude":null,"longitude":null}),
    ] {
        assert!(
            app.schools
                .0
                .save_school(&actor, None, input)
                .await
                .is_err()
        );
    }
    let school = app
        .schools
        .0
        .save_school(
            &actor,
            None,
            json!({"name":"零坐标","latitude":0,"longitude":0}),
        )
        .await
        .unwrap();
    let updated = app
        .schools
        .0
        .save_school(
            &actor,
            Some(school.id),
            json!({"revision":school.revision,"theme_primary":"#abc"}),
        )
        .await
        .unwrap();
    assert_eq!(updated.latitude, Some(0.0));
    assert_eq!(updated.longitude, Some(0.0));
    assert!(
        app.schools
            .0
            .save_school(
                &actor,
                Some(school.id),
                json!({"revision":updated.revision,"latitude":null,"longitude":null})
            )
            .await
            .is_err()
    );
    assert!(
        sqlx::query("INSERT INTO schools(id,name) VALUES(gen_random_uuid(),'禁止空坐标')")
            .execute(&pool)
            .await
            .is_err()
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
