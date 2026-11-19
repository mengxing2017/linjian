use super::*;
use linjian_api::{
    business::{Error, SessionKind},
    community::domain::post_recommendation::RecommendPost,
    content_publication::{
        domain::{Failure, Link, Preparation},
        north::local::Worker as PreparationWorker,
        south::port::MiniappLinkProvider,
    },
    outbox::south::{
        adapter::postgres::Repository as Outbox,
        port::{OutboxStore, ReceiptResult},
    },
};
struct NoLink;
impl MiniappLinkProvider for NoLink {
    async fn generate(&self, _: &Preparation) -> Result<Link, Failure> {
        panic!("web source needs no external provider")
    }
}
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn manual_recommendation_uses_single_chain_and_live_capabilities() {
    // Configuration belongs to this child process, never mutate global environment in parallel tests.
    if std::env::var_os("LINJIAN_RECOMMENDATION_TEST_CHILD").is_none() {
        let result=std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored","--exact","post_recommendation_tests::manual_recommendation_uses_single_chain_and_live_capabilities","--nocapture"])
            .env("LINJIAN_RECOMMENDATION_TEST_CHILD","1")
            .env("LINJIAN_WEKEEPER_ORIGIN","https://keeper.example.com")
            .env("LINJIAN_WEKEEPER_KEY_ID","test-key")
            .env("LINJIAN_WEKEEPER_SECRET",linjian_api::source_access::domain::encode(&[1;32]))
            .status().unwrap();
        assert!(result.success());
        return;
    }
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated URL required"),
    )
    .await
    .unwrap();
    let schema = format!("recommendation_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let obsolete:i64=sqlx::query_scalar("SELECT count(*) FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname IN ('forum_enqueue_material','forum_source_event','forum_legacy_source_event')").fetch_one(&pool).await.unwrap();
    assert_eq!(obsolete, 0);
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap(
            "recommend-root",
            "平台管理",
            "test-password-at-least-12",
            true,
        )
        .await
        .unwrap();
    let (root_token, actor) = app
        .identity
        .0
        .session(
            root,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let school: Uuid = app.schools.create(&actor, "人工推荐校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let operator_id = app
        .identity
        .bootstrap(
            "recommend-operator",
            "学校运营",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, operator_id, "operator")
        .await
        .unwrap();
    let (_, operator) = app
        .identity
        .0
        .session(
            operator_id,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let connection = Uuid::new_v4();
    let version = Uuid::new_v4();
    let source = Uuid::new_v4();
    sqlx::query("INSERT INTO source_connections(id,name,source_id,origin,key_id,checked_at) VALUES($1,'test',$2,'https://keeper.example.com','test-key',now())").bind(connection).bind(source).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin) VALUES($1,$2,1,$3,'https://keeper.example.com')").bind(version).bind(connection).bind(source).execute(&pool).await.unwrap();
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
                route_id: "route-recommend".into(),
                active: true,
            },
            0,
            1,
        )
        .await
        .unwrap();
    let post=app.community.post(&actor,school,None,serde_json::from_value(json!({"title":"优质讨论","content":"人工认定具有再次推荐价值","images":[],"tags":[]})).unwrap()).await.unwrap();
    let id: Uuid = post["id"].as_str().unwrap().parse().unwrap();
    approve_post(&pool, &actor, id).await;
    let preview = app
        .community
        .recommendation_preview(&operator, school, id)
        .await
        .unwrap();
    assert_eq!(preview["authorized"], false);
    let input:RecommendPost=serde_json::from_value(json!({"revision":preview["revision"],"content_revision":preview["content_revision"],"publication_snapshot_id":preview["publication_snapshot_id"],"reason":"人工再次推荐","idempotency_key":Uuid::new_v4()})).unwrap();
    assert!(matches!(
        app.community
            .recommend_post(&operator, school, id, input.clone())
            .await,
        Err(Error::Forbidden)
    ));
    app.schools
        .recommendation_capability(&actor, school, operator_id, true)
        .await
        .unwrap();
    assert!(matches!(
        app.community
            .recommend_post(&operator, school, id, input.clone())
            .await,
        Err(Error::Policy("recommendation_pending"))
    ));
    let preparation = PreparationWorker::new(pool.clone()).unwrap();
    assert!(preparation.run_with(&NoLink).await.unwrap());
    let outbox = Outbox(pool.clone());
    let auto = outbox.claim().await.unwrap().unwrap();
    let original_body = auto.body.clone();
    outbox
        .finish(
            &auto,
            Ok(ReceiptResult {
                id: Uuid::new_v4(),
                status: "received".into(),
                summary: json!({}),
            }),
        )
        .await
        .unwrap();
    assert!(matches!(
        app.community
            .recommend_post(&operator, school, id, input.clone())
            .await,
        Err(Error::Policy("recommendation_cooldown"))
    ));
    sqlx::query("UPDATE forum_outbox SET created_at=now()-interval '25 hours' WHERE post_id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        app.community
            .recommend_post(&operator, school, id, input.clone()),
        app.community
            .recommend_post(&operator, school, id, input.clone())
    );
    let (record, new) = one.unwrap();
    let (replay, other_new) = two.unwrap();
    assert_ne!(new, other_new);
    assert_eq!(record, replay);
    let event: Uuid = record["id"].as_str().unwrap().parse().unwrap();
    assert_ne!(event, auto.id);
    assert_eq!(record["trigger_type"], "manual_recommendation");
    assert_eq!(record["actor_name"], "学校运营");
    assert_eq!(
        app.community
            .recommendations(&operator, school, id, None, Default::default())
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(
        app.community
            .recommendation_preview(&operator, other, id)
            .await
            .is_err()
    );
    assert!(
        outbox
            .list(&operator, school, Default::default())
            .await
            .is_err()
    );
    let mut changed = input.clone();
    changed.reason = "换一个原因".into();
    assert!(matches!(
        app.community
            .recommend_post(&operator, school, id, changed)
            .await,
        Err(Error::Policy("recommendation_idempotency_conflict"))
    ));
    let mut different = input.clone();
    different.idempotency_key = Uuid::new_v4();
    assert!(matches!(
        app.community
            .recommend_post(&operator, school, id, different)
            .await,
        Err(Error::Policy("recommendation_pending"))
    ));
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert!(preparation.run_with(&NoLink).await.unwrap());
    let manual = outbox.claim().await.unwrap().unwrap();
    assert_eq!(manual.id, event);
    assert_eq!(
        serde_json::from_str::<Value>(&manual.body).unwrap()["data"]["attributes"]["content"],
        serde_json::from_str::<Value>(&original_body).unwrap()["data"]["attributes"]["content"]
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT request_body FROM forum_outbox WHERE id=$1")
            .bind(auto.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        original_body
    );
    outbox
        .finish(
            &manual,
            Err((
                linjian_api::source_access::domain::Fault("temporarily_unavailable"),
                503,
            )),
        )
        .await
        .unwrap();
    assert!(matches!(
        outbox
            .terminate(&actor, school, event, "未知请求不得关闭")
            .await,
        Err(Error::Policy("source_termination_not_safe"))
    ));
    // Content changes do not turn an already successful idempotency identity into a new intent.
    let current = app
        .community
        .management_detail(&actor, school, id)
        .await
        .unwrap();
    app.community
        .management_status(
            &actor,
            school,
            id,
            serde_json::from_value(
                json!({"revision":current.post.revision,"visibility":"hidden","reason":"测试失效"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        app.community
            .recommend_post(&operator, school, id, input.clone())
            .await
            .unwrap()
            .0["id"],
        record["id"]
    );
    app.schools
        .recommendation_capability(&actor, school, operator_id, false)
        .await
        .unwrap();
    assert!(matches!(
        app.community
            .recommend_post(&operator, school, id, input.clone())
            .await,
        Err(Error::Forbidden)
    ));
    app.schools
        .recommendation_capability(&actor, school, operator_id, true)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, operator_id, "member")
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, operator_id, "operator")
        .await
        .unwrap();
    assert_eq!(
        app.community
            .recommendation_preview(&operator, school, id)
            .await
            .unwrap()["authorized"],
        false
    );
    // Safe termination is only available for a never-attempted blocked event.
    let another = app
        .community
        .post(
            &actor,
            school,
            None,
            serde_json::from_value(json!({"title":"受阻帖子","content":"无法准备的内容"})).unwrap(),
        )
        .await
        .unwrap();
    let another: Uuid = another["id"].as_str().unwrap().parse().unwrap();
    approve_post(&pool, &actor, another).await;
    let blocked: Uuid = sqlx::query_scalar(
        "UPDATE forum_outbox SET delivery_state='blocked' WHERE post_id=$1 RETURNING id",
    )
    .bind(another)
    .fetch_one(&pool)
    .await
    .unwrap();
    outbox
        .terminate(&actor, school, blocked, "核对从未尝试且无法恢复")
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT delivery_state FROM forum_outbox WHERE id=$1")
            .bind(blocked)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "cancelled"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_post_recommendations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    sqlx::query("UPDATE forum_outbox SET created_at=now()-interval '25 hours' WHERE post_id=$1")
        .bind(another)
        .execute(&pool)
        .await
        .unwrap();
    let p = app
        .community
        .recommendation_preview(&actor, school, another)
        .await
        .unwrap();
    let command:RecommendPost=serde_json::from_value(json!({"revision":p["revision"],"content_revision":p["content_revision"],"publication_snapshot_id":p["publication_snapshot_id"],"reason":"再次人工推荐","idempotency_key":Uuid::new_v4()})).unwrap();
    let event_count: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_recommendation_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='post.recommend' THEN RAISE EXCEPTION 'injected audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_recommendation_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_recommendation_audit();").execute(&pool).await.unwrap();
    assert!(
        app.community
            .recommend_post(&actor, school, another, command.clone())
            .await
            .is_err()
    );
    assert_eq!(
        event_count,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    sqlx::raw_sql("DROP TRIGGER reject_recommendation_audit ON audit_events; DROP FUNCTION reject_recommendation_audit();").execute(&pool).await.unwrap();
    let mut second = command.clone();
    second.idempotency_key = Uuid::new_v4();
    let (left, right) = tokio::join!(
        app.community
            .recommend_post(&actor, school, another, command.clone()),
        app.community
            .recommend_post(&actor, school, another, second.clone())
    );
    let winner = match (left, right) {
        (Ok(_), Err(Error::Policy("recommendation_pending"))) => command,
        (Err(Error::Policy("recommendation_pending")), Ok(_)) => second,
        _ => panic!("different request keys must not bypass the pending-intent constraint"),
    };
    let router = linjian_api::http::app_router(app.clone());
    let path = format!("/api/v1/schools/{school}/post-management/{another}/recommendations");
    let body = json!({"data":{"type":"post-recommendations","attributes":winner}}).to_string();
    let forbidden = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&path)
                .header("Cookie", format!("linjian_session={root_token}"))
                .header("Content-Type", "application/vnd.api+json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&path)
                .header("Cookie", format!("linjian_session={root_token}"))
                .header("Origin", "http://localhost:5173")
                .header("X-CSRF-Token", &actor.csrf)
                .header("Content-Type", "application/vnd.api+json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let resource: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(resource["data"]["type"], "post-recommendations");
    assert!(resource["data"]["attributes"].get("route_id").is_none());
    let target_schema = format!("recommendation_restore_{}", Uuid::new_v4().simple());
    let target = isolated(&admin, &target_schema).await;
    database::migrate(&target).await.unwrap();
    let files = linjian_api::media::south::adapter::filesystem::Files::default();
    let snapshot = migration::export(&pool, &files).await.unwrap();
    migration::import(&target, &snapshot, false, &files)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_post_recommendations")
            .fetch_one(&target)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        snapshot.digest,
        migration::export(&target, &files).await.unwrap().digest
    );
    target.close().await;
    sqlx::query(&format!("DROP SCHEMA {target_schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
