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
