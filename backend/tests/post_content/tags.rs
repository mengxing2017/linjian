use super::*;
use linjian_api::{business::Error, schools::south::adapter::postgres::Repository};

fn input(tags: &[&str], ids: &[Uuid], revision: Option<i64>) -> PostInput {
    serde_json::from_value(json!({
        "title": "", "content": "标签选择测试", "images": [], "tags": tags,
        "candidate_tag_ids": ids, "revision": revision,
    }))
    .unwrap()
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn miniapp_tags_require_candidates_and_only_retain_locked_original_names() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("forum_post_tags_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap("root", "标签管理员", "test-password-at-least-12", true)
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
        .create(&actor, "标签测试校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let other: Uuid = app
        .schools
        .create(&actor, "其他学校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    app.schools
        .grant(&actor, school, root, "member")
        .await
        .unwrap();
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'标签测试成员')")
        .bind(root)
        .execute(&pool)
        .await
        .unwrap();
    phone_bindings_tests::verify_fixture_phone(&pool, root).await;
    let (_, mini) = app
        .identity
        .0
        .session(root, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let repository = Repository(pool.clone());
    let tag = repository
        .home_menu_save(
            &actor,
            school,
            None,
            serde_json::from_value(
                json!({"name":"校园","menu_status":"hot","enable_status":"enabled"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let tag_id: Uuid = tag["id"].as_str().unwrap().parse().unwrap();
    let foreign = repository
        .home_menu_save(
            &actor,
            other,
            None,
            serde_json::from_value(
                json!({"name":"外校","menu_status":"shown","enable_status":"enabled"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let foreign_id: Uuid = foreign["id"].as_str().unwrap().parse().unwrap();

    app.community
        .post(&mini, school, None, input(&[], &[], None))
        .await
        .unwrap();
    let created = app
        .community
        .post(&mini, school, None, input(&["校园"], &[tag_id], None))
        .await
        .unwrap();
    let id: Uuid = created["id"].as_str().unwrap().parse().unwrap();
    for invalid in [
        input(&["自定义"], &[], None),
        input(&["校园"], &[], None),
        input(&["校园", "额外自定义"], &[tag_id], None),
        input(&["校园", "校园"], &[tag_id], None),
        input(&["校园"], &[tag_id, tag_id], None),
        input(&["外校"], &[foreign_id], None),
        input(&["不匹配"], &[tag_id], None),
    ] {
        assert!(matches!(
            app.community.post(&mini, school, None, invalid).await,
            Err(Error::Invalid(_))
        ));
    }
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM posts WHERE school_id=$1")
        .bind(school)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE home_menu_tags SET enable_status='disabled' WHERE id=$1")
        .bind(tag_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        app.community
            .post(&mini, school, None, input(&["校园"], &[tag_id], None))
            .await,
        Err(Error::Invalid(_))
    ));
    let retained = app
        .community
        .post(&mini, school, Some(id), input(&["校园"], &[], Some(1)))
        .await
        .unwrap();
    assert_eq!(retained["tags"], json!(["校园"]));
    assert_eq!(retained["revision"], 1);
    assert!(matches!(
        app.community
            .post(&mini, school, Some(id), input(&["伪造历史"], &[], Some(1)))
            .await,
        Err(Error::Invalid(_))
    ));
    let removed = app
        .community
        .post(&mini, school, Some(id), input(&[], &[], Some(1)))
        .await
        .unwrap();
    let revision = removed["revision"].as_i64().unwrap();
    assert!(matches!(
        app.community
            .post(
                &mini,
                school,
                Some(id),
                input(&["校园"], &[], Some(revision))
            )
            .await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        app.community
            .post(&mini, school, Some(id), input(&[], &[], Some(1)))
            .await,
        Err(Error::Conflict)
    ));
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM posts WHERE school_id=$1")
        .bind(school)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);

    let legacy = app
        .community
        .management_write(&actor, school, None, input(&["后台历史"], &[], None))
        .await
        .unwrap();
    app.community
        .post(
            &mini,
            school,
            Some(legacy.post.id),
            input(&["后台历史"], &[], Some(legacy.post.revision)),
        )
        .await
        .unwrap();
    let admin_created = app
        .community
        .post(&actor, school, None, input(&["后台自定义"], &[], None))
        .await
        .unwrap();
    assert_eq!(admin_created["tags"], json!(["后台自定义"]));
    for change in [
        "menu_status='hidden'",
        "source_type='user'",
        "deleted_at=now()",
    ] {
        sqlx::query(&format!(
            "UPDATE home_menu_tags SET enable_status='enabled',{change} WHERE id=$1"
        ))
        .bind(tag_id)
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            app.community
                .post(&mini, school, None, input(&["校园"], &[tag_id], None))
                .await,
            Err(Error::Invalid(_))
        ));
        sqlx::query("UPDATE home_menu_tags SET menu_status='hot',source_type='admin',deleted_at=NULL WHERE id=$1").bind(tag_id).execute(&pool).await.unwrap();
    }
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
