use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use linjian_api::{
    bootstrap,
    business::SessionKind,
    community::domain::hot_posts::{HotPostPolicy, HotPostsQuery},
    database,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "explicit isolated LINJIAN_TEST_DATABASE_URL; synthetic capacity benchmark"]
async fn hot_posts_capacity_and_query_plan() {
    let url = std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required");
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("forum_hot_capacity_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    let scope = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |connection, _| {
            let query = format!("SET search_path TO {scope}");
            Box::pin(async move {
                sqlx::query(&query).execute(connection).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap("root", "性能测试管理员", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (token, actor) = app
        .identity
        .0
        .session(
            root,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let school: Uuid = app.schools.create(&actor, "热点学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let empty: Uuid = app.schools.create(&actor, "空学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let time: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT now()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let seed = format!(
        include_str!("hot_posts/performance.sql"),
        school = school,
        other = other,
        author = root,
        time = time.to_rfc3339()
    );
    let mut transaction = pool.begin().await.unwrap();
    sqlx::raw_sql(&seed)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    sqlx::query("VACUUM ANALYZE").execute(&pool).await.unwrap();
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&pool)
        .await
        .unwrap();
    println!(
        "PostgreSQL: {version}; schema={schema}; pool=5; concurrent requests=20; posts=30000; candidate posts=10000; likes=100000; comments=110000"
    );
    let first = std::time::Instant::now();
    let initial = app
        .community
        .hot_posts(&actor, school, HotPostsQuery::default())
        .await
        .unwrap();
    println!(
        "first read after seed/vacuum analyze: {:?} (not a controlled cold-cache benchmark)",
        first.elapsed()
    );
    assert_eq!(initial.items.len(), 10);
    assert_eq!(initial.items[0].comment_count, 10010);
    assert!(
        app.community
            .hot_posts(&actor, empty, HotPostsQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let policy = HotPostPolicy::default();
    let explain = format!(
        "EXPLAIN (ANALYZE, BUFFERS) {}",
        include_str!("../src/community/south/adapter/postgres/hot_posts.sql")
    );
    let plan: Vec<String> = sqlx::query_scalar(&explain)
        .bind(school)
        .bind(time - policy.window)
        .bind(time)
        .bind(policy.comment_weight)
        .bind(policy.offset_hours)
        .bind(policy.exponent)
        .bind(policy.minimum_engagement)
        .bind(10_i64)
        .fetch_all(&pool)
        .await
        .unwrap();
    println!("{}", plan.join("\n"));
    let router = linjian_api::http::app_router(app);
    let mut samples = Vec::new();
    for _ in 0..3 {
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..20 {
            let router = router.clone();
            let token = token.clone();
            tasks.spawn(async move {
                let started = std::time::Instant::now();
                let response = router
                    .oneshot(
                        Request::builder()
                            .uri(format!("/api/v1/schools/{school}/hot-posts?limit=10"))
                            .header("cookie", format!("linjian_session={token}"))
                            .header("origin", "http://localhost:5173")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                started.elapsed().as_secs_f64() * 1000.0
            });
        }
        while let Some(sample) = tasks.join_next().await {
            samples.push(sample.unwrap());
        }
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "60 HTTP samples, 20 concurrent: p50={:.2}ms p95={:.2}ms max={:.2}ms",
        samples[29], samples[56], samples[59]
    );
    assert!(
        samples[56] <= 500.0,
        "hot posts p95 exceeds the proposed 500ms capacity budget; isolated schema {schema} retained for diagnosis"
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
