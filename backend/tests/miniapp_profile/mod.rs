use super::*;
use linjian_api::{
    business::{Actor, Error, SessionKind},
    miniapp_users::{
        domain::{ProfileInput, registration::RegistrationSettingsInput},
        south::{
            adapter::postgres::{Repository, registration::NICKNAME_LOCK},
            port::{NicknameRandom, ProfileStore},
        },
    },
};

struct Sequence(std::collections::VecDeque<u32>);
impl NicknameRandom for Sequence {
    fn next_number(&mut self) -> linjian_api::business::Result<u32> {
        self.0.pop_front().ok_or(Error::Unavailable)
    }
}
fn sequence(values: &[u32]) -> Sequence {
    Sequence(values.iter().copied().collect())
}
fn settings(prefix: &str, revision: i64) -> RegistrationSettingsInput {
    RegistrationSettingsInput {
        nickname_prefix: prefix.into(),
        revision,
    }
}

struct Fixture {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    app: bootstrap::Application,
    actor: Actor,
    token: String,
}
impl Fixture {
    async fn new() -> Self {
        let admin = PgPool::connect(
            &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
        )
        .await
        .unwrap();
        let schema = format!("miniapp_profile_{}", Uuid::new_v4().simple());
        let pool = isolated(&admin, &schema).await;
        database::migrate(&pool).await.unwrap();
        let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
        let root = app
            .identity
            .bootstrap("profile-root", "管理员", "test-password-at-least-12", true)
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
        Self {
            admin,
            pool,
            schema,
            app,
            actor,
            token,
        }
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .execute(&self.admin)
            .await
            .unwrap();
        self.admin.close().await;
    }
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn registration_settings_are_atomic_platform_only_and_match_contract() {
    let fixture = Fixture::new().await;
    let service = &fixture.app.registration_settings;
    let initial = service.read(&fixture.actor).await.unwrap();
    let expected: Value = serde_json::from_str(include_str!(
        "../../../api-contract/fixtures/miniapp-registration-settings.json"
    ))
    .unwrap();
    let mut attributes = serde_json::to_value(initial).unwrap();
    attributes["updated_at"] = expected["data"]["attributes"]["updated_at"].clone();
    assert_eq!(attributes, expected["data"]["attributes"]);
    let mut unauthorized = fixture.actor.clone();
    unauthorized.platform_admin = false;
    assert!(matches!(
        service.read(&unauthorized).await,
        Err(Error::Forbidden)
    ));
    unauthorized = fixture.actor.clone();
    unauthorized.session_kind = SessionKind::Miniapp;
    assert!(matches!(
        service.update(&unauthorized, settings("不能保存", 0)).await,
        Err(Error::Forbidden)
    ));
    assert!(
        service
            .update(&fixture.actor, settings(" ", 0))
            .await
            .is_err()
    );
    assert!(
        service
            .update(&fixture.actor, settings(&"字".repeat(92), 0))
            .await
            .is_err()
    );
    let saved = service
        .update(&fixture.actor, settings(" 校园用户 ", 0))
        .await
        .unwrap();
    assert_eq!(saved.nickname_prefix, "校园用户");
    assert_eq!(saved.revision, 1);
    assert!(matches!(
        service.update(&fixture.actor, settings("旧版本", 0)).await,
        Err(Error::Conflict)
    ));
    let detail: Value = sqlx::query_scalar(
        "SELECT detail FROM audit_events WHERE action='miniapp_user.registration_settings.update'",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(detail["from"]["nickname_prefix"], "用户");
    assert_eq!(detail["to"]["nickname_prefix"], "校园用户");

    let router = linjian_api::http::app_router(fixture.app.clone());
    let path = "/api/v1/platform/miniapp-registration-settings";
    let response = router
        .clone()
        .oneshot(
            Request::get(path)
                .header("cookie", format!("linjian_session={}", fixture.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let response = router
        .clone()
        .oneshot(
            Request::put(path)
                .header("cookie", format!("linjian_session={}", fixture.token))
                .header("content-type", "application/vnd.api+json")
                .body(Body::from(include_str!(
                    "../../../api-contract/fixtures/miniapp-registration-settings-input.json"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = router.oneshot(Request::put(path).header("cookie", format!("linjian_session={}", fixture.token)).header("origin", "http://localhost:5173")
        .header("x-csrf-token", &fixture.actor.csrf).header("content-type", "application/vnd.api+json")
        .body(Body::from(json!({"data":{"type":"miniapp-registration-settings","attributes":{"nickname_prefix":"新前缀","revision":1}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn generated_nicknames_retry_without_enforcing_manual_uniqueness() {
    let fixture = Fixture::new().await;
    let repo = Repository(fixture.pool.clone());
    let first = repo
        .register_with_random("app", "one", || Ok(sequence(&[123456789])))
        .await
        .unwrap();
    let second = repo
        .register_with_random("app", "two", || Ok(sequence(&[123456789, 987654321])))
        .await
        .unwrap();
    assert_eq!(repo.own(first).await.unwrap().nickname, "用户123456789");
    assert_eq!(repo.own(second).await.unwrap().nickname, "用户987654321");
    let repeated = repo
        .register_with_random(
            "app",
            "one",
            || -> linjian_api::business::Result<Sequence> {
                panic!("existing identity must not generate a new nickname")
            },
        )
        .await
        .unwrap();
    assert_eq!(first, repeated);
    let (_, first_actor) = fixture
        .app
        .identity
        .0
        .session(first, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let saved = repo
        .profile(
            &first_actor,
            ProfileInput {
                nickname: "用户987654321".into(),
                avatar_asset_id: None,
                revision: repo.own(first).await.unwrap().revision,
            },
        )
        .await
        .unwrap();
    assert_eq!(saved.nickname, repo.own(second).await.unwrap().nickname);
    let third = repo
        .register_with_random("app", "three", || Ok(sequence(&[123456789])))
        .await
        .unwrap();
    assert_eq!(repo.own(third).await.unwrap().nickname, "用户123456789");
    fixture
        .app
        .registration_settings
        .update(&fixture.actor, settings("新前缀", 0))
        .await
        .unwrap();
    let fourth = repo
        .register_with_random("app", "four", || Ok(sequence(&[123456789])))
        .await
        .unwrap();
    assert_eq!(repo.own(fourth).await.unwrap().nickname, "新前缀123456789");
    let exhausted = repo
        .register_with_random("app", "exhausted", || Ok(sequence(&[123456789; 32])))
        .await;
    assert!(matches!(exhausted, Err(Error::Unavailable)));
    let entropy_failed = repo
        .register_with_random(
            "app",
            "entropy-failed",
            || -> linjian_api::business::Result<Sequence> { Err(Error::Unavailable) },
        )
        .await;
    assert!(matches!(entropy_failed, Err(Error::Unavailable)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM miniapp_identities")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        4
    );
    let unique_indexes: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_index WHERE indrelid='miniapp_users'::regclass AND indisunique AND pg_get_indexdef(indexrelid) LIKE '%(nickname)%'").fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(unique_indexes, 0);
    fixture
        .app
        .registration_settings
        .update(&fixture.actor, settings("回滚", 1))
        .await
        .unwrap();
    sqlx::query("CREATE FUNCTION reject_test_profile() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected profile failure'; END $$")
        .execute(&fixture.pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_test_profile BEFORE INSERT ON miniapp_users FOR EACH ROW EXECUTE FUNCTION reject_test_profile()")
        .execute(&fixture.pool).await.unwrap();
    assert!(
        repo.register_with_random("app", "rollback", || Ok(sequence(&[111111111])))
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM accounts")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        5
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM miniapp_identities")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        4
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn concurrent_generators_read_fresh_data_after_the_transaction_lock() {
    let fixture = Fixture::new().await;
    let mut blocker = fixture.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(NICKNAME_LOCK)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let first_pool = fixture.pool.clone();
    let first = tokio::spawn(async move {
        Repository(first_pool)
            .register_with_random("app", "one", || Ok(sequence(&[123456789, 234567890])))
            .await
    });
    let second_pool = fixture.pool.clone();
    let second = tokio::spawn(async move {
        Repository(second_pool)
            .register_with_random("app", "two", || Ok(sequence(&[123456789, 234567890])))
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_locks WHERE locktype='advisory' AND NOT granted AND objid::bigint=$1")
                .bind((NICKNAME_LOCK as u64 & 0xffff_ffff) as i64).fetch_one(&fixture.pool).await.unwrap();
            if waiting >= 2 { break; }
            tokio::task::yield_now().await;
        }
    }).await.unwrap();
    blocker.commit().await.unwrap();
    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        (
            first.await.unwrap().unwrap(),
            second.await.unwrap().unwrap(),
        )
    })
    .await
    .unwrap();
    let repo = Repository(fixture.pool.clone());
    assert_ne!(
        repo.own(first).await.unwrap().nickname,
        repo.own(second).await.unwrap().nickname
    );
    let (same_first, same_second) = tokio::join!(
        repo.register_with_random("app", "same", || Ok(sequence(&[345678901]))),
        repo.register_with_random("app", "same", || Ok(sequence(&[345678901])))
    );
    assert_eq!(same_first.unwrap(), same_second.unwrap());
    fixture.close().await;
}
