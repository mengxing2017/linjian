use super::*;
use linjian_api::business::{Error, SessionKind};

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn snapshots_enforce_school_scope_and_are_read_only_with_idempotent_reactions() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("forum_interactions_{}", Uuid::new_v4().simple());
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
    let school: Uuid = app.schools.create(&actor, "互动测试校").await.unwrap()["id"]
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
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'评论用户')")
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
    let created = app
        .community
        .post(
            &actor,
            school,
            None,
            serde_json::from_value(json!({"title":"帖子","content":"正文","images":[],"tags":[]}))
                .unwrap(),
        )
        .await
        .unwrap();
    let post: Uuid = created["id"].as_str().unwrap().parse().unwrap();
    approve_post(&pool, &actor, post).await;
    let parent = Uuid::new_v4();
    sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,content,audit_status) VALUES($1,$2,$3,$4,'已通过','approved')").bind(parent).bind(school).bind(post).bind(user).execute(&pool).await.unwrap();
    for (audit, visibility, deleted, reply) in [
        ("approved", "visible", false, Some(parent)),
        ("pending", "visible", false, None),
        ("rejected", "visible", false, None),
        ("approved", "hidden", false, None),
        ("approved", "visible", true, None),
    ] {
        sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,parent_id,content,audit_status,visibility,deleted) VALUES($1,$2,$3,$4,$5,'测试评论',$6,$7,$8)").bind(Uuid::new_v4()).bind(school).bind(post).bind(user).bind(reply).bind(audit).bind(visibility).bind(deleted).execute(&pool).await.unwrap();
    }
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(post)
        .fetch_one(&pool)
        .await
        .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    for _ in 0..2 {
        let state = app
            .community
            .interaction_state(&member, school, post)
            .await
            .unwrap();
        assert_eq!(state.like_count, 0);
        assert_eq!(state.comment_count, 2);
        assert!(state.can_interact && state.can_share);
    }
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(post)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert!(
        app.community
            .interaction_state(&member, other, post)
            .await
            .is_err()
    );
    for _ in 0..2 {
        app.community
            .react(&member, school, post, "like", true)
            .await
            .unwrap();
    }
    let state = app
        .community
        .interaction_state(&member, school, post)
        .await
        .unwrap();
    assert_eq!(state.like_count, 1);
    assert!(state.liked);
    let notifications: i64 =
        sqlx::query_scalar("SELECT count(*) FROM notifications WHERE kind='like' AND object_id=$1")
            .bind(post)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(notifications, 1);
    for _ in 0..2 {
        app.community
            .react(&member, school, post, "like", false)
            .await
            .unwrap();
    }
    assert_eq!(
        app.community
            .interaction_state(&member, school, post)
            .await
            .unwrap()
            .like_count,
        0
    );
    app.community
        .react(&member, school, post, "like", true)
        .await
        .unwrap();
    let notifications: i64 =
        sqlx::query_scalar("SELECT count(*) FROM notifications WHERE kind='like' AND object_id=$1")
            .bind(post)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(notifications, 2);
    let router = linjian_api::http::app_router(app.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/schools/{school}/posts/{post}/interaction-state"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let document: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(document["data"]["type"], "post-interaction-states");
    assert_eq!(document["data"]["id"], post.to_string());
    assert_eq!(document["data"]["attributes"]["comment_count"], 2);
    assert!(
        document["data"]["attributes"]
            .get("active_member")
            .is_none()
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/schools/{school}/posts/{post}/interaction-state"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let own = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts(id,school_id,author_id,title,content) VALUES($1,$2,$3,'待审','正文')",
    )
    .bind(own)
    .bind(school)
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    let state = app
        .community
        .interaction_state(&member, school, own)
        .await
        .unwrap();
    assert!(!state.can_interact && !state.can_share);
    let hidden_parent = Uuid::new_v4();
    sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,content,audit_status,visibility) VALUES($1,$2,$3,$4,'隐藏评论','approved','hidden')").bind(hidden_parent).bind(school).bind(post).bind(user).execute(&pool).await.unwrap();
    assert!(matches!(
        app.community
            .comment(
                &member,
                school,
                post,
                serde_json::from_value(json!({"content":"回复","parent_id":hidden_parent}))
                    .unwrap()
            )
            .await,
        Err(Error::NotFound)
    ));
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("UPDATE posts SET visibility='hidden' WHERE id=$1")
        .bind(post)
        .execute(&mut *transaction)
        .await
        .unwrap();
    let community = app.community.clone();
    let concurrent_member = member.clone();
    let pending = tokio::spawn(async move {
        community
            .react(&concurrent_member, school, post, "like", true)
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    transaction.commit().await.unwrap();
    assert!(matches!(pending.await.unwrap(), Err(Error::NotFound)));
    assert!(matches!(
        app.community.interaction_state(&member, school, post).await,
        Err(Error::NotFound)
    ));
    let outbox_after: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_after, outbox);
    sqlx::query("UPDATE miniapp_users SET status='banned' WHERE account_id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !app.community
            .interaction_state(&member, school, own)
            .await
            .unwrap()
            .can_share
    );
    assert!(matches!(
        app.community
            .react(&member, school, own, "like", true)
            .await,
        Err(Error::MiniappBanned)
    ));
    sqlx::query(
        "UPDATE school_memberships SET role='revoked' WHERE school_id=$1 AND account_id=$2",
    )
    .bind(school)
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        app.community.interaction_state(&member, school, own).await,
        Err(Error::NotFound)
    ));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
