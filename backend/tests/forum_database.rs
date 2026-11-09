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
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn full_text_review_read_only_scope_and_version_fencing() {
    use linjian_api::{business::Page, community::domain::review::digest};
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_review_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone());
    let root = app
        .identity
        .bootstrap("root", "后台显示名", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (token, actor) = app
        .identity
        .0
        .session(root, "browser", false)
        .await
        .unwrap();
    let school: Uuid = app.schools.create(&actor, "全文审核校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "其他校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let content = format!("{}\n<script>正文末尾 😀</script>", "全文".repeat(9980));
    let post = app
        .community
        .post(
            &actor,
            school,
            None,
            serde_json::from_value(
                json!({"title":"完整审核标题","content":content,"images":[],"tags":["校园"]}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let id: Uuid = post["id"].as_str().unwrap().parse().unwrap();
    let before: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT browse FROM posts WHERE id=$1),(SELECT count(*) FROM forum_outbox),(SELECT count(*) FROM forum_publication_snapshots)").bind(id).fetch_one(&pool).await.unwrap();
    let review = app.community.review(&actor, school, id).await.unwrap();
    assert_eq!(review.content, content);
    assert_eq!(review.school_name, "全文审核校");
    assert_eq!(review.author_nickname, "后台显示名");
    assert_eq!(review.review_digest, digest(&post).unwrap());
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'微信注册昵称')")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .review(&actor, school, id)
            .await
            .unwrap()
            .author_nickname,
        "微信注册昵称"
    );
    let after: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT browse FROM posts WHERE id=$1),(SELECT count(*) FROM forum_outbox),(SELECT count(*) FROM forum_publication_snapshots)").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(
        before, after,
        "review reads must have no business side effects"
    );
    let mut bearer = actor.clone();
    bearer.browser = false;
    assert!(app.community.review(&bearer, school, id).await.is_err());
    let mut member = actor.clone();
    member.platform_admin = false;
    member.grants.clear();
    assert!(
        app.community
            .reviews(&member, school, Page::default())
            .await
            .is_err()
    );
    assert!(app.community.review(&actor, other, id).await.is_err());
    let rows = app
        .community
        .reviews(
            &actor,
            school,
            Page {
                search: Some("正文末尾".into()),
                status: Some("pending".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].content, content);
    let router = linjian_api::http::app_router(app.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/schools/{school}/post-reviews?page%5Bsize%5D=1"
                ))
                .header("Cookie", format!("linjian_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let doc: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(doc["data"][0]["type"], "post-reviews");
    assert_eq!(doc["data"][0]["attributes"]["content"], content);
    assert!(doc["data"][0]["attributes"].get("summary").is_none());
    for fingerprint in [None, Some("tampered")] {
        let decision = serde_json::from_value(
            json!({"revision":review.revision,"decision":"approved","review_digest":fingerprint}),
        )
        .unwrap();
        assert!(
            app.community
                .moderate(&actor, school, id, decision, false)
                .await
                .is_err()
        );
    }
    let edited=app.community.post(&actor,school,Some(id),serde_json::from_value(json!({"title":"完整审核标题","content":"变更后的全文","images":[],"tags":[],"revision":review.revision})).unwrap()).await.unwrap();
    let decision=serde_json::from_value(json!({"revision":edited["revision"],"decision":"approved","review_digest":review.review_digest})).unwrap();
    assert!(
        app.community
            .moderate(&actor, school, id, decision, false)
            .await
            .is_err(),
        "old content fingerprint cannot approve an edited post even with the new revision"
    );
    let csrf_response=router.clone().oneshot(Request::builder().method("POST").uri(format!("/api/v1/schools/{school}/posts/{id}/moderation-decisions")).header("Cookie",format!("linjian_session={token}")).header("Content-Type","application/vnd.api+json").body(Body::from(json!({"data":{"type":"posts","attributes":{"revision":edited["revision"],"decision":"approved","review_digest":digest(&edited).unwrap()}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(csrf_response.status(), StatusCode::FORBIDDEN);
    let decision=serde_json::from_value(json!({"revision":edited["revision"],"decision":"approved","reason":"全文核验通过","review_digest":digest(&edited).unwrap()})).unwrap();
    app.community
        .moderate(&actor, school, id, decision, false)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .review(&actor, school, id)
            .await
            .unwrap()
            .audit_status,
        "approved"
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/schools/{school}/post-reviews"))
                .header("Cookie", format!("linjian_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let doc: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert!(
        doc["data"].as_array().unwrap().is_empty(),
        "HTTP review list defaults to pending"
    );
    let current = app.schools.0.school_detail(&actor, school).await.unwrap();
    app.schools
        .update(
            &actor,
            school,
            json!({"status":"hidden","revision":current.revision}),
        )
        .await
        .unwrap();
    assert!(
        app.community.review(&actor, school, id).await.is_err(),
        "even platform administrators cannot review a hidden community"
    );
    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/schools/{school}/posts/{id}/review"))
                .header("Cookie", format!("linjian_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
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
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'测试小程序成员')")
        .bind(user)
        .execute(&source)
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

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn reviewed_miniapp_publication_freezing_and_fencing() {
    use linjian_api::content_publication::{
        domain::{Failure, Link, Policy},
        south::{adapter::postgres::Repository, port::PreparationStore},
    };
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("forum_publication_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &name).await;
    database::migrate(&pool).await.unwrap();
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
    let school: Uuid = app.schools.create(&actor, "发布测试校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'测试小程序成员')")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, root, "member")
        .await
        .unwrap();
    let (_, mini) = app
        .identity
        .0
        .session(root, "miniapp", false)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE schools SET config=config||'{\"post_audit_mode\":\"manual\"}'::jsonb WHERE id=$1",
    )
    .bind(school)
    .execute(&pool)
    .await
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
    linjian_api::source_access::south::adapter::postgres::Repository(pool.clone())
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
    let create = || {
        serde_json::from_value(
            json!({"title":"审核标题","content":" 第一行\n 第二行 😀 ","images":[],"tags":[]}),
        )
        .unwrap()
    };
    assert!(
        serde_json::from_value::<linjian_api::community::domain::PostInput>(
            json!({"title":"伪造","content":"正文","source_projection":"legacy_full_v2"})
        )
        .is_err()
    );
    let pending = app
        .community
        .post(&mini, school, None, create())
        .await
        .unwrap();
    let post: Uuid = pending["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        pending["audit_status"], "pending",
        "manual policy must require human review"
    );
    assert_eq!(pending["created_via"], "miniapp");
    let store = Repository(pool.clone());
    assert!(app.community.review(&mini, school, post).await.is_err());
    let stale =
        json!({"decision":"approved","reason":"人工审核","revision":1,"review_digest":"stale"});
    assert!(
        app.community
            .moderate(
                &actor,
                school,
                post,
                serde_json::from_value(stale).unwrap(),
                false
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_publication_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let decision = |row: &Value| {
        serde_json::from_value(json!({"decision":"approved","reason":"人工审核","revision":row["revision"],"review_digest":linjian_api::community::domain::review::digest(row).unwrap()})).unwrap()
    };
    assert!(
        app.community
            .moderate(&mini, school, post, decision(&pending), false)
            .await
            .is_err()
    );
    let approved = app
        .community
        .moderate(&actor, school, post, decision(&pending), false)
        .await
        .unwrap();
    let event: Uuid = sqlx::query_scalar("SELECT id FROM forum_outbox WHERE post_id=$1")
        .bind(post)
        .fetch_one(&pool)
        .await
        .unwrap();
    let intent: Value = sqlx::query_scalar("SELECT to_jsonb(o) FROM forum_outbox o WHERE id=$1")
        .bind(event)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(intent["delivery_state"], "preparing_content");
    assert!(intent["request_body"].is_null());
    assert_eq!(intent["payload"]["content"], "第一行 第二行 😀");
    let delivery_store = linjian_api::outbox::south::adapter::postgres::Repository(pool.clone());
    let detail = delivery_store.detail(&actor, school, event).await.unwrap();
    assert_eq!(detail["preparation"]["state"], "pending");
    assert_eq!(detail["publication"]["link_type"], "short_link");
    assert!(detail["publication"]["link"].is_null());
    app.community
        .moderate(&actor, school, post, decision(&approved), false)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_publication_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1,
        "repeat approval is idempotent"
    );
    let policy = Policy::parse(
        "test-app".into(),
        "short_link".into(),
        "false",
        "30",
        "86400",
    )
    .unwrap();
    sqlx::query("UPDATE forum_publication_preparations SET resolved_policy=$2 WHERE event_id=$1")
        .bind(event)
        .bind(json!(policy))
        .execute(&pool)
        .await
        .unwrap();
    let work = store.claim().await.unwrap().unwrap();
    assert!(
        store.claim().await.unwrap().is_none(),
        "one live preparation lease"
    );
    store
        .finish(&work, Err(Failure::blocked("permission_denied")))
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT request_body IS NULL FROM forum_outbox WHERE id=$1")
            .bind(event)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM forum_publication_preparations WHERE event_id=$1")
            .bind(event)
            .fetch_one(&pool)
            .await
            .unwrap();
    store
        .retry(
            &actor,
            school,
            event,
            linjian_api::content_publication::domain::Retry {
                reason: "权限已开通".into(),
                revision,
            },
        )
        .await
        .unwrap();
    let expired = store.claim().await.unwrap().unwrap();
    sqlx::query("UPDATE forum_publication_preparations SET lease_until=now()-interval '1 second' WHERE event_id=$1").bind(event).execute(&pool).await.unwrap();
    let work = store.claim().await.unwrap().unwrap();
    assert_ne!(expired.token, work.token);
    let link = Link {
        value: "#小程序://Linjian/发布测试".into(),
        generated_at: chrono::Utc::now(),
        expires_at: Some(chrono::Utc::now() + chrono::Duration::days(30)),
    };
    store.finish(&expired, Ok(link.clone())).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT request_body IS NULL FROM forum_outbox WHERE id=$1")
            .bind(event)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "an expired lease cannot freeze bytes after reclamation"
    );
    store.finish(&work, Ok(link.clone())).await.unwrap();
    let frozen: String = sqlx::query_scalar("SELECT request_body FROM forum_outbox WHERE id=$1")
        .bind(event)
        .fetch_one(&pool)
        .await
        .unwrap();
    let body: Value = serde_json::from_str(&frozen).unwrap();
    assert_eq!(
        body["data"]["attributes"]["content"],
        "第一行 第二行 😀\n查看详情：#小程序://Linjian/发布测试"
    );
    assert_eq!(
        body["data"]["attributes"]["detail_url"],
        format!("https://forum.example.com/community/{school}/posts/{post}")
    );
    store
        .finish(
            &work,
            Ok(Link {
                value: "#小程序://changed".into(),
                ..link.clone()
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT request_body FROM forum_outbox WHERE id=$1")
            .bind(event)
            .fetch_one(&pool)
            .await
            .unwrap(),
        frozen,
        "late results cannot replace frozen bytes"
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    app.community.detail(&mini, school, post).await.unwrap();
    app.community
        .react(&mini, school, post, "like", true)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before,
        "interactions do not push"
    );
    let edited = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            serde_json::from_value(
                json!({"title":"审核标题","content":"新版本正文","revision":approved["revision"]}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(edited["audit_status"], "pending");
    assert!(edited["approved_snapshot_id"].is_null());
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM forum_outbox WHERE post_id=$1 AND payload->>'event_type'='content.withdrawn' AND request_body IS NOT NULL)").bind(post).fetch_one(&pool).await.unwrap());
    let approved2 = app
        .community
        .moderate(&actor, school, post, decision(&edited), false)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE forum_publication_preparations SET resolved_policy=$1 WHERE state='pending'",
    )
    .bind(json!(policy))
    .execute(&pool)
    .await
    .unwrap();
    let late = store.claim().await.unwrap().unwrap();
    app.community.post(&mini,school,Some(post),serde_json::from_value(json!({"title":"再次修改","content":"编辑发生在链接生成期间","revision":approved2["revision"]})).unwrap()).await.unwrap();
    store.finish(&late, Ok(link.clone())).await.unwrap();
    let obsolete: Value = sqlx::query_scalar("SELECT to_jsonb(o) FROM forum_outbox o WHERE id=$1")
        .bind(late.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(obsolete["delivery_state"], "superseded");
    assert!(obsolete["request_body"].is_null());
    // Stopping the school fences an in-progress generation even before reconciliation runs.
    let row: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(post)
        .fetch_one(&pool)
        .await
        .unwrap();
    app.community
        .moderate(&actor, school, post, decision(&row), false)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE forum_publication_preparations SET resolved_policy=$1 WHERE state='pending'",
    )
    .bind(json!(policy))
    .execute(&pool)
    .await
    .unwrap();
    let stopped = store.claim().await.unwrap().unwrap();
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    store.finish(&stopped, Ok(link)).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT request_body IS NULL FROM forum_outbox WHERE id=$1")
            .bind(stopped.id)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    sqlx::query("SELECT forum_reconcile_school($1)")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM forum_source_route_states WHERE school_id=$1 AND last_event_type='content.upsert')").bind(school).fetch_one(&pool).await.unwrap());
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn miniapp_users_registration_school_scope_and_ban() {
    use linjian_api::miniapp_users::{
        domain::*,
        north::local::register,
        south::{adapter::postgres::Repository, port::ProfileStore},
    };
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("miniapp_users_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &name).await;
    database::migrate(&pool).await.unwrap();
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
    let school: Uuid = app.schools.create(&actor, "甲校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "乙校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let repo = Repository(pool.clone());
    let (one, two) = tokio::join!(
        register(&repo, "app-a", "openid-a"),
        register(&repo, "app-a", "openid-a")
    );
    let id = one.unwrap();
    assert_eq!(id, two.unwrap());
    let different = register(&repo, "app-b", "openid-a").await.unwrap();
    assert_ne!(id, different);
    assert_eq!(
        repo.list(&actor, None, &Query::default())
            .await
            .unwrap()
            .len(),
        2,
        "backend accounts are not miniapp users"
    );
    assert_eq!(
        app.identity.list(&actor).await.unwrap().len(),
        1,
        "pure miniapp users are not backend login accounts"
    );
    assert!(
        app.identity
            .reset(&actor, id, "test-reset-password-long")
            .await
            .is_err(),
        "cannot provision a backend password for a pure miniapp user"
    );
    let (token, mini) = app.identity.0.session(id, "miniapp", false).await.unwrap();
    assert!(!mini.platform_admin);
    assert!(mini.platform().is_err());
    app.schools.bind(&mini, school).await.unwrap();
    let mini = app.identity.authenticate(&token, "miniapp").await.unwrap();
    let own = repo.own(id).await.unwrap();
    assert_eq!(own.school_id, Some(school));
    assert_eq!(own.status, "normal");
    assert_eq!(own.post_count, 0);
    assert!(repo.detail(&actor, Some(other), id).await.is_err());
    assert!(
        repo.list(&mini, Some(school), &Query::default())
            .await
            .is_err()
    );
    let profile = repo
        .profile(
            &mini,
            ProfileInput {
                nickname: "小程序昵称".into(),
                avatar_asset_id: None,
                revision: own.revision,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT display_name FROM accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "微信用户",
        "miniapp nickname is independent"
    );
    let profile = repo
        .mobile(
            &mini,
            id,
            MobileInput {
                mobile: "13800138000".into(),
                revision: profile.revision,
                reason: None,
            },
            false,
        )
        .await
        .unwrap();
    assert!(
        repo.mobile(
            &mini,
            id,
            MobileInput {
                mobile: "13800138001".into(),
                revision: profile.revision,
                reason: None
            },
            false
        )
        .await
        .is_err()
    );
    let again = repo
        .mobile(
            &mini,
            id,
            MobileInput {
                mobile: "13800138000".into(),
                revision: profile.revision - 1,
                reason: None,
            },
            false,
        )
        .await
        .unwrap();
    assert_eq!(again.revision, profile.revision);
    let mut image = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    let png = image.into_inner();
    let asset = repo.upload(&mini, png.clone()).await.unwrap();
    let profile = repo
        .profile(
            &mini,
            ProfileInput {
                nickname: "小程序昵称".into(),
                avatar_asset_id: Some(asset),
                revision: profile.revision,
            },
        )
        .await
        .unwrap();
    let (_, outsider) = app
        .identity
        .0
        .session(different, "miniapp", false)
        .await
        .unwrap();
    assert!(repo.avatar(&outsider, asset).await.is_err());
    assert!(
        repo.profile(
            &outsider,
            ProfileInput {
                nickname: "偷头像".into(),
                avatar_asset_id: Some(asset),
                revision: 1
            }
        )
        .await
        .is_err()
    );
    assert!(repo.avatar(&actor, asset).await.is_ok());
    for (title, status, visibility) in [
        ("待审", "pending", "visible"),
        ("通过", "approved", "visible"),
        ("拒绝", "rejected", "hidden"),
        ("删除", "approved", "deleted"),
    ] {
        sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content,audit_status,visibility) VALUES($1,$2,$3,$4,'统计正文',$5,$6)").bind(Uuid::new_v4()).bind(school).bind(id).bind(title).bind(status).bind(visibility).execute(&pool).await.unwrap();
    }
    let count = repo.own(id).await.unwrap();
    assert_eq!(count.post_count, 3);
    let manager_id = app
        .identity
        .bootstrap(
            "school-admin",
            "学校管理员",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, manager_id, "admin")
        .await
        .unwrap();
    let (_, manager) = app
        .identity
        .0
        .session(manager_id, "browser", false)
        .await
        .unwrap();
    let sorted = repo
        .list(
            &manager,
            Some(school),
            &Query {
                sort: Some("-post_count".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(sorted[0].post_count, 3);
    let visible:Uuid=sqlx::query_scalar("SELECT id FROM posts WHERE author_id=$1 AND audit_status='approved' AND visibility='visible' LIMIT 1").bind(id).fetch_one(&pool).await.unwrap();
    let comment = app
        .community
        .comment(
            &mini,
            school,
            visible,
            serde_json::from_value(json!({"content":"论坛评论回归","parent_id":null})).unwrap(),
        )
        .await
        .unwrap();
    app.community
        .delete_comment(
            &mini,
            school,
            comment["id"].as_str().unwrap().parse().unwrap(),
            1,
        )
        .await
        .unwrap();
    app.community
        .follow(&mini, school, manager_id, true)
        .await
        .unwrap();
    app.community
        .follow(&mini, school, manager_id, false)
        .await
        .unwrap();
    app.community
        .case(
            &mini,
            school,
            json!({"kind":"report","post_id":visible,"content":"举报回归"}),
        )
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    repo.status(
        &manager,
        Some(school),
        id,
        StatusInput {
            status: "banned".into(),
            reason: "违反学校规则".into(),
            revision: profile.revision,
        },
    )
    .await
    .unwrap();
    assert!(app.identity.authenticate(&token, "miniapp").await.is_err());
    assert!(matches!(
        app.identity.0.session(id, "miniapp", false).await,
        Err(linjian_api::business::Error::MiniappBanned)
    ));
    assert_eq!(
        register(&repo, "app-a", "openid-a").await.unwrap(),
        id,
        "banned registration cannot create a replacement user"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before,
        "user state does not push content"
    );
    assert!(
        matches!(
            app.community.follow(&mini, school, manager_id, true).await,
            Err(linjian_api::business::Error::MiniappBanned)
        ),
        "a previously authenticated actor cannot write after a ban"
    );
    let banned = repo.own(id).await.unwrap();
    repo.school(
        &actor,
        id,
        SchoolInput {
            school_id: other,
            reason: "纠正归属".into(),
            revision: banned.revision,
        },
    )
    .await
    .unwrap();
    assert!(repo.detail(&manager, Some(school), id).await.is_err());
    let moved = repo.own(id).await.unwrap();
    assert_eq!(moved.status, "banned");
    assert_eq!(moved.post_count, 0);
    repo.status(
        &actor,
        None,
        id,
        StatusInput {
            status: "normal".into(),
            reason: "验证旧会话".into(),
            revision: moved.revision,
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(
            repo.upload(&mini, png.clone()).await,
            Err(linjian_api::business::Error::Unauthorized)
        ),
        "a revoked actor cannot upload after unban"
    );
    let moved = repo.own(id).await.unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM posts WHERE author_id=$1 AND school_id=$2"
        )
        .bind(id)
        .bind(school)
        .fetch_one(&pool)
        .await
        .unwrap(),
        4,
        "moving user must not move historical posts"
    );
    repo.status(
        &actor,
        None,
        id,
        StatusInput {
            status: "normal".into(),
            reason: "复核解封".into(),
            revision: moved.revision,
        },
    )
    .await
    .unwrap();
    assert!(app.identity.authenticate(&token, "miniapp").await.is_err());
    let (fresh, _) = app.identity.0.session(id, "miniapp", false).await.unwrap();
    assert!(app.identity.authenticate(&fresh, "miniapp").await.is_ok());
    // A dual identity retains browser privileges while miniapp never inherits them.
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'管理员的小程序身份')")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    app.schools
        .grant(&actor, other, root, "admin")
        .await
        .unwrap();
    let (root_token, root_mini) = app
        .identity
        .0
        .session(root, "miniapp", false)
        .await
        .unwrap();
    assert!(!root_mini.can(other, "admin"));
    assert!(app.identity.list(&root_mini).await.is_err());
    let (browser_token, root_browser) = app
        .identity
        .0
        .session(root, "browser", false)
        .await
        .unwrap();
    let race = repo.own(root).await.unwrap();
    let issue = app.identity.0.session(root, "miniapp", false);
    let ban = repo.status(
        &actor,
        None,
        root,
        StatusInput {
            status: "banned".into(),
            reason: "并发封禁".into(),
            revision: race.revision,
        },
    );
    let (issued, result) = tokio::join!(issue, ban);
    result.unwrap();
    if let Ok((raced, _)) = issued {
        assert!(app.identity.authenticate(&raced, "miniapp").await.is_err())
    }
    assert!(
        app.identity
            .authenticate(&root_token, "miniapp")
            .await
            .is_err()
    );
    assert!(
        app.identity
            .authenticate(&browser_token, "browser")
            .await
            .is_ok(),
        "miniapp ban preserves browser sessions"
    );
    assert!(root_browser.platform().is_ok());
    let audits: Vec<Value> =
        sqlx::query_scalar("SELECT detail FROM audit_events WHERE action='miniapp_user.mobile'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        !serde_json::to_string(&audits)
            .unwrap()
            .contains("13800138000")
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn school_profiles_visibility_artwork_and_transactions() {
    use linjian_api::{
        business::{Error, Page},
        schools::{domain::profile::SchoolQuery, south::adapter::postgres::Repository},
    };
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("school_profiles_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone());
    let root = app
        .identity
        .bootstrap("root", "管理员", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (token, actor) = app
        .identity
        .0
        .session(root, "browser", false)
        .await
        .unwrap();
    let resource_operations =
        linjian_api::schools::north::resources::Service(Repository(pool.clone()));
    let repo = Repository(pool.clone());
    let school=repo.save_school(&actor,None,json!({"name":"示例大学","short_name":"示大","province":"广东省","city":"广州市","area":"天河区","latitude":0,"longitude":0,"theme_primary":"#abc"})).await.unwrap();
    let id = school.id;
    assert_eq!(school.theme_primary, "#AABBCC");
    assert_eq!(school.status, "normal");
    assert_eq!(school.latitude, Some(0.0));
    let other = repo
        .save_school(&actor, None, json!({"name":"另一学校"}))
        .await
        .unwrap();
    for invalid in [
        json!({"name":"校","latitude":1}),
        json!({"name":"校","longitude":180.1,"latitude":0}),
        json!({"name":"校","active":true}),
        json!({"name":"校","theme_primary":"var(--color)"}),
    ] {
        assert!(repo.save_school(&actor, None, invalid).await.is_err());
    }
    let manager_id = app
        .identity
        .bootstrap("manager", "校管", "test-password-at-least-12", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, id, manager_id, "admin")
        .await
        .unwrap();
    let (_, manager) = app
        .identity
        .0
        .session(manager_id, "browser", false)
        .await
        .unwrap();
    let operator_id = app
        .identity
        .bootstrap("operator", "操作员", "test-password-at-least-12", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, id, operator_id, "operator")
        .await
        .unwrap();
    let (_, operator) = app
        .identity
        .0
        .session(operator_id, "browser", false)
        .await
        .unwrap();
    assert!(
        repo.save_school(&operator, Some(id), json!({"name":"越权","revision":1}))
            .await
            .is_err()
    );
    assert!(
        repo.save_school(&manager, Some(id), json!({"status":"hidden","revision":1}))
            .await
            .is_err()
    );
    assert!(
        repo.save_school(
            &manager,
            Some(other.id),
            json!({"name":"跨校","revision":1})
        )
        .await
        .is_err()
    );
    let mini_id = linjian_api::miniapp_users::north::local::register(
        &linjian_api::miniapp_users::south::adapter::postgres::Repository(pool.clone()),
        "school-app",
        "school-openid",
    )
    .await
    .unwrap();
    let (mini_token, mini) = app
        .identity
        .0
        .session(mini_id, "miniapp", false)
        .await
        .unwrap();
    let directory = repo
        .search_schools(
            &mini,
            &SchoolQuery {
                search: Some("示大".into()),
                ..Default::default()
            },
            false,
        )
        .await
        .unwrap();
    assert_eq!(directory.len(), 1);
    assert!(directory[0].get("config").is_none());
    assert!(directory[0].get("logo_asset_id").is_none());
    let mut image = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    let bytes = image.into_inner();
    let image = resource_operations
        .upload(&actor, Some(id), bytes.clone())
        .await
        .unwrap();
    let asset: Uuid = image["id"].as_str().unwrap().parse().unwrap();
    assert!(
        repo.save_school(
            &actor,
            Some(other.id),
            json!({"logo_asset_id":asset,"revision":1})
        )
        .await
        .is_err()
    );
    let school=repo.save_school(&manager,Some(id),json!({"logo_asset_id":asset,"header_bg_asset_id":asset,"revision":1,"config":{"post_audit_mode":"manual"}})).await.unwrap();
    assert_eq!(school.short_name, "示大");
    assert_eq!(school.revision, 2);
    assert_eq!(repo.branding(id, "logo").await.unwrap().1, bytes);
    assert!(matches!(
        repo.save_school(&manager, Some(id), json!({"name":"旧修订","revision":1}))
            .await,
        Err(Error::Conflict)
    ));
    sqlx::query("UPDATE operational_media SET created_at=now()-interval '8 days' WHERE id=$1")
        .bind(asset)
        .execute(&pool)
        .await
        .unwrap();
    repo.cleanup_artwork().await.unwrap();
    assert!(repo.branding(id, "header").await.is_ok());
    app.schools.bind(&mini, id).await.unwrap();
    let mini = app
        .identity
        .authenticate(&mini_token, "miniapp")
        .await
        .unwrap();
    // Establish a real source route and already published content, without contacting an upstream.
    let c = Uuid::new_v4();
    let cv = Uuid::new_v4();
    let source = Uuid::new_v4();
    sqlx::query("INSERT INTO source_connections(id,name,source_id,origin,key_id,checked_at) VALUES($1,'test',$2,'https://keeper.example.com','test-key',now())").bind(c).bind(source).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin) VALUES($1,$2,1,$3,'https://keeper.example.com','https://forum.example.com')").bind(cv).bind(c).bind(source).execute(&pool).await.unwrap();
    sqlx::query("UPDATE source_active_configuration SET connection_id=$1")
        .bind(c)
        .execute(&pool)
        .await
        .unwrap();
    linjian_api::source_access::south::adapter::postgres::Repository(pool.clone())
        .bind(
            &actor,
            id,
            c,
            linjian_api::source_access::north::local::BindingInput {
                route_id: "school-route".into(),
                active: true,
            },
            0,
            1,
        )
        .await
        .unwrap();
    let post = Uuid::new_v4();
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES($1,$2,$3,'标题','正文','approved')").bind(post).bind(id).bind(root).execute(&pool).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let school = repo
        .save_school(
            &manager,
            Some(id),
            json!({"name":"学校新名称","theme_secondary":"#123456","revision":2}),
        )
        .await
        .unwrap();
    assert_eq!(school.logo_asset_id, Some(asset));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        count
    );
    // Serialize school hiding against a content write; neither path may deadlock or publish after hiding.
    let hidden = repo.save_school(
        &actor,
        Some(id),
        json!({"status":"hidden","revision":school.revision}),
    );
    let edit = app.community.post(
        &actor,
        id,
        Some(post),
        serde_json::from_value(
            json!({"title":"编辑标题","content":"编辑正文","images":[],"tags":[],"revision":1}),
        )
        .unwrap(),
    );
    let (hidden, edit) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(hidden, edit)
    })
    .await
    .unwrap();
    let hidden = hidden.unwrap();
    assert_eq!(hidden.status, "hidden");
    let _ = edit;
    assert!(
        app.identity
            .authenticate(&mini_token, "miniapp")
            .await
            .is_ok(),
        "hidden school preserves user login"
    );
    assert!(
        app.community
            .posts(&mini, id, Page::default())
            .await
            .is_err()
    );
    assert!(
        app.community
            .posts(&actor, id, Page::default())
            .await
            .is_err()
    );
    assert!(repo.branding(id, "logo").await.is_err());
    assert!(
        repo.search_schools(
            &mini,
            &SchoolQuery {
                search: Some("示大".into()),
                ..Default::default()
            },
            false
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert!(app.schools.bind(&mini, id).await.is_err());
    assert!(repo.school_detail(&manager, id).await.is_err());
    assert!(repo.school_detail(&actor, id).await.is_ok());
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT bound_school_id FROM accounts WHERE id=$1")
            .bind(mini_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        id
    );
    sqlx::query("SELECT forum_reconcile_school($1,50)")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM forum_outbox WHERE school_id=$1 AND payload->>'event_type'='content.withdrawn')").bind(id).fetch_one(&pool).await.unwrap());
    // Platform can maintain hidden branding and restore the school, even without active school grants.
    resource_operations
        .upload(&actor, Some(id), bytes)
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let restored = repo
        .save_school(
            &actor,
            Some(id),
            json!({"status":"normal","revision":hidden.revision}),
        )
        .await
        .unwrap();
    assert_eq!(restored.status, "normal");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    // Real HTTP routing, JSON:API pagination and browser CSRF remain enforced.
    let router = linjian_api::http::app_router(app.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/school-directory?page%5Bsize%5D=1")
                .header("Authorization", format!("Bearer {mini_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let doc: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(doc["data"].as_array().unwrap().len(), 1);
    assert_eq!(doc["meta"]["page"]["has_more"], true);
    let response=router.clone().oneshot(Request::builder().method("PUT").uri(format!("/api/v1/schools/{id}")).header("Cookie",format!("linjian_session={token}")).header("Content-Type","application/vnd.api+json").body(Body::from(json!({"data":{"type":"schools","attributes":{"name":"CSRF","revision":restored.revision}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn moderation_modes_preserve_policy_evidence_and_publication_boundaries() {
    use linjian_api::{business::Page, community::domain::review::digest};
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_modes_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
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
    let school: Uuid = app.schools.create(&actor, "自动审核校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'作者')")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, root, "member")
        .await
        .unwrap();
    let (_, mini) = app
        .identity
        .0
        .session(root, "miniapp", false)
        .await
        .unwrap();
    let operator_id = app
        .identity
        .bootstrap("operator", "运营", "test-password-at-least-12", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, operator_id, "operator")
        .await
        .unwrap();
    let (_, operator) = app
        .identity
        .0
        .session(operator_id, "browser", false)
        .await
        .unwrap();
    let viewer_id = app
        .identity
        .bootstrap("viewer", "普通成员", "test-password-at-least-12", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, viewer_id, "member")
        .await
        .unwrap();
    let (_, viewer) = app
        .identity
        .0
        .session(viewer_id, "browser", false)
        .await
        .unwrap();
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    for forbidden in [&operator, &viewer, &mini] {
        assert!(
            app.schools
                .update(
                    forbidden,
                    school,
                    json!({"revision":profile.revision,"config":{"post_audit_mode":"auto"}})
                )
                .await
                .is_err()
        );
    }
    let input = |title: &str, content: &str, revision: Value| {
        serde_json::from_value(json!({"title":title,"content":content,"revision":revision}))
            .unwrap()
    };
    let pending = app
        .community
        .post(
            &mini,
            school,
            None,
            input("人工待审", "待审正文", Value::Null),
        )
        .await
        .unwrap();
    assert_eq!(pending["audit_status"], "pending");
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    app.schools.update(&actor,school,json!({"revision":profile.revision,"config":{"post_audit_mode":"auto","comment_audit_mode":"manual"}})).await.unwrap();
    let id: Uuid = pending["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        app.community
            .review(&actor, school, id)
            .await
            .unwrap()
            .audit_status,
        "pending"
    );
    let approved = app
        .community
        .post(
            &mini,
            school,
            None,
            input("自动标题", "自动全文", Value::Null),
        )
        .await
        .unwrap();
    let post: Uuid = approved["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(approved["audit_status"], "approved");
    assert_eq!(approved["created_via"], "miniapp");
    assert_eq!(approved["source_projection"], "miniapp_summary_v1");
    let evidence: Value =
        sqlx::query_scalar("SELECT to_jsonb(s) FROM forum_publication_snapshots s WHERE id=$1")
            .bind(
                approved["approved_snapshot_id"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
            )
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(evidence["decision_source"], "automatic");
    assert!(evidence["moderator_id"].is_null());
    let review = app.community.review(&actor, school, post).await.unwrap();
    assert_eq!(json!(review.decision_source), "automatic");
    assert!(review.decided_at.is_some());
    assert!(review.moderator_id.is_none());
    assert!(app.community.review(&actor, other, post).await.is_err());
    assert!(app.community.review(&mini, school, post).await.is_err());
    assert!(
        app.community
            .reviews(&operator, school, Page::default())
            .await
            .is_ok()
    );
    assert!(
        app.community
            .reviews(&viewer, school, Page::default())
            .await
            .is_err()
    );
    let before: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM moderation_events),(SELECT count(*) FROM forum_outbox)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let unchanged = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            input("自动标题", "自动全文", approved["revision"].clone()),
        )
        .await
        .unwrap();
    assert_eq!(unchanged["revision"], approved["revision"]);
    assert_eq!(
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT (SELECT count(*) FROM moderation_events),(SELECT count(*) FROM forum_outbox)"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        before
    );
    let edited = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            input("自动标题", "修改后的全文", approved["revision"].clone()),
        )
        .await
        .unwrap();
    assert_eq!(edited["audit_status"], "approved");
    assert_ne!(
        edited["approved_snapshot_id"],
        approved["approved_snapshot_id"]
    );
    assert_eq!(
        edited["content_revision"].as_i64().unwrap(),
        approved["content_revision"].as_i64().unwrap() + 1
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT delivery_state='superseded' FROM forum_outbox WHERE publication_snapshot_id=$1"
        )
        .bind(
            approved["approved_snapshot_id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    let rejected = app
        .community
        .moderate(
            &actor,
            school,
            post,
            serde_json::from_value(
                json!({"revision":edited["revision"],"decision":"rejected","reason":"人工拒绝"}),
            )
            .unwrap(),
            false,
        )
        .await
        .unwrap();
    let same = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            input("自动标题", "修改后的全文", rejected["revision"].clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        same["audit_status"], "rejected",
        "no-op save cannot override human rejection"
    );
    let restored = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            input("自动标题", "作者重新修改", rejected["revision"].clone()),
        )
        .await
        .unwrap();
    assert_eq!(restored["audit_status"], "approved");
    assert!(app.community.moderate(&actor,school,post,serde_json::from_value(json!({"revision":restored["revision"],"decision":"approved","reason":"旧指纹","review_digest":digest(&edited).unwrap()})).unwrap(),false).await.is_err());
    // Policy changes never invalidate prior automatic evidence on ordinary reads/interactions.
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    app.schools
        .update(
            &actor,
            school,
            json!({"revision":profile.revision,"config":{"post_audit_mode":"manual"}}),
        )
        .await
        .unwrap();
    assert_eq!(
        app.community.detail(&mini, school, post).await.unwrap()["audit_status"],
        "approved"
    );
    let manual_edit = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            input("待审修改", "重新人工审核", restored["revision"].clone()),
        )
        .await
        .unwrap();
    assert_eq!(manual_edit["audit_status"], "pending");
    assert!(manual_edit["approved_snapshot_id"].is_null());
    let human = app.community.moderate(&actor,school,post,serde_json::from_value(json!({"revision":manual_edit["revision"],"decision":"approved","reason":"核对全文","review_digest":digest(&manual_edit).unwrap()})).unwrap(),false).await.unwrap();
    let review = app.community.review(&actor, school, post).await.unwrap();
    assert_eq!(json!(review.decision_source), "human");
    assert_eq!(review.moderator_id, Some(root));
    // Manual comments stay private and don't notify another user before approval.
    let c = app
        .community
        .comment(
            &mini,
            school,
            post,
            serde_json::from_value(json!({"content":"人工评论"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(c["audit_status"], "pending");
    let cid: Uuid = c["id"].as_str().unwrap().parse().unwrap();
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM notifications WHERE object_id=$1 AND kind='comment')"
        )
        .bind(cid)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    let public_comments = app
        .community
        .comments(&viewer, school, Some(post), Page::default())
        .await
        .unwrap();
    assert!(!public_comments.iter().any(|v| v["id"] == c["id"]));
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    app.schools
        .update(
            &actor,
            school,
            json!({"revision":profile.revision,"config":{"comment_audit_mode":"auto"}}),
        )
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let automatic_comment = app
        .community
        .comment(
            &mini,
            school,
            post,
            serde_json::from_value(json!({"content":"自动评论"})).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(automatic_comment["audit_status"], "approved");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    let comments = app
        .community
        .comments(&actor, school, None, Page::default())
        .await
        .unwrap();
    assert!(comments.iter().any(|v| v["id"] == automatic_comment["id"]
        && v["decision_source"] == "automatic"
        && v["moderator_id"].is_null()));
    // Author content input cannot forge decision metadata.
    assert!(
        serde_json::from_value::<linjian_api::community::domain::CommentInput>(
            json!({"content":"伪造","audit_status":"approved"})
        )
        .is_err()
    );
    // Database guards still reject missing snapshots, foreign evidence, and stale policy.
    assert!(
        sqlx::query("UPDATE posts SET audit_status='approved' WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE posts SET approved_snapshot_id=$2 WHERE id=$1")
            .bind(post)
            .bind(evidence["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO moderation_events(id,school_id,object_id,object_kind,decision,reason,decision_source,content_revision,policy_revision,policy_mode,review_digest) VALUES($1,$2,$3,'post','approved','','automatic',1,1,'auto','x')").bind(Uuid::now_v7()).bind(school).bind(post).execute(&pool).await.is_err());
    assert!(sqlx::query("INSERT INTO moderation_events(id,school_id,actor_id,object_id,object_kind,decision,reason,decision_source,content_revision,review_digest) VALUES($1,$2,$3,$4,'post','approved','','human',1,'x')").bind(Uuid::now_v7()).bind(school).bind(viewer_id).bind(post).execute(&pool).await.is_err());
    // Hidden status survives approval and cannot be used to change audit policy.
    let hidden = app
        .community
        .visibility(
            &actor,
            school,
            post,
            json!({"revision":human["revision"],"visibility":"hidden","top_sort":0}),
        )
        .await
        .unwrap();
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    app.schools
        .update(
            &actor,
            school,
            json!({"revision":profile.revision,"config":{"post_audit_mode":"auto"}}),
        )
        .await
        .unwrap();
    let hidden_edit = app
        .community
        .post(
            &mini,
            school,
            Some(post),
            input("隐藏标题", "隐藏编辑", hidden["revision"].clone()),
        )
        .await
        .unwrap();
    assert_eq!(hidden_edit["audit_status"], "approved");
    assert_eq!(hidden_edit["visibility"], "hidden");
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    app.schools
        .update(
            &actor,
            school,
            json!({"revision":profile.revision,"status":"hidden"}),
        )
        .await
        .unwrap();
    let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
    assert!(
        app.schools
            .update(
                &actor,
                school,
                json!({"revision":profile.revision,"config":{"post_audit_mode":"manual"}})
            )
            .await
            .is_err()
    );
    assert!(
        app.community
            .post(&mini, school, None, input("隐藏校", "正文", Value::Null))
            .await
            .is_err()
    );
    // The write guard serializes policy selection with an in-flight configuration transaction.
    sqlx::query("UPDATE schools SET active=true WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    let mut policy_tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE schools SET config=config||'{\"post_audit_mode\":\"manual\"}'::jsonb,revision=revision+1 WHERE id=$1").bind(school).execute(&mut *policy_tx).await.unwrap();
    let service = app.community.clone();
    let author = mini.clone();
    let mut waiting = tokio::spawn(async move {
        service
            .post(
                &author,
                school,
                None,
                serde_json::from_value(json!({"title":"并发投稿","content":"以提交后的策略为准"}))
                    .unwrap(),
            )
            .await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut waiting)
            .await
            .is_err()
    );
    policy_tx.commit().await.unwrap();
    assert_eq!(waiting.await.unwrap().unwrap()["audit_status"], "pending");
    // Both clients obey the four independent post/comment mode combinations.
    for post_mode in ["manual", "auto"] {
        for comment_mode in ["manual", "auto"] {
            let profile = app.schools.0.school_detail(&actor, school).await.unwrap();
            app.schools.update(&actor,school,json!({"revision":profile.revision,"config":{"post_audit_mode":post_mode,"comment_audit_mode":comment_mode}})).await.unwrap();
            for author in [&actor, &mini] {
                let mut row = app
                    .community
                    .post(
                        author,
                        school,
                        None,
                        input("模式组合", "组合正文", Value::Null),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    row["audit_status"],
                    if post_mode == "auto" {
                        "approved"
                    } else {
                        "pending"
                    }
                );
                let pid: Uuid = row["id"].as_str().unwrap().parse().unwrap();
                if post_mode == "manual" {
                    let decision = serde_json::from_value(json!({"revision":row["revision"],"decision":"approved","reason":"组合核对","review_digest":digest(&row).unwrap()})).unwrap();
                    row = app
                        .community
                        .moderate(&actor, school, pid, decision, false)
                        .await
                        .unwrap();
                }
                assert_eq!(row["audit_status"], "approved");
                let comment = app
                    .community
                    .comment(
                        author,
                        school,
                        pid,
                        serde_json::from_value(json!({"content":"组合评论"})).unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    comment["audit_status"],
                    if comment_mode == "auto" {
                        "approved"
                    } else {
                        "pending"
                    }
                );
            }
        }
    }
    let isolated_post = app
        .community
        .post(
            &actor,
            other,
            None,
            input("其他校默认", "仍须人工", Value::Null),
        )
        .await
        .unwrap();
    assert_eq!(isolated_post["audit_status"], "pending");
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn moderation_mode_upgrade_preserves_legacy_publication_evidence() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_mode_upgrade_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    let migrations = database::MIGRATOR.iter().collect::<Vec<_>>();
    let target = migrations
        .iter()
        .position(|m| m.version == 202611090005)
        .unwrap();
    for migration in &migrations[..target] {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let account = Uuid::new_v4();
    let school = Uuid::new_v4();
    let post = Uuid::new_v4();
    let event = Uuid::new_v4();
    let snapshot = Uuid::new_v4();
    sqlx::query("INSERT INTO accounts(id,username,display_name,password_hash) VALUES($1,'upgrade','历史管理员','unused')").bind(account).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO schools(id,name) VALUES($1,'历史学校')")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content,created_via,source_projection) VALUES($1,$2,$3,'历史标题','历史正文','miniapp','miniapp_summary_v1')").bind(post).bind(school).bind(account).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO moderation_events(id,school_id,actor_id,object_id,object_kind,decision,reason) VALUES($1,$2,$3,$4,'post','approved','历史人工审核')").bind(event).bind(school).bind(account).bind(post).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO forum_publication_snapshots(id,school_id,post_id,author_id,content_revision,approved_post_revision,moderation_event_id,moderator_id,title,content,images,tags,content_digest,summary,summary_algorithm,preview_digest,link_policy) VALUES($1,$2,$3,$4,1,2,$5,$4,'历史标题','历史正文','[]','[]','old-digest','历史正文','plain-v1','old-preview','{}')").bind(snapshot).bind(school).bind(post).bind(account).bind(event).execute(&pool).await.unwrap();
    sqlx::query(
        "UPDATE posts SET audit_status='approved',approved_snapshot_id=$2,revision=2 WHERE id=$1",
    )
    .bind(post)
    .bind(snapshot)
    .execute(&pool)
    .await
    .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(&migrations[target].sql)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT decision_source FROM forum_publication_snapshots WHERE id=$1"
        )
        .bind(snapshot)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "legacy"
    );
    sqlx::query("UPDATE posts SET browse=browse+1 WHERE id=$1")
        .bind(post)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT audit_status FROM posts WHERE id=$1")
            .bind(post)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "approved"
    );
    assert!(
        sqlx::query("UPDATE forum_publication_snapshots SET summary='改写' WHERE id=$1")
            .bind(snapshot)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO moderation_events(id,school_id,actor_id,object_id,object_kind,decision,reason,decision_source) VALUES($1,$2,$3,$4,'post','approved','','legacy')").bind(Uuid::new_v4()).bind(school).bind(account).bind(post).execute(&pool).await.is_err());
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn case_management_projection_handling_and_upgrade() {
    use linjian_api::{
        business::{Error, Page},
        community::domain::case::ResolveCaseInput,
    };
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_cases_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    // Exercise a populated upgrade, keeping historical terminal metadata unknown.
    let migrations = database::MIGRATOR.iter().collect::<Vec<_>>();
    let target = migrations
        .iter()
        .position(|m| m.version == 202611100001)
        .unwrap();
    for migration in &migrations[..target] {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let app = bootstrap::build_application(pool.clone());
    let root = app
        .identity
        .bootstrap("case-root", "显示名", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(root, "browser", false)
        .await
        .unwrap();
    let school: Uuid = app.schools.create(&actor, "案件所属学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let legacy = Uuid::now_v7();
    sqlx::query("INSERT INTO cases(id,school_id,account_id,kind,content,state,resolution) VALUES($1,$2,$3,'feedback','历史反馈','resolved','旧处理')").bind(legacy).bind(school).bind(root).execute(&pool).await.unwrap();
    sqlx::raw_sql(&migrations[target].sql)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
            "SELECT handled_at FROM cases WHERE id=$1"
        )
        .bind(legacy)
        .fetch_one(&pool)
        .await
        .unwrap()
        .is_none()
    );
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'提交人昵称')")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    let post = Uuid::now_v7();
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content) VALUES($1,$2,$3,'搜索帖子标题','原始正文')").bind(post).bind(school).bind(root).execute(&pool).await.unwrap();
    let case = app
        .community
        .case(
            &actor,
            school,
            json!({"kind":"report","post_id":post,"content":"举报原因","images":[]}),
        )
        .await
        .unwrap();
    assert!(case.get("handled_by").is_none());
    let id: Uuid = case["id"].as_str().unwrap().parse().unwrap();
    let rows = app
        .community
        .cases(
            &actor,
            school,
            Page {
                search: Some("搜索帖子标题".into()),
                status: Some("open".into()),
                page_size: Some(1),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["reporter_name"], "提交人昵称");
    assert_eq!(rows[0]["school_name"], "案件所属学校");
    assert!(rows[0]["handled_at"].is_null());
    assert!(
        app.community
            .cases(
                &actor,
                school,
                Page {
                    status: Some("pending".into()),
                    ..Default::default()
                }
            )
            .await
            .is_err()
    );
    assert!(
        app.community
            .cases(&actor, other, Page::default())
            .await
            .unwrap()
            .is_empty()
    );
    let mut member = actor.clone();
    member.platform_admin = false;
    member.grants = vec![linjian_api::business::Grant {
        school_id: school,
        role: "member".into(),
    }];
    let own = app
        .community
        .cases(&member, school, Page::default())
        .await
        .unwrap();
    assert_eq!(own.len(), 2);
    assert!(
        own.iter()
            .all(|v| v.get("handled_by").is_none() && v.get("reporter_name").is_none())
    );
    assert!(
        app.community
            .cases(&member, other, Page::default())
            .await
            .is_err()
    );
    let other_account = app
        .identity
        .bootstrap(
            "other-reporter",
            "其他人",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO cases(id,school_id,account_id,kind,content) VALUES($1,$2,$3,'feedback','其他人的反馈')").bind(Uuid::now_v7()).bind(school).bind(other_account).execute(&pool).await.unwrap();
    assert_eq!(
        app.community
            .cases(&member, school, Page::default())
            .await
            .unwrap()
            .len(),
        2
    );
    let mut mini = actor.clone();
    mini.browser = false;
    let mini_rows = app
        .community
        .cases(&mini, school, Page::default())
        .await
        .unwrap();
    assert!(mini_rows.iter().all(|v| v.get("handled_by").is_none()));
    let input = |revision, state: &str, resolution: &str| {
        serde_json::from_value::<ResolveCaseInput>(
            json!({"revision":revision,"state":state,"resolution":resolution}),
        )
        .unwrap()
    };
    assert!(
        app.community
            .resolve(&mini, school, id, input(1, "resolved", "完成"))
            .await
            .is_err()
    );
    assert!(
        app.community
            .resolve(&member, school, id, input(1, "resolved", "完成"))
            .await
            .is_err()
    );
    assert!(matches!(
        app.community
            .resolve(&actor, other, id, input(1, "resolved", "完成"))
            .await,
        Err(Error::Conflict)
    ));
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(post)
        .fetch_one(&pool)
        .await
        .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        app.community
            .resolve(&actor, school, id, input(1, "resolved", "核实处理")),
        app.community
            .resolve(&actor, school, id, input(1, "dismissed", "核实驳回"))
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let handled = first.or(second).unwrap();
    assert_eq!(handled.record.revision, 2);
    assert_eq!(handled.handled_by, Some(root));
    assert!(handled.handled_at.is_some());
    let repeated = app
        .community
        .resolve(
            &actor,
            school,
            id,
            input(2, handled.record.state.as_str(), &handled.record.resolution),
        )
        .await
        .unwrap();
    assert_eq!(repeated.handled_at, handled.handled_at);
    assert_eq!(repeated.record.revision, 2);
    assert!(matches!(
        app.community
            .resolve(
                &actor,
                school,
                id,
                input(1, handled.record.state.as_str(), &handled.record.resolution)
            )
            .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_events WHERE action='case.resolve' AND object_id=$1"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    // An audit failure rolls back metadata and revision as well as state.
    sqlx::raw_sql("CREATE FUNCTION reject_case_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='case.resolve' THEN RAISE EXCEPTION 'audit unavailable'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_case_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_case_audit();").execute(&pool).await.unwrap();
    assert!(
        app.community
            .resolve(&actor, school, id, input(2, "resolved", "修正处理说明"))
            .await
            .is_err()
    );
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(c) FROM cases c WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after["revision"], 2);
    assert_eq!(after["resolution"], handled.record.resolution);
    assert_eq!(
        after["handled_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
        handled.handled_at.unwrap()
    );
    sqlx::raw_sql(
        "DROP TRIGGER reject_case_audit ON audit_events; DROP FUNCTION reject_case_audit();",
    )
    .execute(&pool)
    .await
    .unwrap();
    let corrected = app
        .community
        .resolve(&actor, school, id, input(2, "resolved", "修正处理说明"))
        .await
        .unwrap();
    assert_eq!(corrected.record.revision, 3);
    assert!(corrected.handled_at > handled.handled_at);
    assert_eq!(
        sqlx::query_scalar::<_, Value>("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
            .bind(post)
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        outbox
    );
    // Reporter fallback uses public display name, never username or contact.
    sqlx::query("DELETE FROM miniapp_users WHERE account_id=$1")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    let found = app
        .community
        .cases(
            &actor,
            school,
            Page {
                search: Some("显示名".into()),
                page_size: Some(1),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(found.len(), 2); // size+1 is fetched before JSON:API truncation.
    assert!(found.iter().all(|v| v["reporter_name"] == "显示名"));
    sqlx::query(
        "UPDATE school_memberships SET role='revoked' WHERE account_id=$1 AND school_id=$2",
    )
    .bind(root)
    .bind(school)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE accounts SET platform_admin=false WHERE id=$1")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        app.community.cases(&actor, school, Page::default()).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        app.community
            .resolve(&actor, school, id, input(3, "resolved", "再次处理"))
            .await,
        Err(Error::Forbidden)
    ));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
