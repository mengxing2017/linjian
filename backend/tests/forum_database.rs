use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use linjian_api::{bootstrap, database, migration};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;
async fn isolated(admin: &PgPool, name: &str) -> PgPool {
    sqlx::query(&format!("CREATE SCHEMA {name}"))
        .execute(admin)
        .await
        .unwrap();
    let name = name.to_owned();
    PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _| {
            let query = format!("SET search_path TO {name}");
            Box::pin(async move {
                sqlx::query(&query).execute(connection).await?;
                Ok(())
            })
        })
        .connect(&std::env::var("LINJIAN_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap()
}
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL; tools/test_database.py provisions one"]
async fn forum_migration_auth_and_outbox() {
    let url = std::env::var("LINJIAN_TEST_DATABASE_URL")
        .expect("isolated test URL required; no production fallback");
    let admin = PgPool::connect(&url).await.unwrap();
    let source_name = format!("forum_source_{}", Uuid::new_v4().simple());
    let target_name = format!("forum_target_{}", Uuid::new_v4().simple());
    let source = isolated(&admin, &source_name).await;
    let target = isolated(&admin, &target_name).await;
    database::migrate(&source).await.unwrap();
    database::migrate(&source).await.unwrap();
    database::migrate(&target).await.unwrap();
    let app = bootstrap::build_application(source.clone());
    let root = app
        .identity
        .bootstrap("root", "管理员", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(root, "browser", false)
        .await
        .unwrap();
    let school = app.schools.create(&actor, "甲学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let other = app.schools.create(&actor, "乙学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let user = app
        .identity
        .bootstrap("member", "成员", "test-member-password", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, user, "member")
        .await
        .unwrap();
    let (_, member) = app
        .identity
        .0
        .session(user, "miniapp", false)
        .await
        .unwrap();
    assert!(app.schools.members(&member, school).await.is_err());
    assert!(
        app.community
            .posts(&member, other, Default::default())
            .await
            .is_err()
    );
    let post = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts(id,school_id,author_id,title,content) VALUES($1,$2,$3,'标题','正文')",
    )
    .bind(post)
    .bind(school)
    .bind(user)
    .execute(&source)
    .await
    .unwrap();
    let count =
        || sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox").fetch_one(&source);
    assert_eq!(count().await.unwrap(), 0, "pending content is not pushed");
    let mut tx = source.begin().await.unwrap();
    sqlx::query("UPDATE posts SET audit_status='approved' WHERE id=$1")
        .bind(post)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(
        count().await.unwrap(),
        0,
        "rolled-back content has no outbox event"
    );
    sqlx::query("UPDATE posts SET audit_status='approved' WHERE id=$1")
        .bind(post)
        .execute(&source)
        .await
        .unwrap();
    assert_eq!(count().await.unwrap(), 1);
    sqlx::query("UPDATE posts SET browse=browse+1 WHERE id=$1")
        .bind(post)
        .execute(&source)
        .await
        .unwrap();
    assert_eq!(count().await.unwrap(), 1, "browse updates must not forward");
    sqlx::query("UPDATE posts SET content='新正文',revision=revision+1 WHERE id=$1")
        .bind(post)
        .execute(&source)
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET visibility='hidden' WHERE id=$1")
        .bind(post)
        .execute(&source)
        .await
        .unwrap();
    let events: Vec<Value> =
        sqlx::query_scalar("SELECT payload FROM forum_outbox ORDER BY sequence")
            .fetch_all(&source)
            .await
            .unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[2]["sequence"], 3);
    assert_eq!(events[2]["event_type"], "content.withdrawn");
    assert!(events[2]["content"].is_null());
    sqlx::query("UPDATE posts SET visibility='visible' WHERE id=$1")
        .bind(post)
        .execute(&source)
        .await
        .unwrap();
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(school)
        .execute(&source)
        .await
        .unwrap();
    linjian_api::outbox::north::local::Worker::from_env(source.clone())
        .unwrap()
        .unwrap()
        .run_once()
        .await
        .unwrap();
    assert_eq!(
        count().await.unwrap(),
        5,
        "school disable withdraws content"
    );
    // Both media stores, social content and permissions travel in the same verified snapshot.
    sqlx::query("INSERT INTO media_assets(id,school_id,owner_id,mime_type,size_bytes,content) VALUES($1,$2,$3,'image/png',3,$4)").bind(Uuid::new_v4()).bind(school).bind(user).bind(vec![0_u8,1,255]).execute(&source).await.unwrap();
    let snapshot = migration::export(&source).await.unwrap();
    assert!(!snapshot.tables.contains_key("sessions"));
    assert!(!snapshot.tables.contains_key("forum_outbox"));
    let report = migration::import(&target, &snapshot, false).await.unwrap();
    assert_eq!(report["history_enqueued"], false);
    let reexport = migration::export(&target).await.unwrap();
    assert_eq!(snapshot.digest, reexport.digest);
    assert!(
        migration::import(&target, &snapshot, false).await.is_err(),
        "nonempty target must not be overwritten"
    );
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&target)
        .await
        .unwrap();
    assert_eq!(outbox, 0);
    let session_count: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&target)
        .await
        .unwrap();
    assert_eq!(session_count, 0);
    let target_app = bootstrap::build_application(target.clone());
    let (_, new_actor) = target_app
        .identity
        .login(linjian_api::identity::north::local::Login {
            username: "root".into(),
            password: "test-password-at-least-12".into(),
        })
        .await
        .unwrap();
    assert!(new_actor.platform_admin);
    // HTTP session isolation, origin and CSRF are exercised against actual persistence.
    let router = linjian_api::http::app_router(target_app);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/session")
                .header("Cookie", format!("wekeeper_session={}", Uuid::new_v4()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response=router.clone().oneshot(Request::builder().method("POST").uri("/api/v1/auth/sessions").header("Origin","http://localhost:5173").header("Content-Type","application/vnd.api+json").body(Body::from(json!({"data":{"type":"sessions","attributes":{"username":"root","password":"test-password-at-least-12"}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    assert!(cookie.starts_with("linjian_session="));
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap();
    assert_eq!(body["data"]["type"], "sessions");
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/schools")
                .header("Cookie", &cookie)
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/vnd.api+json")
                .body(Body::from(
                    json!({"data":{"type":"schools","attributes":{"name":"跨站请求"}}}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    for path in [
        "/api/v1/platform/execution-devices",
        "/api/v1/schools/x/mobile-execution/shared-tasks",
        "/api/v1/schools/x/broadcast-tasks",
    ] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let tables:Vec<String>=sqlx::query_scalar("SELECT table_name::text FROM information_schema.tables WHERE table_schema=current_schema()").fetch_all(&target).await.unwrap();
    assert!(
        !tables.iter().any(|t| t.starts_with("mobile_")
            || t.starts_with("broadcast_")
            || t == "wecom_groups")
    );
    source.close().await;
    target.close().await;
    sqlx::query(&format!("DROP SCHEMA {source_name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    sqlx::query(&format!("DROP SCHEMA {target_name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn dynamic_source_history_leases_and_school_isolation() {
    use linjian_api::outbox::{
        domain::ReplayInput,
        south::{
            adapter::postgres::Repository,
            port::{OutboxStore, ReceiptResult},
        },
    };
    use linjian_api::source_access::domain::Fault;
    let url = std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required");
    let admin = PgPool::connect(&url).await.unwrap();
    let name = format!("forum_dynamic_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &name).await;
    database::migrate(&pool).await.unwrap();
    sqlx::raw_sql(include_str!("dynamic_source.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let app = bootstrap::build_application(pool.clone());
    let root = app
        .identity
        .bootstrap("root", "管理员", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(root, "browser", false)
        .await
        .unwrap();
    let school = app.schools.create(&actor, "甲校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let other = app.schools.create(&actor, "乙校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let connection = Uuid::new_v4();
    let version = Uuid::new_v4();
    let source = Uuid::new_v4();
    sqlx::query("INSERT INTO source_connections(id,name,source_id,origin,key_id,checked_at) VALUES($1,'test',$2,'https://keeper.example.com','test-key',now())").bind(connection).bind(source).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin) VALUES($1,$2,1,$3,'https://keeper.example.com','https://forum.example.com')").bind(version).bind(connection).bind(source).execute(&pool).await.unwrap();
    sqlx::query("UPDATE source_active_configuration SET connection_id=$1")
        .bind(connection)
        .execute(&pool)
        .await
        .unwrap();
    let bindings = linjian_api::source_access::south::adapter::postgres::Repository(pool.clone());
    bindings
        .bind(
            &actor,
            school,
            connection,
            linjian_api::source_access::north::local::BindingInput {
                route_id: "route-a".into(),
                active: true,
            },
            0,
            1,
        )
        .await
        .unwrap();
    // The same revision can only be saved once, including concurrent attempts.
    let a = bindings.bind(
        &actor,
        school,
        connection,
        linjian_api::source_access::north::local::BindingInput {
            route_id: "route-a".into(),
            active: true,
        },
        1,
        1,
    );
    let b = bindings.bind(
        &actor,
        school,
        connection,
        linjian_api::source_access::north::local::BindingInput {
            route_id: "route-a".into(),
            active: false,
        },
        1,
        1,
    );
    let (a, b) = tokio::join!(a, b);
    assert_ne!(a.is_ok(), b.is_ok());
    // Resume deterministically regardless of which concurrent request won.
    bindings
        .bind(
            &actor,
            school,
            connection,
            linjian_api::source_access::north::local::BindingInput {
                route_id: "route-a".into(),
                active: true,
            },
            2,
            1,
        )
        .await
        .unwrap();
    let post = Uuid::new_v4();
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES($1,$2,$3,'标题','正文','approved')").bind(post).bind(school).bind(root).execute(&pool).await.unwrap();
    let store = Repository(pool.clone());
    let first = store.claim().await.unwrap().unwrap();
    assert!(
        store.claim().await.unwrap().is_none(),
        "a live lease must exclude another worker"
    );
    sqlx::query("UPDATE forum_outbox SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(first.id)
        .execute(&pool)
        .await
        .unwrap();
    let second = store.claim().await.unwrap().unwrap();
    assert_eq!(first.id, second.id);
    assert_ne!(first.token, second.token);
    assert_eq!(first.body, second.body);
    assert_eq!(first.body_sha256, second.body_sha256);
    let receipt = Uuid::new_v4();
    store
        .finish(
            &first,
            Ok(ReceiptResult {
                id: receipt,
                status: "received".into(),
                summary: json!({"sending":null}),
            }),
        )
        .await
        .unwrap();
    let saved: Option<String> =
        sqlx::query_scalar("SELECT receipt_id FROM forum_outbox WHERE id=$1")
            .bind(first.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        saved.is_none(),
        "expired worker must not overwrite a renewed lease"
    );
    // Upstream may already have received the bytes: retry only the original identity.
    store
        .finish(&second, Err((Fault("temporarily_unavailable"), 0)))
        .await
        .unwrap();
    sqlx::query("UPDATE forum_outbox SET next_attempt_at=now() WHERE id=$1")
        .bind(first.id)
        .execute(&pool)
        .await
        .unwrap();
    let third = store.claim().await.unwrap().unwrap();
    assert_eq!(third.id, first.id);
    assert_eq!(third.body, first.body);
    assert!(third.receipt_id.is_none());
    store
        .finish(
            &third,
            Ok(ReceiptResult {
                id: receipt,
                status: "received".into(),
                summary: json!({"sending":null}),
            }),
        )
        .await
        .unwrap();
    sqlx::query("UPDATE forum_outbox SET next_attempt_at=now() WHERE id=$1")
        .bind(first.id)
        .execute(&pool)
        .await
        .unwrap();
    let fourth = store.claim().await.unwrap().unwrap();
    assert_eq!(fourth.receipt_id, Some(receipt));
    assert_eq!(fourth.body, first.body);
    store
        .finish(
            &fourth,
            Ok(ReceiptResult {
                id: receipt,
                status: "applied".into(),
                summary: json!({"sending":null}),
            }),
        )
        .await
        .unwrap();
    assert!(
        store.claim().await.unwrap().is_none(),
        "applied without sending capability must not poll forever"
    );
    assert!(
        store
            .replay(&actor, school, first.id, ReplayInput { reason: " ".into() })
            .await
            .is_err()
    );
    let mut operator = actor.clone();
    operator.platform_admin = false;
    operator.grants = vec![linjian_api::business::Grant {
        school_id: school,
        role: "operator".into(),
    }];
    assert!(matches!(
        store.detail(&operator, other, first.id).await,
        Err(linjian_api::business::Error::NotFound)
    ));
    let detail = store.detail(&operator, school, first.id).await.unwrap();
    assert_eq!(detail["recoverable"], false);
    assert!(detail.get("request_body").is_none());
    // No binding: retain a v2 intent and assign its identity once a verified route exists.
    let unbound_post = Uuid::new_v4();
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES($1,$2,$3,'乙标题','乙正文','approved')").bind(unbound_post).bind(other).bind(root).execute(&pool).await.unwrap();
    let unbound: Uuid = sqlx::query_scalar("SELECT id FROM forum_outbox WHERE post_id=$1")
        .bind(unbound_post)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!store.prepare().await.unwrap());
    bindings
        .bind(
            &actor,
            other,
            connection,
            linjian_api::source_access::north::local::BindingInput {
                route_id: "route-b".into(),
                active: true,
            },
            0,
            1,
        )
        .await
        .unwrap();
    assert!(store.prepare().await.unwrap());
    let bound = store.claim().await.unwrap().unwrap();
    assert_eq!(bound.id, unbound);
    store
        .finish(&bound, Err((Fault("protocol_error"), 0)))
        .await
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT delivery_state FROM forum_outbox WHERE id=$1")
        .bind(unbound)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "retry_wait");
    // Inject an actual enqueue failure; content and the event must roll back together.
    sqlx::raw_sql("CREATE FUNCTION fail_source_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected outbox failure'; END $$; CREATE TRIGGER fail_source_test BEFORE INSERT ON forum_outbox FOR EACH ROW EXECUTE FUNCTION fail_source_test();").execute(&pool).await.unwrap();
    assert!(
        sqlx::query("UPDATE posts SET content='must roll back' WHERE id=$1")
            .bind(post)
            .execute(&pool)
            .await
            .is_err()
    );
    let content: String = sqlx::query_scalar("SELECT content FROM posts WHERE id=$1")
        .bind(post)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(content, "正文");
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
