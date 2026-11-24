use super::*;
use linjian_api::business::{Actor, Error, Page, SessionKind};

async fn create_post(
    app: &bootstrap::Application,
    pool: &PgPool,
    actor: &Actor,
    school: Uuid,
    title: &str,
    content: &str,
    approved: bool,
) -> Uuid {
    let value = app
        .community
        .post(
            actor,
            school,
            None,
            serde_json::from_value(json!({"title":title,"content":content,"images":[],"tags":[]}))
                .unwrap(),
        )
        .await
        .unwrap();
    let id = value["id"].as_str().unwrap().parse().unwrap();
    if approved {
        approve_post(pool, actor, id).await;
    }
    id
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn post_search_literals_scope_visibility_pagination_and_read_only_boundaries() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_search_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
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
        .create(&actor, "搜索测试校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app
        .schools
        .create(&actor, "另一学校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let reader = app
        .identity
        .bootstrap("reader", "读者", "test-password-at-least-12", false)
        .await
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'搜索读者')")
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, reader, "member")
        .await
        .unwrap();
    let (token, member) = app
        .identity
        .0
        .session(reader, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let titled = create_post(
        &app,
        &pool,
        &actor,
        school,
        "scope 课本",
        "仅标题命中",
        true,
    )
    .await;
    let untitled = create_post(
        &app,
        &pool,
        &actor,
        school,
        "",
        "scope 正文课本 100% !_\\ AbC",
        true,
    )
    .await;
    let third = create_post(&app, &pool, &actor, school, "scope 普通", "普通帖子", true).await;
    let pending = create_post(&app, &pool, &actor, school, "scope 待审", "课本", false).await;
    let own = Uuid::new_v4();
    sqlx::query("INSERT INTO posts(id,school_id,author_id,title,content) VALUES($1,$2,$3,'scope 自己待审','课本')")
        .bind(own)
        .bind(school)
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    let hidden = create_post(&app, &pool, &actor, school, "scope 隐藏", "课本", true).await;
    let deleted = create_post(&app, &pool, &actor, school, "scope 删除", "课本", true).await;
    sqlx::query("UPDATE posts SET visibility='hidden' WHERE id=$1")
        .bind(hidden)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE posts SET visibility='deleted' WHERE id=$1")
        .bind(deleted)
        .execute(&pool)
        .await
        .unwrap();
    create_post(&app, &pool, &actor, other, "scope 他校", "课本", true).await;
    sqlx::query("UPDATE posts SET is_top=true WHERE id=$1")
        .bind(titled)
        .execute(&pool)
        .await
        .unwrap();
    let before: Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM posts p")
            .fetch_one(&pool)
            .await
            .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();

    for keyword in ["%", "_", "!", "\\", "abc", " 100% "] {
        let rows = app
            .community
            .posts(
                &member,
                school,
                Page {
                    search: Some(keyword.into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "keyword={keyword}");
        assert_eq!(rows[0]["id"], untitled.to_string());
    }
    let matches = app
        .community
        .posts(
            &member,
            school,
            Page {
                search: Some("课本".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0]["id"], titled.to_string());
    assert!(matches.iter().any(|item| item["id"] == own.to_string()));
    assert!(!matches.iter().any(|item| {
        [pending, hidden, deleted]
            .iter()
            .any(|id| item["id"] == id.to_string())
    }));
    let privileged = app
        .community
        .posts(&actor, school, Page::default())
        .await
        .unwrap();
    assert!(
        privileged
            .iter()
            .any(|item| item["id"] == hidden.to_string())
    );
    assert!(
        privileged
            .iter()
            .any(|item| item["id"] == pending.to_string())
    );
    assert!(
        !privileged
            .iter()
            .any(|item| item["id"] == deleted.to_string())
    );
    assert!(matches!(
        app.community.posts(&member, other, Page::default()).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        app.community
            .posts(
                &member,
                school,
                Page {
                    search: Some("😀".repeat(101)),
                    ..Default::default()
                }
            )
            .await,
        Err(Error::Invalid(_))
    ));

    let router = linjian_api::http::app_router(app.clone());
    let mut found = Vec::new();
    for number in [1, 2] {
        let response = router.clone().oneshot(Request::builder().uri(format!("/api/v1/schools/{school}/posts?search=scope&page[number]={number}&page[size]=2")).header("authorization", format!("Bearer {token}")).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let document: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
        assert_eq!(
            document["meta"]["page"],
            json!({"number":number,"size":2,"has_more":number==1})
        );
        let rows = document["data"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row["type"], "posts");
            assert_eq!(row["attributes"]["school_id"], school.to_string());
            assert!(row["attributes"]["published_at_text"].is_string());
            assert!(row["attributes"]["display_label"].is_string());
            assert!(!found.contains(&row["id"]));
            found.push(row["id"].clone());
        }
    }
    assert_eq!(found[0], titled.to_string());
    assert!(found.contains(&json!(third)));
    let after: Value = sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM posts p")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    let after_outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox, after_outbox);
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        app.community.posts(&member, school, Page::default()).await,
        Err(Error::SchoolUnavailable)
    ));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
