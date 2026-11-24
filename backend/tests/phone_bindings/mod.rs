use super::*;
use linjian_api::{
    business::{Actor, Error, SessionKind},
    miniapp_users::{
        domain::phone::{BindPhoneInput, VerifiedPhone},
        north::local::phone::Service,
        south::{
            adapter::{phone_cipher::PhoneCipher, postgres::Repository},
            port::{PhoneBindingStore, PhoneProvider},
        },
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
struct Provider(Arc<AtomicUsize>);
impl PhoneProvider for Provider {
    fn app_id(&self) -> &str {
        "phone-test-app"
    }
    async fn verify_phone(
        &self,
        code: &str,
        openid: &str,
    ) -> linjian_api::business::Result<VerifiedPhone> {
        self.0.fetch_add(1, Ordering::SeqCst);
        assert_eq!(openid, "trusted-openid");
        if code == "invalid" {
            return Err(Error::Phone("phone_authorization_invalid"));
        }
        VerifiedPhone::from_wechat(
            &json!({"errcode":0,"phone_info":{"phoneNumber":"13800138000","purePhoneNumber":"13800138000","countryCode":"86","watermark":{"appid":self.app_id(),"timestamp":chrono::Utc::now().timestamp()}}}),
            self.app_id(),
            chrono::Utc::now(),
        )
    }
}

fn input(revision: i64) -> BindPhoneInput {
    BindPhoneInput {
        code: "valid-once-code".into(),
        request_id: Uuid::new_v4(),
        revision,
        notice_version: "linjian-phone-v1".into(),
    }
}

pub(super) async fn verify_fixture_phone(pool: &PgPool, account: Uuid) {
    let cipher = PhoneCipher::new(&"12".repeat(32), "test-v1".into()).unwrap();
    sqlx::query("INSERT INTO miniapp_phone_bindings(account_id,status,phone_ciphertext,key_version,country_code,phone_last4,app_id,verified_at,notice_version,revision) VALUES($1,'verified',$2,'test-v1','86','8000','phone-test-app',now(),'linjian-phone-v1',1) ON CONFLICT(account_id) DO NOTHING")
        .bind(account).bind(cipher.encrypt(account, "13800138000").unwrap()).execute(pool).await.unwrap();
}

async fn member(app: &bootstrap::Application, root: &Actor, school: Uuid) -> (String, Actor) {
    let account = linjian_api::miniapp_users::north::local::register(
        &Repository(app.pool.clone()),
        "phone-test-app",
        "trusted-openid",
    )
    .await
    .unwrap();
    app.schools
        .grant(root, school, account, "member")
        .await
        .unwrap();
    app.identity
        .0
        .session(account, SessionKind::Miniapp, None)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn publication_requires_phone_only_for_miniapp_and_binding_is_versioned_and_private() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("phone_binding_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &name).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let root = app
        .identity
        .bootstrap(
            "phone-root",
            "平台管理员",
            "test-password-at-least-12",
            true,
        )
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
        .create(&actor, "手机号测试校", 23.1, 113.3)
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (token, mini) = member(&app, &actor, school).await;
    let store = Repository(pool.clone());
    let provider = Provider::default();
    let service = Service {
        store: store.clone(),
        provider: provider.clone(),
        cipher: Some(PhoneCipher::new(&"12".repeat(32), "test-v1".into()).unwrap()),
    };
    let post_input = || {
        serde_json::from_value(json!({"title":"测试","content":"正常正文","images":[],"tags":[]}))
            .unwrap()
    };
    assert_eq!(
        store.phone_binding(&mini).await.unwrap().status,
        "unverified"
    );
    sqlx::query("UPDATE miniapp_users SET mobile='13800138000',mobile_source='admin_corrected' WHERE account_id=$1").bind(mini.id).execute(&pool).await.unwrap();
    assert!(matches!(
        app.community.post(&mini, school, None, post_input()).await,
        Err(Error::Phone("phone_verification_required"))
    ));
    let root_post = app
        .community
        .post(&actor, school, None, post_input())
        .await
        .unwrap();
    let root_post: Uuid = root_post["id"].as_str().unwrap().parse().unwrap();
    approve_post(&pool, &actor, root_post).await;
    assert!(app.community.detail(&mini, school, root_post).await.is_ok());
    assert!(
        app.community
            .interaction_state(&mini, school, root_post)
            .await
            .unwrap()
            .phone_verification_required
    );
    assert!(matches!(
        app.community
            .react(&mini, school, root_post, "like", true)
            .await,
        Err(Error::Phone("phone_verification_required"))
    ));
    assert!(
        app.community
            .react(&mini, school, root_post, "like", false)
            .await
            .is_ok()
    );
    assert!(
        app.community
            .react(&mini, school, root_post, "collect", true)
            .await
            .is_ok()
    );
    let router = linjian_api::http::app_router(app.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/schools/{school}/posts"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/miniapp/profile/mobile")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::GONE);
    let mut invalid = input(0);
    invalid.code = "invalid".into();
    assert!(matches!(
        service.bind(&mini, invalid).await,
        Err(Error::Phone("phone_authorization_invalid"))
    ));
    assert_eq!(
        store.phone_binding(&mini).await.unwrap().status,
        "unverified"
    );
    let request = input(0);
    let request_id = request.request_id;
    let bound = service.bind(&mini, request).await.unwrap();
    assert_eq!(bound.status, "verified");
    assert_eq!(bound.masked_phone.as_deref(), Some("*******8000"));
    assert_eq!(bound.revision, 1);
    assert!(
        !app.community
            .interaction_state(&mini, school, root_post)
            .await
            .unwrap()
            .phone_verification_required
    );
    let replay = BindPhoneInput {
        request_id,
        ..input(0)
    };
    assert_eq!(service.bind(&mini, replay).await.unwrap().revision, 1);
    assert_eq!(provider.0.load(Ordering::SeqCst), 2);
    let changed = BindPhoneInput {
        code: "different".into(),
        request_id,
        ..input(0)
    };
    assert!(matches!(
        service.bind(&mini, changed).await,
        Err(Error::Phone("phone_binding_conflict"))
    ));
    let created = app
        .community
        .post(&mini, school, None, post_input())
        .await
        .unwrap();
    let created: Uuid = created["id"].as_str().unwrap().parse().unwrap();
    app.community
        .react(&mini, school, root_post, "like", true)
        .await
        .unwrap();
    let outbox_before: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let revoked = store.revoke_phone(&mini, 1).await.unwrap();
    assert_eq!(revoked.status, "revoked");
    assert_eq!(revoked.revision, 2);
    assert!(revoked.masked_phone.is_none());
    let erased: bool = sqlx::query_scalar("SELECT phone_ciphertext IS NULL AND phone_last4 IS NULL FROM miniapp_phone_bindings WHERE account_id=$1").bind(mini.id).fetch_one(&pool).await.unwrap();
    assert!(erased);
    assert!(matches!(
        app.community
            .post(&mini, school, Some(created), post_input())
            .await,
        Err(Error::Phone("phone_verification_required"))
    ));
    let pending = input(2);
    let pending_id = pending.request_id;
    store
        .reserve_phone(&mini, "phone-test-app", &pending)
        .await
        .unwrap();
    store.revoke_phone(&mini, 2).await.unwrap();
    let phone = provider
        .verify_phone("valid", "trusted-openid")
        .await
        .unwrap();
    assert!(matches!(
        store
            .finish_phone(&mini, &pending, &phone, &[1; 39], "test-v1")
            .await,
        Err(Error::Phone("phone_binding_conflict"))
    ));
    let pending_status: String = sqlx::query_scalar(
        "SELECT status FROM miniapp_phone_binding_requests WHERE account_id=$1 AND request_id=$2",
    )
    .bind(mini.id)
    .bind(pending_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending_status, "uncertain");
    let browser_user = app
        .identity
        .bootstrap(
            "school-admin",
            "学校管理员",
            "test-password-at-least-12",
            false,
        )
        .await
        .unwrap();
    app.schools
        .grant(&actor, school, browser_user, "admin")
        .await
        .unwrap();
    let (_, browser) = app
        .identity
        .0
        .session(
            browser_user,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    let mut edit = post_input();
    edit.revision = Some(
        sqlx::query_scalar("SELECT revision FROM posts WHERE id=$1")
            .bind(created)
            .fetch_one(&pool)
            .await
            .unwrap(),
    );
    app.community
        .management_write(&browser, school, Some(created), edit)
        .await
        .unwrap();
    assert!(service.bind(&actor, input(0)).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM forum_outbox")
            .fetch_one(&pool)
            .await
            .unwrap(),
        outbox_before
    );
    let audit: String = sqlx::query_scalar("SELECT coalesce(jsonb_agg(detail)::text,'[]') FROM audit_events WHERE action LIKE 'miniapp_user.phone_%'").fetch_one(&pool).await.unwrap();
    assert!(!audit.contains("13800138000") && !audit.contains("valid-once-code"));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn reservations_are_rate_limited_and_revocation_or_logout_fences_late_results() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let name = format!("phone_races_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &name).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let store = Repository(pool.clone());
    let account = linjian_api::miniapp_users::north::local::register(
        &store,
        "phone-test-app",
        "trusted-openid",
    )
    .await
    .unwrap();
    let (_, mini) = app
        .identity
        .0
        .session(account, SessionKind::Miniapp, None)
        .await
        .unwrap();
    let mut requests = Vec::new();
    for _ in 0..5 {
        let request = input(0);
        store
            .reserve_phone(&mini, "phone-test-app", &request)
            .await
            .unwrap();
        requests.push(request);
    }
    assert!(matches!(
        store
            .reserve_phone(&mini, "phone-test-app", &input(0))
            .await,
        Err(Error::Phone("phone_binding_rate_limited"))
    ));
    assert!(matches!(
        store
            .reserve_phone(&mini, "phone-test-app", &requests[0])
            .await,
        Err(Error::Phone("phone_binding_result_uncertain"))
    ));
    store.revoke_phone(&mini, 0).await.unwrap();
    let phone = Provider::default()
        .verify_phone("valid", "trusted-openid")
        .await
        .unwrap();
    assert!(matches!(
        store
            .finish_phone(&mini, &requests[0], &phone, &[1; 39], "test-v1")
            .await,
        Err(Error::Phone("phone_binding_conflict"))
    ));
    assert_eq!(store.phone_binding(&mini).await.unwrap().status, "revoked");
    app.identity.logout(&mini).await.unwrap();
    assert!(matches!(
        store
            .finish_phone(&mini, &requests[1], &phone, &[1; 39], "test-v1")
            .await,
        Err(Error::Unauthorized)
    ));
    assert!(matches!(
        store.phone_binding(&mini).await,
        Err(Error::Unauthorized)
    ));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {name} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
