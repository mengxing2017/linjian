use super::*;
use linjian_api::{
    business::{Actor, SessionKind},
    community::domain::post_seed::*,
};

fn preview_input() -> PreviewInput {
    serde_json::from_value(json!({"name":"校园仿真发帖","authors":[{"key":"one","nickname":"仿真作者","avatar_id":2}],"rows":[
        {"author_key":"one","title":"校园生活","content":"欢迎分享校园生活","tags":[],"display_published_at":"2026-01-02T09:00:00+08:00"},
        {"author_key":"one","title":"食堂推荐","content":"交流喜欢的食堂菜品","tags":[],"display_published_at":"2026-01-01T09:00:00+08:00"}
    ]})).unwrap()
}
fn commit_input(batch: &Value, key: Uuid) -> CommitInput {
    CommitInput {
        revision: batch["revision"].as_i64().unwrap(),
        digest: batch["preview_digest"].as_str().unwrap().into(),
        idempotency_key: key,
    }
}
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn ordinary_own_posts_expose_only_server_formatted_publication_time() {
    let (pool, app, actor, school) = setup("publication_time").await;
    let member_id = app
        .identity
        .bootstrap(
            "publication-member",
            "普通用户",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, member_id, "member")
        .await
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'普通用户')")
        .bind(member_id)
        .execute(&pool)
        .await
        .unwrap();
    phone_bindings_tests::verify_fixture_phone(&pool, member_id).await;
    let (token, _) = app
        .identity
        .0
        .session(member_id, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let router = linjian_api::http::app_router(app.clone());
    let response = router.clone().oneshot(Request::builder().method("POST")
        .uri(format!("/api/v1/schools/{school}/posts"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/vnd.api+json")
        .body(Body::from(json!({"data":{"type":"posts","attributes":{"title":"我的校园生活","content":"普通用户自己发帖","images":[],"tags":[]}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    let post_id: Uuid = created["data"]["id"].as_str().unwrap().parse().unwrap();
    let attributes = &created["data"]["attributes"];
    assert!(attributes.get("created_at").is_none());
    assert!(attributes.get("updated_at").is_none());
    let (created_at, published_at): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT created_at,display_published_at FROM posts WHERE id=$1")
            .bind(post_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(created_at, published_at);
    let expected_text = published_at
        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M")
        .to_string();
    assert_eq!(attributes["published_at_text"], expected_text);
    let response = router.clone().oneshot(Request::builder().method("PUT")
        .uri(format!("/api/v1/schools/{school}/posts/{post_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/vnd.api+json")
        .body(Body::from(json!({"data":{"type":"posts","attributes":{"title":"我的校园生活","content":"编辑后的正文","images":[],"tags":[],"revision":attributes["revision"]}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let edited: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(
        edited["data"]["attributes"]["published_at_text"],
        expected_text
    );
    assert!(edited["data"]["attributes"].get("created_at").is_none());
    for path in [
        format!("/api/v1/schools/{school}/posts?mine=true"),
        format!("/api/v1/schools/{school}/posts/{post_id}"),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let document: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        let item = if document["data"].is_array() {
            &document["data"][0]
        } else {
            &document["data"]
        };
        assert_eq!(item["attributes"]["published_at_text"], expected_text);
        assert!(item["attributes"].get("created_at").is_none());
        assert!(item["attributes"].get("updated_at").is_none());
    }
}
async fn setup(name: &str) -> (PgPool, linjian_api::bootstrap::Application, Actor, Uuid) {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated URL required"),
    )
    .await
    .unwrap();
    let pool = isolated(
        &admin,
        &format!("seed_{}_{}", name, Uuid::new_v4().simple()),
    )
    .await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let id = app
        .identity
        .bootstrap("seed-root", "平台管理", "test-password-at-least-12", true)
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(id, SessionKind::AdminBrowser, Some("http://localhost:5173"))
        .await
        .unwrap();
    let school = app
        .schools
        .create(&actor, "仿真发帖测试校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    (pool, app, actor, school)
}
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn seed_batches_are_atomic_scoped_idempotent_and_revocable() {
    let (pool, app, actor, school) = setup("manual").await;
    let catalog = app
        .post_seeds
        .read(&actor, school, "avatars", None, SeedQuery::default())
        .await
        .unwrap();
    assert_eq!(catalog.as_array().unwrap().len(), 30);
    assert_eq!(catalog[29]["slot"], 29);
    assert_eq!(catalog[29]["version"], AVATAR_VERSION);
    let mut input = preview_input();
    input.authors[0].avatar_id = Some(29);
    let preview = app.post_seeds.preview(&actor, school, input).await.unwrap();
    assert_eq!(preview["preview_payload"]["authors"][0]["avatar_id"], 29);
    let preview_authors = app
        .post_seeds
        .read(&actor, school, "authors", None, SeedQuery::default())
        .await
        .unwrap();
    assert!(preview_authors.as_array().unwrap().is_empty());
    let batch = preview["id"].as_str().unwrap().parse().unwrap();
    let expired_batch = Uuid::new_v4();
    sqlx::query("INSERT INTO post_seed_batches(id,school_id,created_by,name,preview_digest,policy_digest,preview_payload,expires_at) SELECT $1,school_id,created_by,name,preview_digest,policy_digest,preview_payload,now()-interval '1 minute' FROM post_seed_batches WHERE id=$2")
        .bind(expired_batch).bind(batch).execute(&pool).await.unwrap();
    for _attempt in 0..100 {
        app.media.0.cleanup().await.unwrap();
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM post_seed_batches WHERE id=$1)")
                .bind(expired_batch)
                .fetch_one(&pool)
                .await
                .unwrap();
        if !exists {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM post_seed_batches WHERE id=$1)"
        )
        .bind(expired_batch)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    assert!(
        app.post_seeds
            .read(&actor, school, "detail", Some(batch), SeedQuery::default())
            .await
            .is_ok()
    );
    assert!(
        preview["preview_payload"]["errors"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM accounts WHERE account_kind='simulated'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let key = Uuid::new_v4();
    let (first, second) = tokio::join!(
        app.post_seeds
            .commit(&actor, school, batch, commit_input(&preview, key)),
        app.post_seeds
            .commit(&actor, school, batch, commit_input(&preview, key))
    );
    assert_eq!(first.unwrap(), second.unwrap());
    assert!(
        app.post_seeds
            .commit(
                &actor,
                school,
                batch,
                commit_input(&preview, Uuid::new_v4())
            )
            .await
            .is_err()
    );
    let rows: Vec<Value> =
        sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE seed_batch_id=$1 ORDER BY id")
            .bind(batch)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["audit_status"], "pending");
    assert_ne!(rows[0]["author_id"], json!(actor.id));
    let author: Uuid = rows[0]["author_id"].as_str().unwrap().parse().unwrap();
    let matches = app
        .post_seeds
        .read(
            &actor,
            school,
            "authors",
            None,
            SeedQuery {
                search: Some("仿真".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(matches.as_array().unwrap().len(), 1);
    assert_eq!(matches[0]["id"], author.to_string());
    assert_eq!(matches[0]["nickname"], "仿真作者");
    let avatar_id: Uuid =
        sqlx::query_scalar("SELECT avatar_asset_id FROM miniapp_users WHERE account_id=$1")
            .bind(author)
            .fetch_one(&pool)
            .await
            .unwrap();
    let (mime, bytes) = app.media.0.read(avatar_id).await.unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(
        bytes,
        linjian_api::community::south::adapter::postgres::post_seed::avatar_png(29).unwrap()
    );
    assert!(
        app.identity
            .0
            .session(author, SessionKind::Miniapp, None)
            .await
            .is_err()
    );
    assert!(
        app.identity
            .0
            .session(author, SessionKind::AdminBrowser, None)
            .await
            .is_err()
    );
    for statement in [
        "INSERT INTO miniapp_identities(app_id,openid,account_id) VALUES('seed-test','bad',$1)",
        "INSERT INTO school_memberships(school_id,account_id,role) SELECT bound_school_id,id,'admin' FROM accounts WHERE id=$1",
        "UPDATE accounts SET password_hash='bad' WHERE id=$1",
        "UPDATE accounts SET account_kind='ordinary' WHERE id=$1",
    ] {
        assert!(
            sqlx::query(statement)
                .bind(author)
                .execute(&pool)
                .await
                .is_err(),
            "{statement}"
        );
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statistics_registrations WHERE account_id=$1")
            .bind(author)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let post: Uuid = rows[0]["id"].as_str().unwrap().parse().unwrap();
    let approved = approve_post(&pool, &actor, post).await;
    assert_eq!(approved["audit_status"], "approved");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM forum_publication_snapshots WHERE post_id=$1")
            .bind(post)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    for table in [
        "forum_outbox",
        "forum_source_versions",
        "forum_publication_preparations",
        "forum_post_recommendations",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    let view = app.community.0.detail(&actor, school, post).await.unwrap();
    assert!(
        view["published_at_text"]
            .as_str()
            .unwrap()
            .starts_with("2026-01-0")
    );
    assert!(view.get("created_at").is_none());
    assert!(view.get("updated_at").is_none());
    assert!(view.get("is_initial_content").is_none());
    let managed = app
        .community
        .management_list(&actor, school, Default::default())
        .await
        .unwrap();
    assert_eq!(managed.items[0].post.title, "校园生活");
    let order=serde_json::from_value(json!({"order_revision":managed.order_revision,"is_top":false,"page":1,"page_size":30,"original":managed.items.iter().map(|item|json!({"id":item.post.id,"revision":item.post.revision})).collect::<Vec<_>>(),"ordered_ids":managed.items.iter().rev().map(|item|item.post.id).collect::<Vec<_>>()})).unwrap();
    app.community
        .management_reorder(&actor, school, order)
        .await
        .unwrap();
    let reordered = app
        .community
        .management_list(&actor, school, Default::default())
        .await
        .unwrap();
    assert_eq!(reordered.items[0].post.title, "食堂推荐");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    for field in [
        "seed_batch_id",
        "created_by",
        "approved_snapshot_id",
        "source_projection",
        "created_via",
    ] {
        assert!(view.get(field).is_none(), "{field}");
    }
    let recommendation = app
        .community
        .recommendation_preview(&actor, school, post)
        .await
        .unwrap();
    assert_eq!(
        recommendation["unavailable_reason"],
        "seed_content_not_recommendable"
    );
    assert!(
        sqlx::query(
            "SELECT forum_create_source_intent(p,'manual_recommendation') FROM posts p WHERE id=$1"
        )
        .bind(post)
        .execute(&pool)
        .await
        .is_err()
    );
    let other: Uuid = app
        .schools
        .create(&actor, "其他学校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other_authors = app
        .post_seeds
        .read(&actor, other, "authors", None, SeedQuery::default())
        .await
        .unwrap();
    assert!(other_authors.as_array().unwrap().is_empty());
    assert!(
        app.post_seeds
            .read(&actor, other, "batches", Some(batch), SeedQuery::default())
            .await
            .is_err()
    );
    let revoked = app
        .post_seeds
        .revoke(
            &actor,
            school,
            batch,
            RevokeInput {
                revision: 2,
                reason: "测试撤销".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(revoked["state"], "revoked");
    assert_eq!(
        app.post_seeds
            .revoke(
                &actor,
                school,
                batch,
                RevokeInput {
                    revision: 2,
                    reason: "重复撤销".into()
                }
            )
            .await
            .unwrap(),
        revoked
    );
    assert!(
        sqlx::query("UPDATE posts SET visibility='visible' WHERE id=$1")
            .bind(post)
            .execute(&pool)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM posts WHERE seed_batch_id=$1 AND visibility='offline'",
    )
    .bind(batch)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 2);
    let avatar_count:i64=sqlx::query_scalar("SELECT count(*) FROM file_asset_references WHERE object_type='miniapp_users' AND object_id=$1").bind(author).fetch_one(&pool).await.unwrap();
    assert_eq!(avatar_count, 1);
    let snapshot = migration::export(&pool, &app.media.0.1).await.unwrap();
    let admin = PgPool::connect(&std::env::var("LINJIAN_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let target = isolated(&admin, &format!("seed_restore_{}", Uuid::new_v4().simple())).await;
    database::migrate(&target).await.unwrap();
    let directory = std::env::temp_dir().join(format!("linjian-seed-restore-{}", Uuid::new_v4()));
    let files = linjian_api::media::south::adapter::filesystem::Files::open(
        linjian_api::media::config::Config::local(directory.clone()),
    )
    .unwrap();
    migration::import(&target, &snapshot, false, &files)
        .await
        .unwrap();
    let restored = migration::export(&target, &files).await.unwrap();
    assert_eq!(snapshot.digest, restored.digest);
    assert_eq!(snapshot.files, restored.files);
    std::fs::remove_dir_all(directory).unwrap();
}
#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn seed_auto_review_revalidates_policy_content_and_permissions() {
    let (pool, app, actor, school) = setup("auto").await;
    let preview = app
        .post_seeds
        .preview(&actor, school, preview_input())
        .await
        .unwrap();
    let batch = preview["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE schools SET config=jsonb_set(config,'{post_audit_mode}','\"auto\"'),revision=revision+1 WHERE id=$1").bind(school).execute(&pool).await.unwrap();
    assert!(
        app.post_seeds
            .commit(
                &actor,
                school,
                batch,
                commit_input(&preview, Uuid::new_v4())
            )
            .await
            .is_err()
    );
    let preview = app
        .post_seeds
        .preview(&actor, school, preview_input())
        .await
        .unwrap();
    let batch = preview["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(preview["preview_payload"]["audit_mode"], "auto");
    let result = app
        .post_seeds
        .commit(
            &actor,
            school,
            batch,
            commit_input(&preview, Uuid::new_v4()),
        )
        .await
        .unwrap();
    assert_eq!(result["post_count"], 2);
    let feed = app
        .community
        .0
        .posts(&actor, school, linjian_api::business::Page::default())
        .await
        .unwrap();
    assert_eq!(feed[0]["title"], "校园生活");
    let windows = json!([{"key":"seed","start_at":(chrono::Utc::now()-chrono::Duration::days(1)).to_rfc3339(),"actual_end_at":(chrono::Utc::now()+chrono::Duration::minutes(1)).to_rfc3339(),"school_ids":[school],"full_site":true,"content":true,"registration":true,"activity":true,"cohort":true,"detail":true}]);
    let metrics: Value = sqlx::query_scalar(include_str!(
        "../../src/statistics/south/adapter/metrics.sql"
    ))
    .bind(windows)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(metrics["metrics"]["posts"], 2);
    assert_eq!(metrics["metrics"]["seed_posts"], 2);
    assert_eq!(metrics["metrics"]["post_authors"], 0);
    assert_eq!(metrics["metrics"]["participants"], 0);
    assert_eq!(metrics["metrics"]["registrations"], 0);
    let approved:i64=sqlx::query_scalar("SELECT count(*) FROM posts WHERE seed_batch_id=$1 AND audit_status='approved' AND approved_snapshot_id IS NOT NULL").bind(batch).fetch_one(&pool).await.unwrap();
    assert_eq!(approved, 2);
    let automatic:i64=sqlx::query_scalar("SELECT count(*) FROM moderation_events WHERE decision_source='automatic' AND actor_id IS NULL").fetch_one(&pool).await.unwrap();
    assert_eq!(automatic, 2);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let mut bad = preview_input();
    let author_id = result["preview_payload"]["authors"][0]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let mut reuse = preview_input();
    reuse.authors[0].existing_id = Some(author_id);
    let reused = app.post_seeds.preview(&actor, school, reuse).await.unwrap();
    let reused_batch = reused["id"].as_str().unwrap().parse().unwrap();
    app.post_seeds
        .commit(
            &actor,
            school,
            reused_batch,
            commit_input(&reused, Uuid::new_v4()),
        )
        .await
        .unwrap();
    app.post_seeds
        .read(
            &actor,
            school,
            "author-avatar",
            Some(author_id),
            SeedQuery::default(),
        )
        .await
        .unwrap();
    app.post_seeds
        .revoke(
            &actor,
            school,
            batch,
            RevokeInput {
                revision: 2,
                reason: "保留其他批次".into(),
            },
        )
        .await
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM accounts WHERE account_kind='simulated'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM posts WHERE seed_batch_id=$1 AND visibility='visible'",
    )
    .bind(reused_batch)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 2);
    bad.rows[1].title = " \n ".into();
    let expected_content = bad.rows[1].content.clone();
    bad.rows[1].content = format!(" \n{expected_content}\n ");
    let untitled = app
        .post_seeds
        .preview(&actor, school, bad.clone())
        .await
        .unwrap();
    assert!(
        untitled["preview_payload"]["errors"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(untitled["preview_payload"]["rows"][1]["title"], "");
    assert_eq!(
        untitled["preview_payload"]["rows"][1]["content"],
        expected_content
    );
    assert_eq!(untitled["row_labels"][1], expected_content);
    bad.rows[1].content = String::new();
    let invalid = app.post_seeds.preview(&actor, school, bad).await.unwrap();
    assert_eq!(invalid["preview_payload"]["errors"][0]["row"], 2);
    assert!(
        app.post_seeds
            .commit(
                &actor,
                school,
                invalid["id"].as_str().unwrap().parse().unwrap(),
                commit_input(&invalid, Uuid::new_v4())
            )
            .await
            .is_err()
    );
    let user = app
        .identity
        .bootstrap("seed-operator", "运营", "test-password-at-least-12", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, user, "operator")
        .await
        .unwrap();
    let (_, operator) = app
        .identity
        .0
        .session(
            user,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let operator_preview = app
        .post_seeds
        .preview(&operator, school, preview_input())
        .await
        .unwrap();
    let operator_batch = operator_preview["id"].as_str().unwrap().parse().unwrap();
    let operator_result = app
        .post_seeds
        .commit(
            &operator,
            school,
            operator_batch,
            commit_input(&operator_preview, Uuid::new_v4()),
        )
        .await
        .unwrap();
    assert_eq!(operator_result["post_count"], 2);
    sqlx::query("INSERT INTO school_member_permission_overrides(school_id,account_id,permission_key,effect,updated_by) VALUES($1,$2,'content.simulation.manage','deny',$3)")
        .bind(school).bind(user).bind(actor.id).execute(&pool).await.unwrap();
    assert!(
        app.post_seeds
            .preview(&operator, school, preview_input())
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn seed_failure_rolls_back_business_data_and_http_requires_csrf() {
    let (pool, app, actor, school) = setup("rollback").await;
    let preview = app
        .post_seeds
        .preview(&actor, school, preview_input())
        .await
        .unwrap();
    let batch = preview["id"].as_str().unwrap().parse().unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_seed_second() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.title='食堂推荐' THEN RAISE EXCEPTION 'test failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_seed_second BEFORE INSERT ON posts FOR EACH ROW EXECUTE FUNCTION reject_seed_second();").execute(&pool).await.unwrap();
    let key = Uuid::new_v4();
    assert!(
        app.post_seeds
            .commit(&actor, school, batch, commit_input(&preview, key))
            .await
            .is_err()
    );
    for table in [
        "posts",
        "post_seed_items",
        "miniapp_users",
        "moderation_events",
        "forum_outbox",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM accounts WHERE account_kind='simulated'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM file_assets WHERE owner_id=$1 AND state='uploading' AND NOT ever_bound AND expires_at IS NOT NULL").bind(actor.id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    sqlx::raw_sql("DROP TRIGGER reject_seed_second ON posts; DROP FUNCTION reject_seed_second();")
        .execute(&pool)
        .await
        .unwrap();
    app.post_seeds
        .commit(&actor, school, batch, commit_input(&preview, key))
        .await
        .unwrap();
    let (token, http_actor) = app
        .identity
        .0
        .session(
            actor.id,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let router = linjian_api::http::app_router(app.clone());
    for (origin, csrf, expected) in [
        (None, false, StatusCode::FORBIDDEN),
        (Some("https://wrong.example"), true, StatusCode::FORBIDDEN),
        (Some("http://localhost:5173"), false, StatusCode::FORBIDDEN),
        (Some("http://localhost:5173"), true, StatusCode::OK),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/schools/{school}/post-seed-batches/preview"
            ))
            .header("Cookie", format!("linjian_session={token}"))
            .header("Content-Type", "application/vnd.api+json");
        if let Some(origin) = origin {
            request = request.header("Origin", origin);
        }
        if csrf {
            request = request.header("X-CSRF-Token", &http_actor.csrf);
        }
        let response = router
            .clone()
            .oneshot(
                request
                    .body(Body::from(
                        json!({"data":{"type":"post-seed-batches","attributes":preview_input()}})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    let admin = app
        .identity
        .bootstrap("seed-admin", "学校管理", "test-password-at-least-12", false)
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, admin, "admin")
        .await
        .unwrap();
    let (_, school_actor) = app
        .identity
        .0
        .session(
            admin,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let draft = app
        .post_seeds
        .preview(&school_actor, school, preview_input())
        .await
        .unwrap();
    sqlx::query("INSERT INTO school_member_permission_overrides(school_id,account_id,permission_key,effect,updated_by) VALUES($1,$2,'content.simulation.manage','deny',$3)").bind(school).bind(admin).bind(actor.id).execute(&pool).await.unwrap();
    assert!(
        app.post_seeds
            .commit(
                &school_actor,
                school,
                draft["id"].as_str().unwrap().parse().unwrap(),
                commit_input(&draft, Uuid::new_v4())
            )
            .await
            .is_err()
    );
}
