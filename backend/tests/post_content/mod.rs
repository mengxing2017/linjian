use super::*;
use linjian_api::{
    business::SessionKind,
    community::domain::{PostInput, hot_posts::HotPostsQuery, public_post},
};

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn optional_titles_preserve_schema_evidence_and_publication_boundaries() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_post_content_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap("root", "内容管理员", "test-password-at-least-12", true)
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
    let school: Uuid = app.schools.create(&actor, "无标题测试校").await.unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let input: PostInput = serde_json::from_value(
        json!({"title":"  ","content":" 正文第一行\n👩‍💻第二行 ","images":[],"tags":[]}),
    )
    .unwrap();
    let created = app
        .community
        .management_write(&actor, school, None, input)
        .await
        .unwrap();
    assert_eq!(created.post.title, "");
    assert_eq!(created.post.content, "正文第一行\n👩‍💻第二行");
    assert_eq!(created.post.display_label, "正文第一行 👩‍💻第二行");
    let id = created.post.id;
    approve_post(&pool, &actor, id).await;
    let snapshot: Value = sqlx::query_scalar(
        "SELECT to_jsonb(s) FROM forum_publication_snapshots s WHERE post_id=$1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(snapshot["title"], "");
    assert_eq!(snapshot["content"], "正文第一行\n👩‍💻第二行");
    assert!(snapshot.get("display_label").is_none());
    let operations = linjian_api::operations::north::local::Service(
        linjian_api::operations::south::adapter::postgres::Repository(
            pool.clone(),
            Default::default(),
        ),
    );
    let options = operations
        .execute(
            &actor,
            Some(school),
            linjian_api::operations::south::port::Command::PostOptions {
                context: "operation.banner.create".into(),
                page: linjian_api::business::Page {
                    search: Some("正文第一行".into()),
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(options[0]["id"], id.to_string());
    assert_eq!(options[0]["display_label"], "正文第一行 👩‍💻第二行");
    assert!(options[0].get("title").is_none());
    assert!(options[0].get("content").is_none());
    sqlx::query("INSERT INTO comments(id,school_id,post_id,author_id,content,audit_status) SELECT gen_random_uuid(),$1,$2,$3,'热榜评论','approved' FROM generate_series(1,3)").bind(school).bind(id).bind(root).execute(&pool).await.unwrap();
    let hot = app
        .community
        .hot_posts(&actor, school, HotPostsQuery::default())
        .await
        .unwrap();
    assert_eq!(hot.items[0].id, id);
    assert_eq!(hot.items[0].display_label, "正文第一行 👩‍💻第二行");
    let document = serde_json::to_value(&hot.items[0]).unwrap();
    assert!(document.get("title").is_none());
    assert!(document.get("content").is_none());
    let comments = app
        .community
        .0
        .comments(&actor, school, Some(id), Default::default())
        .await
        .unwrap();
    assert_eq!(comments[0]["post_display_label"], "正文第一行 👩‍💻第二行");
    assert!(comments[0].get("_post").is_none());
    let current = app
        .community
        .management_detail(&actor, school, id)
        .await
        .unwrap();
    let intents: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox WHERE post_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let unchanged = app.community.management_write(&actor, school, Some(id), serde_json::from_value(json!({"title":" \n ","content":" 正文第一行\n👩‍💻第二行 ","images":[],"tags":[],"revision":current.post.revision})).unwrap()).await.unwrap();
    assert_eq!(unchanged.post.revision, current.post.revision);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox WHERE post_id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, intents);
    let titled = app.community.management_write(&actor, school, Some(id), serde_json::from_value(json!({"title":"主动添加标题","content":current.post.content,"images":[],"tags":[],"revision":current.post.revision})).unwrap()).await.unwrap();
    assert_eq!(titled.post.audit_status, "pending");
    assert!(titled.post.content_revision > current.post.content_revision);
    let cleared = app.community.management_write(&actor, school, Some(id), serde_json::from_value(json!({"title":"","content":titled.post.content,"images":[],"tags":[],"revision":titled.post.revision})).unwrap()).await.unwrap();
    assert_eq!(cleared.post.title, "");
    assert_eq!(cleared.post.display_label, current.post.display_label);
    assert!(
        app.community
            .management_write(
                &actor,
                school,
                Some(id),
                serde_json::from_value(
                    json!({"title":"旧修订","content":"正文","revision":current.post.revision})
                )
                .unwrap()
            )
            .await
            .is_err()
    );
    assert!(
        app.community
            .management_write(
                &actor,
                school,
                None,
                serde_json::from_value(json!({"title":"仅标题","content":"\u{200b}"})).unwrap()
            )
            .await
            .is_err()
    );
    let raw: Value = sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        public_post::project(raw).unwrap()["display_label"],
        current.post.display_label
    );
    let preserved: Value =
        sqlx::query_scalar("SELECT to_jsonb(s) FROM forum_publication_snapshots s WHERE id=$1")
            .bind(snapshot["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(preserved, snapshot);
    let preview = app
        .community
        .preview_post_import(
            &actor,
            school,
            "posts.csv".into(),
            "正文\n导入无标题正文\n".as_bytes().to_vec(),
        )
        .await
        .unwrap();
    assert!(preview.errors.is_empty());
    assert_eq!(preview.row_labels, vec!["导入无标题正文"]);
    let stored: Value = sqlx::query_scalar("SELECT rows FROM post_import_jobs WHERE id=$1")
        .bind(preview.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(stored[0].get("display_label").is_none());
    assert!(stored[0]["post"].get("display_label").is_none());
    let digest = preview.digest.clone();
    database::migrate(&pool).await.unwrap();
    let restored = app
        .community
        .post_import_detail(&actor, school, preview.id)
        .await
        .unwrap();
    assert_eq!(restored.digest, digest);
    let committed = app.community.commit_post_import(&actor, school, preview.id, serde_json::from_value(json!({"revision":preview.revision,"digest":digest,"idempotency_key":Uuid::new_v4()})).unwrap()).await.unwrap();
    assert_eq!(committed.result.len(), 1);
    let title: String = sqlx::query_scalar("SELECT title FROM posts WHERE id=$1")
        .bind(committed.result[0])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "");
    sqlx::query("UPDATE posts SET visibility='deleted' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let comments = app
        .community
        .0
        .comments(&actor, school, Some(id), Default::default())
        .await
        .unwrap();
    assert_eq!(comments.len(), 3);
    assert!(
        comments
            .iter()
            .all(|comment| comment["post_display_label"].is_null())
    );
    assert!(
        comments
            .iter()
            .all(|comment| comment.get("_post").is_none())
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
