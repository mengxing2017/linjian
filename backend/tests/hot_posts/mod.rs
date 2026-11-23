use super::*;
use chrono::{DateTime, Duration, Utc};
use linjian_api::{
    business::{Actor, Error, SessionKind},
    community::domain::hot_posts::{HotPostPolicy, HotPostsQuery},
};

async fn post_at(pool: &PgPool, actor: &Actor, school: Uuid, id: Uuid, time: DateTime<Utc>) {
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content,display_published_at) VALUES($1,$2,$3,$4,'正文',$5)")
        .bind(id).bind(school).bind(actor.id).bind(format!("热榜帖子 {id}")).bind(time)
        .execute(pool).await.unwrap();
    approve_post(pool, actor, id).await;
}

async fn comments(pool: &PgPool, actor: &Actor, school: Uuid, post: Uuid, amount: i32) {
    sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,content,audit_status) SELECT gen_random_uuid(),$1,$2,$3,'有效评论','approved' FROM generate_series(1,$4)")
        .bind(school).bind(post).bind(actor.id).bind(amount).execute(pool).await.unwrap();
}

async fn ranking_at(
    pool: &PgPool,
    school: Uuid,
    time: DateTime<Utc>,
    limit: i64,
) -> Vec<(Uuid, Uuid, i64, String, String, bool, i64, i64)> {
    let policy = HotPostPolicy::default();
    sqlx::query_as(include_str!(
        "../../src/community/south/adapter/postgres/hot_posts.sql"
    ))
    .bind(school)
    .bind(time - policy.window)
    .bind(time)
    .bind(policy.comment_weight)
    .bind(policy.offset_hours)
    .bind(policy.exponent)
    .bind(policy.minimum_engagement)
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn hot_posts_ranking_scope_counts_protocol_and_no_side_effects() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("forum_hot_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &name).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap("root", "管理员", "test-password-at-least-12", true)
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
    let school: Uuid = app.schools.create(&actor, "热榜学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app.schools.create(&actor, "其他学校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let user = app
        .identity
        .bootstrap("reader", "成员", "test-password-at-least-12", false)
        .await
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'读者')")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, user, "member")
        .await
        .unwrap();
    phone_bindings_tests::verify_fixture_phone(&pool, user).await;
    let (token, member) = app
        .identity
        .0
        .session(user, SessionKind::Miniapp, None)
        .await
        .unwrap();
    assert!(
        app.community
            .hot_posts(&member, school, HotPostsQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(matches!(
        app.community
            .hot_posts(&member, other, HotPostsQuery::default())
            .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        app.community
            .hot_posts(&member, school, HotPostsQuery { limit: 0 })
            .await,
        Err(Error::Invalid(_))
    ));

    let time: DateTime<Utc> = sqlx::query_scalar("SELECT now()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let fresh = Uuid::from_u128(1);
    let tie = Uuid::from_u128(2);
    let lower_boundary = Uuid::from_u128(3);
    let expired = Uuid::from_u128(4);
    let future = Uuid::from_u128(5);
    let low = Uuid::from_u128(6);
    let aggregate = Uuid::from_u128(7);
    for (post, published) in [
        (fresh, time),
        (tie, time),
        (lower_boundary, time - Duration::days(7)),
        (
            expired,
            time - Duration::days(7) - Duration::microseconds(1),
        ),
        (future, time + Duration::microseconds(1)),
        (low, time),
        (aggregate, time - Duration::hours(1)),
    ] {
        post_at(&pool, &actor, school, post, published).await;
        comments(
            &pool,
            &actor,
            school,
            post,
            if post == low {
                1
            } else if post == aggregate {
                3
            } else {
                2
            },
        )
        .await;
    }
    sqlx::query(
        "INSERT INTO post_reactions VALUES($1,$2,$3,'like'),($1,$2,$4,'like'),($1,$2,$3,'collect')",
    )
    .bind(school)
    .bind(aggregate)
    .bind(root)
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    for (audit, visibility, deleted) in [
        ("pending", "visible", false),
        ("rejected", "visible", false),
        ("approved", "hidden", false),
        ("approved", "visible", true),
    ] {
        sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,content,audit_status,visibility,deleted) VALUES($1,$2,$3,$4,'不计数',$5,$6,$7)")
            .bind(Uuid::new_v4()).bind(school).bind(aggregate).bind(user).bind(audit).bind(visibility).bind(deleted).execute(&pool).await.unwrap();
    }
    let ranked = ranking_at(&pool, school, time, 20).await;
    assert_eq!(
        ranked.iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![aggregate, fresh, tie, lower_boundary]
    );
    assert_eq!((ranked[0].6, ranked[0].7), (2, 3));
    assert_eq!(
        ranked.iter().map(|row| row.2).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(ranking_at(&pool, school, time, 1).await[0].0, aggregate);
    assert!(ranking_at(&pool, other, time, 20).await.is_empty());
    for _ in 0..20 {
        let post = Uuid::new_v4();
        post_at(&pool, &actor, school, post, time).await;
    }
    assert_eq!(ranking_at(&pool, school, time, 1).await[0].0, aggregate);
    for _ in 0..2 {
        app.community
            .react(&member, school, low, "like", true)
            .await
            .unwrap();
    }
    let threshold = ranking_at(&pool, school, time, 20).await;
    assert_eq!(threshold.iter().find(|row| row.0 == low).unwrap().6, 1);
    app.community
        .react(&member, school, low, "like", false)
        .await
        .unwrap();
    assert!(
        !ranking_at(&pool, school, time, 20)
            .await
            .iter()
            .any(|row| row.0 == low)
    );
    for (audit, visibility) in [
        ("pending", "visible"),
        ("rejected", "visible"),
        ("approved", "hidden"),
        ("approved", "offline"),
        ("approved", "deleted"),
    ] {
        let post = Uuid::new_v4();
        post_at(&pool, &actor, school, post, time).await;
        comments(&pool, &actor, school, post, 20).await;
        sqlx::query("UPDATE posts SET audit_status=$2,visibility=$3 WHERE id=$1")
            .bind(post)
            .bind(audit)
            .bind(visibility)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE posts SET is_top=true,sort_order=999 WHERE id=$1")
        .bind(tie)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        ranking_at(&pool, school, time, 20)
            .await
            .iter()
            .map(|row| row.0)
            .collect::<Vec<_>>(),
        vec![aggregate, fresh, tie, lower_boundary]
    );
    let before: Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM posts p")
            .fetch_one(&pool)
            .await
            .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let notices: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications")
        .fetch_one(&pool)
        .await
        .unwrap();
    let result = app
        .community
        .hot_posts(&member, school, HotPostsQuery::default())
        .await
        .unwrap();
    assert_eq!(result.items[0].id, aggregate);
    assert_eq!(result.meta.ranked_at, result.meta.validated_at);
    assert_eq!(
        result.meta.window_end - result.meta.window_start,
        Duration::days(7)
    );
    assert_eq!(
        before,
        sqlx::query_scalar::<_, Value>("SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM posts p")
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    assert_eq!(
        outbox,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    assert_eq!(
        notices,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications")
            .fetch_one(&pool)
            .await
            .unwrap()
    );

    let router = linjian_api::http::app_router(app.clone());
    for (query, expected) in [
        ("", StatusCode::OK),
        ("?limit=1", StatusCode::OK),
        ("?limit=20", StatusCode::OK),
        ("?limit=0", StatusCode::UNPROCESSABLE_ENTITY),
        ("?limit=21", StatusCode::UNPROCESSABLE_ENTITY),
        ("?limit=-1", StatusCode::BAD_REQUEST),
        ("?limit=1.5", StatusCode::BAD_REQUEST),
        ("?limit=no", StatusCode::BAD_REQUEST),
        ("?limit=1&limit=2", StatusCode::BAD_REQUEST),
        ("?sort=browse", StatusCode::BAD_REQUEST),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/schools/{school}/hot-posts{query}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{query}");
        assert_eq!(
            response.headers()["content-type"],
            "application/vnd.api+json"
        );
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let document: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
        if expected == StatusCode::OK {
            assert_eq!(document["data"][0]["type"], "hot-posts");
            assert_eq!(
                document["data"][0]["attributes"].as_object().unwrap().len(),
                5
            );
            assert!(document["meta"].get("page").is_none());
        } else {
            assert!(document["errors"].is_array());
        }
    }
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/schools/{school}/hot-posts"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/schools/{other}/hot-posts"))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE posts SET visibility='hidden' WHERE id=$1")
        .bind(aggregate)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !app.community
            .hot_posts(&actor, school, HotPostsQuery::default())
            .await
            .unwrap()
            .items
            .iter()
            .any(|item| item.id == aggregate)
    );
    sqlx::query("UPDATE schools SET config=jsonb_set(config,'{post_audit_mode}','\"auto\"'),revision=revision+1 WHERE id=$1")
        .bind(school).execute(&pool).await.unwrap();
    let seed = app.post_seeds.preview(&actor, school, serde_json::from_value(json!({
        "name":"热榜种子时间验证", "authors":[{"key":"author","nickname":"校园作者","avatar_id":2}],
        "rows":[
            {"author_key":"author","title":"近期展示种子帖","content":"正文","tags":[],"display_published_at":time-Duration::hours(2)},
            {"author_key":"author","title":"窗口外种子帖","content":"正文","tags":[],"display_published_at":time-Duration::days(10)}
        ]
    })).unwrap()).await.unwrap();
    let batch: Uuid = seed["id"].as_str().unwrap().parse().unwrap();
    app.post_seeds
        .commit(
            &actor,
            school,
            batch,
            linjian_api::community::domain::post_seed::CommitInput {
                revision: seed["revision"].as_i64().unwrap(),
                digest: seed["preview_digest"].as_str().unwrap().into(),
                idempotency_key: Uuid::new_v4(),
            },
        )
        .await
        .unwrap();
    let seeded: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,title FROM posts WHERE seed_batch_id=$1 ORDER BY display_published_at DESC",
    )
    .bind(batch)
    .fetch_all(&pool)
    .await
    .unwrap();
    for (post, _) in &seeded {
        comments(&pool, &actor, school, *post, 2).await;
    }
    let seed_ranking = app
        .community
        .hot_posts(&member, school, HotPostsQuery::default())
        .await
        .unwrap();
    assert!(seed_ranking.items.iter().any(|item| item.id == seeded[0].0));
    assert!(!seed_ranking.items.iter().any(|item| item.id == seeded[1].0));
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        app.community
            .hot_posts(&actor, school, HotPostsQuery::default())
            .await,
        Err(Error::SchoolUnavailable)
    ));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
