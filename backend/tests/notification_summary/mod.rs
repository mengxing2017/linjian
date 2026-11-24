use super::*;
use linjian_api::business::{Error, Page, SessionKind};

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn summary_matches_visible_unread_notifications_and_enforces_owner_scope() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("forum_notification_summary_{}", Uuid::new_v4().simple());
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
    let school: Uuid = app
        .schools
        .create(&actor, "通知统计校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app
        .schools
        .create(&actor, "通知隔离校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let user = app
        .identity
        .bootstrap("reader", "成员", "test-password-at-least-12", false)
        .await
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'通知成员')")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, user, "member")
        .await
        .unwrap();
    let (token, member) = app
        .identity
        .0
        .session(user, SessionKind::Miniapp, None)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        0
    );
    assert!(matches!(
        app.community.notification_summary(&member, other).await,
        Err(Error::Forbidden)
    ));

    for (owner, scope, count) in [(user, school, 25), (root, school, 3), (user, other, 5)] {
        for _ in 0..count {
            sqlx::query(
                "INSERT INTO notifications(id,school_id,account_id,kind) VALUES($1,$2,$3,'follow')",
            )
            .bind(Uuid::new_v4())
            .bind(scope)
            .bind(owner)
            .execute(&pool)
            .await
            .unwrap();
        }
    }
    let hidden_post = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts(id,school_id,author_id,title,content) VALUES($1,$2,$3,'隐藏','正文')",
    )
    .bind(hidden_post)
    .bind(school)
    .bind(root)
    .execute(&pool)
    .await
    .unwrap();
    approve_post(&pool, &actor, hidden_post).await;
    sqlx::query("UPDATE posts SET visibility='hidden' WHERE id=$1")
        .bind(hidden_post)
        .execute(&pool)
        .await
        .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    for kind in ["comment", "moderation"] {
        sqlx::query("INSERT INTO notifications(id,school_id,account_id,kind,object_id) VALUES($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4()).bind(school).bind(user).bind(kind).bind(hidden_post).execute(&pool).await.unwrap();
    }
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        25
    );
    let first_page = app
        .community
        .notifications(
            &member,
            school,
            Page {
                page_size: Some(10),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(first_page.len() < 25);
    let notification: Uuid = first_page[0]["id"].as_str().unwrap().parse().unwrap();
    app.community
        .read(&member, school, notification)
        .await
        .unwrap();
    app.community
        .read(&member, school, notification)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        24
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        outbox
    );
    sqlx::query("UPDATE posts SET visibility='visible' WHERE id=$1")
        .bind(hidden_post)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        26
    );

    let comment = Uuid::new_v4();
    sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,content,audit_status) VALUES($1,$2,$3,$4,'待审','pending')")
        .bind(comment).bind(school).bind(hidden_post).bind(root).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO notifications(id,school_id,account_id,kind,object_id) VALUES($1,$2,$3,'comment',$4)")
        .bind(Uuid::new_v4()).bind(school).bind(user).bind(comment).execute(&pool).await.unwrap();
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        26
    );
    sqlx::query("UPDATE comments SET audit_status='approved' WHERE id=$1")
        .bind(comment)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        27
    );
    let visible = app
        .community
        .notifications(
            &member,
            school,
            Page {
                page_size: Some(100),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        visible
            .iter()
            .filter(|row| row["read_at"].is_null())
            .count(),
        27
    );
    sqlx::query("UPDATE comments SET deleted=true WHERE id=$1")
        .bind(comment)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        26
    );

    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let router = linjian_api::http::app_router(app.clone());
    let uri = format!("/api/v1/schools/{school}/notification-summary");
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
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
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../../api-contract/fixtures/notification-summary.json"
    ))
    .unwrap();
    fixture["data"]["id"] = json!(school);
    fixture["data"]["attributes"]["school_id"] = json!(school);
    fixture["data"]["attributes"]["unread_count"] = json!(26);
    assert_eq!(document, fixture);
    let denied = router
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    for kind in ["like", "like", "future", "comment", "comment"] {
        sqlx::query("INSERT INTO notifications(id,school_id,account_id,kind,object_id,created_at) VALUES($1,$2,$3,$4,$5,'2020-01-01T00:00:00Z')")
            .bind(Uuid::new_v4()).bind(school).bind(user).bind(kind).bind(hidden_post).execute(&pool).await.unwrap();
    }
    sqlx::query("UPDATE notifications SET created_at=CASE WHEN kind='follow' THEN '2026-01-01T00:00:00Z'::timestamptz ELSE '2020-01-01T00:00:00Z'::timestamptz END")
        .execute(&pool)
        .await
        .unwrap();
    let all = app
        .community
        .notifications(
            &member,
            school,
            Page {
                page_size: Some(100),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(all.iter().take(20).all(|row| row["kind"] == "follow"));
    assert!(
        all.iter()
            .all(|row| row["school_id"] == json!(school) && row["account_id"] == json!(user))
    );
    assert!(all.iter().any(|row| row["kind"] == "future"));
    assert!(!all.iter().any(|row| row["object_id"] == json!(comment)));
    let test_router = linjian_api::http::app_router(app.clone());
    for kind in ["comment", "like", "follow", "moderation"] {
        let expected: Vec<_> = all.iter().filter(|row| row["kind"] == kind).collect();
        let filtered = app
            .community
            .notifications(
                &member,
                school,
                Page {
                    kind: Some(kind.into()),
                    page_size: Some(100),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(
            filtered,
            expected
                .iter()
                .map(|row| (*row).clone())
                .collect::<Vec<_>>()
        );
        for page in 1..=expected.len() + 1 {
            let response = test_router.clone().oneshot(Request::builder()
                .uri(format!("/api/v1/schools/{school}/notifications?page[number]={page}&page[size]=1&kind={kind}"))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty()).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let document: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap())
                    .unwrap();
            let data = document["data"].as_array().unwrap();
            if let Some(row) = expected.get(page - 1) {
                assert_eq!(data.len(), 1);
                assert_eq!(data[0]["type"], "notifications");
                assert_eq!(data[0]["id"], row["id"]);
                assert_eq!(data[0]["attributes"]["kind"], kind);
                assert_eq!(data[0]["attributes"]["school_id"], json!(school));
                assert_eq!(data[0]["attributes"]["account_id"], json!(user));
            } else {
                assert!(data.is_empty());
            }
            assert_eq!(
                document["meta"]["page"],
                json!({"number": page, "size": 1, "has_more": page < expected.len()})
            );
        }
    }
    let follow_ids: Vec<_> = all
        .iter()
        .filter(|row| row["kind"] == "follow")
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert!(follow_ids.windows(2).all(|pair| pair[0] > pair[1]));
    for invalid in ["", "%20", "Comment", "all", "unknown"] {
        let response = test_router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/v1/schools/{school}/notifications?kind={invalid}"
                    ))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let document: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap())
                .unwrap();
        assert_eq!(document["errors"][0]["code"], "validation");
    }
    assert!(matches!(
        app.community
            .notifications(
                &member,
                other,
                Page {
                    kind: Some("comment".into()),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Forbidden)
    ));
    let unread_before = app
        .community
        .notification_summary(&member, school)
        .await
        .unwrap()
        .unread_count;
    let foreign_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM notifications WHERE school_id=$1 AND account_id=$2 LIMIT 1",
    )
    .bind(school)
    .bind(root)
    .fetch_one(&pool)
    .await
    .unwrap();
    app.community
        .read(&member, school, foreign_id)
        .await
        .unwrap();
    let foreign_read: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>> =
        sqlx::query_scalar("SELECT read_at FROM notifications WHERE id=$1")
            .bind(foreign_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(foreign_read.is_none());
    assert_eq!(
        app.community
            .notification_summary(&member, school)
            .await
            .unwrap()
            .unread_count,
        unread_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        outbox
    );
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        app.community.notification_summary(&member, school).await,
        Err(Error::SchoolUnavailable)
    ));
    assert!(matches!(
        app.community
            .notifications(
                &member,
                school,
                Page {
                    kind: Some("comment".into()),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::SchoolUnavailable)
    ));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
