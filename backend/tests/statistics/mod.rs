use super::*;
use linjian_api::{
    business::SessionKind, miniapp_users::south::port::RegistrationStore, statistics::domain::Query,
};
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn statistics_scope_registration_snapshot_and_activity_are_isolated() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("statistics_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap(
            "statistics-root",
            "系统统计管理员",
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
    let s: Uuid = app.schools.create(&actor, "统计甲校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "统计乙校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let input = || {
        serde_json::from_value(
            json!({"title":"统计内容","content":"统计正文","images":[],"tags":["校园"]}),
        )
        .unwrap()
    };
    let p = app.community.post(&actor, s, None, input()).await.unwrap();
    app.community
        .post(&actor, other, None, input())
        .await
        .unwrap();
    let user = linjian_api::miniapp_users::south::adapter::postgres::Repository(pool.clone())
        .register("statistics-app", "openid")
        .await
        .unwrap();
    assert_eq!(
        user,
        linjian_api::miniapp_users::south::adapter::postgres::Repository(pool.clone())
            .register("statistics-app", "openid")
            .await
            .unwrap()
    );
    sqlx::query("UPDATE accounts SET bound_school_id=$2 WHERE id=$1")
        .bind(user)
        .bind(s)
        .execute(&pool)
        .await
        .unwrap();
    let observed = app
        .statistics
        .report(&actor, None, Query::default())
        .await
        .unwrap();
    assert_eq!(observed.summary.posts, Some(2));
    assert!(!observed.coverage.content);
    assert!(observed.comparisons.is_empty());
    assert!(observed.buckets.is_empty());
    let observed_content = app
        .statistics
        .report(
            &actor,
            None,
            Query {
                view: Some("content".into()),
                comparison: Some("previous".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(observed_content.summary.posts, Some(2));
    assert_eq!(
        observed_content.comparisons[0].changes["posts"].state,
        "unavailable"
    );
    assert!(observed_content.summary.registrations.is_none());
    sqlx::query("UPDATE statistics_coverage SET start_at='2020-01-01'")
        .execute(&pool)
        .await
        .unwrap();
    let full = app
        .statistics
        .report(&actor, None, Query::default())
        .await
        .unwrap();
    assert_eq!(full.summary.posts, Some(2));
    assert!(full.summary.participants.is_none());
    assert_eq!(full.summary.registrations, Some(1));
    assert!(full.buckets.is_empty());
    assert!(full.comparisons.is_empty());
    assert!(full.schools.is_empty());
    assert_eq!(full.view, "overview");
    let registration = app
        .statistics
        .report(
            &actor,
            None,
            Query {
                view: Some("registration".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(registration.registration_groups["unassigned"], 1);
    assert!(registration.summary.posts.is_none());
    assert!(!registration.buckets.is_empty());
    assert!(full.summary.active_users.is_none());
    let single = app
        .statistics
        .report(&actor, Some(s), Query::default())
        .await
        .unwrap();
    assert_eq!(single.summary.posts, Some(1));
    assert_eq!(single.summary.registrations, Some(0));
    let school_user = app
        .identity
        .bootstrap(
            "statistics-school",
            "学校统计管理员",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'admin')")
        .bind(s)
        .bind(school_user)
        .execute(&pool)
        .await
        .unwrap();
    let (school_token, school_actor) = app
        .identity
        .0
        .session(
            school_user,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    assert!(
        app.statistics
            .report(&school_actor, None, Query::default())
            .await
            .is_err()
    );
    assert!(
        app.statistics
            .report(&school_actor, Some(other), Query::default())
            .await
            .is_err()
    );
    let own = app
        .statistics
        .report(
            &school_actor,
            Some(s),
            Query {
                view: Some("content".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(
        app.statistics
            .snapshot(&school_actor, Some(s), single.id)
            .await
            .is_err()
    );
    let before: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT browse FROM posts WHERE id=$1),(SELECT count(*) FROM forum_outbox)",
    )
    .bind(p["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    let csv = linjian_api::statistics::north::local::csv(&own, "periods").unwrap();
    assert!(csv.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(!String::from_utf8(csv).unwrap().contains("统计正文"));
    app.community.post(&actor, s, None, input()).await.unwrap();
    assert_eq!(
        app.statistics
            .snapshot(&school_actor, Some(s), own.id)
            .await
            .unwrap()
            .summary
            .posts,
        Some(1)
    );
    let router = linjian_api::http::app_router(app.clone());
    for (token, path, expected) in [
        (
            &school_token,
            format!("/api/v1/schools/{other}/statistics"),
            StatusCode::FORBIDDEN,
        ),
        (
            &school_token,
            "/api/v1/platform/statistics".into(),
            StatusCode::FORBIDDEN,
        ),
        (
            &root_token,
            format!("/api/v1/schools/{s}/statistics"),
            StatusCode::OK,
        ),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("cookie", format!("linjian_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
    }
    sqlx::query("UPDATE school_memberships SET role='revoked' WHERE account_id=$1")
        .bind(school_user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.statistics
            .snapshot(&school_actor, Some(s), own.id)
            .await
            .is_err()
    );
    let after: i64 = sqlx::query_scalar("SELECT browse FROM posts WHERE id=$1")
        .bind(p["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, before.0);
    // Activity remains unavailable before deployment verification, then counts exact account sets.
    sqlx::query("INSERT INTO statistics_coverage VALUES($1,'activity','2020-01-01',NULL,'complete','test',now())").bind(Uuid::new_v4()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO statistics_activity_runs VALUES($1,'2020-01-01',now()+interval '1 minute',now(),NULL,false)").bind(Uuid::new_v4()).execute(&pool).await.unwrap();
    for id in [s, other] {
        sqlx::query("INSERT INTO statistics_activity_events(event_id,account_id,school_id,kind,occurred_at) VALUES($1,$2,$3,'post_create',now())").bind(Uuid::new_v4()).bind(user).bind(id).execute(&pool).await.unwrap();
    }
    let full = app
        .statistics
        .report(
            &actor,
            None,
            Query {
                view: Some("schools".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(full.summary.active_users, Some(1));
    assert_eq!(full.summary.registered_participants, Some(1));
    assert_eq!(
        full.schools
            .iter()
            .map(|s| s.metrics.active_users.unwrap())
            .sum::<i64>(),
        2
    );
    let router = linjian_api::http::app_router(app.clone());
    sqlx::query("INSERT INTO schools(id,name) SELECT gen_random_uuid(),'分页学校' || n FROM generate_series(1,98) AS n").execute(&pool).await.unwrap();
    let overview_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/platform/statistics")
                .header("cookie", format!("linjian_session={root_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(overview_response.status(), StatusCode::OK);
    let overview_bytes = axum::body::to_bytes(overview_response.into_body(), 8192)
        .await
        .unwrap();
    let overview: serde_json::Value = serde_json::from_slice(&overview_bytes).unwrap();
    assert_eq!(overview["data"]["attributes"]["school_count"], 100);
    assert_eq!(overview["data"]["attributes"]["buckets"], json!([]));
    assert_eq!(overview["data"]["attributes"]["comparisons"], json!([]));
    assert_eq!(overview["data"]["attributes"]["schools"], json!([]));
    assert_eq!(overview["data"]["attributes"]["school_ids"], json!([]));
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/platform/statistics?view=schools")
                .header("cookie", format!("linjian_session={root_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 100_000)
        .await
        .unwrap();
    assert!(
        bytes.len() < 8192,
        "school snapshot metadata must stay small"
    );
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(document["data"]["attributes"]["schools"], json!([]));
    assert!(
        document["data"]["attributes"]["school_ids"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let snapshot = document["data"]["id"].as_str().unwrap();
    let response = router.oneshot(Request::builder()
        .uri(format!("/api/v1/platform/statistics/snapshots/{snapshot}/schools?page=1&size=1&sort=posts"))
        .header("cookie", format!("linjian_session={root_token}"))
        .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 100_000)
        .await
        .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    assert_eq!(document["meta"]["page"]["has_more"], true);
    sqlx::query("UPDATE statistics_snapshots SET expires_at=now()-interval '1 second' WHERE id=$1")
        .bind(full.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.statistics
            .snapshot(&actor, None, full.id)
            .await
            .is_err()
    );

    // The real collector persists repeated event IDs once, and never browses content.
    sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'member')")
        .bind(s)
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let (_, mini_actor) = app
        .identity
        .0
        .session(user, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let own_post = app
        .community
        .post(&mini_actor, s, None, input())
        .await
        .unwrap();
    let own_post_id = own_post["id"].as_str().unwrap().parse().unwrap();
    let v = linjian_api::statistics::domain::ActivityInput {
        event_id: Uuid::new_v4(),
        kind: "post_open".into(),
        school_id: s,
        post_id: Some(own_post_id),
    };
    app.statistics
        .validate_activity(&mini_actor, &v)
        .await
        .unwrap();
    for _ in 0..2 {
        app.activity.event(
            &mini_actor,
            Some(s),
            "post_open",
            Some(v.event_id),
            Some(own_post_id),
        );
    }
    let worker = app.activity.clone();
    let worker_pool = pool.clone();
    let task = tokio::spawn(async move {
        worker.run(worker_pool, Default::default()).await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM statistics_activity_events WHERE event_id=$1",
            )
            .bind(v.event_id)
            .fetch_one(&pool)
            .await
            .unwrap();
            if n == 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    let browse: i64 = sqlx::query_scalar("SELECT browse FROM posts WHERE id=$1")
        .bind(own_post_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(browse, 0);
    task.abort();
    let _ = task.await;

    // Overflow is durable as a failed interval; subsequent clean intervals recover.
    let since = chrono::Utc::now();
    let collector = linjian_api::statistics::south::adapter::recorder::Recorder::new(1);
    for _ in 0..2 {
        collector.event(&mini_actor, Some(s), "feed_open", None, None);
    }
    let collector_pool = pool.clone();
    let recovery = tokio::spawn(async move {
        collector.run(collector_pool, Default::default()).await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(10),async{loop{let ready:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM statistics_activity_runs WHERE started_at>=$1 AND failed AND closed_at IS NOT NULL) AND EXISTS(SELECT 1 FROM statistics_activity_runs WHERE started_at>=$1 AND NOT failed)").bind(since).fetch_one(&pool).await.unwrap();if ready{break;}tokio::time::sleep(std::time::Duration::from_millis(25)).await;}}).await.unwrap();
    recovery.abort();
    let _ = recovery.await;
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn registration_upgrade_preserves_time_without_inventing_school_history() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("statistics_upgrade_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    let migrations = database::MIGRATOR.iter().collect::<Vec<_>>();
    let index = migrations
        .iter()
        .position(|m| m.version == 202611180002)
        .unwrap();
    for m in &migrations[..index] {
        sqlx::raw_sql(&m.sql).execute(&pool).await.unwrap();
    }
    let s = Uuid::new_v4();
    let u = Uuid::new_v4();
    sqlx::query("INSERT INTO schools(id,name) VALUES($1,'历史注册归属校')")
        .bind(s)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO accounts(id,username,display_name,bound_school_id) VALUES($1,'historical-user','历史用户',$2)").bind(u).bind(s).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname,created_at) VALUES($1,'历史昵称','2024-02-29T00:00:00Z')").bind(u).execute(&pool).await.unwrap();
    sqlx::raw_sql(&migrations[index].sql)
        .execute(&pool)
        .await
        .unwrap();
    let (assignment,school,time):(String,Option<Uuid>,chrono::DateTime<chrono::Utc>)=sqlx::query_as("SELECT school_assignment,registration_school_id,registered_at FROM statistics_registrations WHERE account_id=$1").bind(u).fetch_one(&pool).await.unwrap();
    assert_eq!(assignment, "unknown");
    assert!(school.is_none());
    assert_eq!(
        time,
        "2024-02-29T00:00:00Z"
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap()
    );
    sqlx::query("UPDATE accounts SET bound_school_id=NULL WHERE id=$1")
        .bind(u)
        .execute(&pool)
        .await
        .unwrap();
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM statistics_registrations WHERE account_id=$1 AND school_assignment='unknown'").bind(u).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
