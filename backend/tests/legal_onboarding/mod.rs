use super::*;
use linjian_api::{business::SessionKind, onboarding::domain::Command};

fn command(ticket: &str, acceptances: Value, school: Option<Uuid>, key: Option<Uuid>) -> Command {
    serde_json::from_value(
        json!({"ticket":ticket,"acceptances":acceptances,"school_id":school,"request_key":key}),
    )
    .unwrap()
}
async fn public(router: &axum::Router, path: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), 512 * 1024).await.unwrap()).unwrap();
    (status, value)
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn legal_documents_and_first_onboarding_are_atomic_public_and_versioned() {
    let admin = PgPool::connect(
        &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
    )
    .await
    .unwrap();
    let schema = format!("legal_onboarding_{}", Uuid::new_v4().simple());
    let pool = isolated(&admin, &schema).await;
    database::migrate(&pool).await.unwrap();
    let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
    let router = linjian_api::http::app_router(app.clone());
    assert_eq!(
        public(&router, "/api/v1/legal-documents").await,
        (StatusCode::OK, json!({"data":[]}))
    );
    assert_eq!(
        public(&router, "/api/v1/legal-documents/privacy_policy")
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        public(&router, "/api/v1/platform/legal-documents/privacy_policy")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let id = app
        .identity
        .bootstrap(
            "legal-root",
            "平台管理员",
            "test-password-long-enough",
            true,
        )
        .await
        .unwrap();
    let (_, actor) = app
        .identity
        .0
        .session(id, SessionKind::AdminBrowser, Some("http://localhost:5173"))
        .await
        .unwrap();
    let school: Uuid = app
        .schools
        .create(&actor, "入驻学校", 23.1, 113.3)
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
    let ordinary = app
        .identity
        .bootstrap(
            "legal-ordinary",
            "普通管理员",
            "test-password-long-enough",
            false,
        )
        .await
        .unwrap();
    let (_, ordinary) = app
        .identity
        .0
        .session(
            ordinary,
            SessionKind::AdminBrowser,
            Some("http://localhost:5173"),
        )
        .await
        .unwrap();
    assert!(
        app.legal_documents
            .save(&ordinary, "privacy_policy", 0, Some("无权".into()))
            .await
            .is_err()
    );
    for kind in ["user_agreement", "privacy_policy"] {
        assert!(
            app.legal_documents
                .save(&actor, kind, 0, Some("\u{200b}　".into()))
                .await
                .is_err()
        );
        let saved = app
            .legal_documents
            .save(&actor, kind, 0, Some(format!("{kind}\n正文")))
            .await
            .unwrap();
        assert_eq!(saved["revision"], 1);
        let identical = app
            .legal_documents
            .save(&actor, kind, 1, Some(format!("{kind}\n正文")))
            .await
            .unwrap();
        assert_eq!(identical["revision"], 1);
        assert!(
            app.legal_documents
                .save(&actor, kind, 0, Some("冲突".into()))
                .await
                .is_err()
        );
    }
    let (status, document) = public(&router, "/api/v1/legal-documents/privacy_policy").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        document["data"]["attributes"]["content"]
            .as_str()
            .unwrap()
            .contains('\n')
    );
    assert!(
        document["data"]["attributes"]
            .get("updated_by_name")
            .is_none()
    );
    let (_, schools) = public(&router, "/api/v1/onboarding/schools").await;
    assert_eq!(schools["data"].as_array().unwrap().len(), 2);
    assert!(schools["data"][0]["attributes"].get("config").is_none());

    let start = app
        .onboarding
        .start("test-app", "new-openid")
        .await
        .unwrap();
    assert_eq!(start["state"], "consent_required");
    assert!(start.get("token").is_none());
    let ticket = start["ticket"].as_str().unwrap();
    assert!(
        app.onboarding
            .command(
                "confirm",
                command(
                    ticket,
                    json!([{ "kind":"user_agreement","revision":1 }]),
                    None,
                    None
                )
            )
            .await
            .is_err()
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM miniapp_identities WHERE openid='new-openid'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let both =
        json!([{"kind":"user_agreement","revision":1},{"kind":"privacy_policy","revision":1}]);
    let confirmed = app
        .onboarding
        .command("confirm", command(ticket, both.clone(), None, None))
        .await
        .unwrap();
    assert_eq!(confirmed["state"], "school_required");
    assert!(confirmed.get("token").is_none());
    let account: Uuid =
        sqlx::query_scalar("SELECT account_id FROM miniapp_identities WHERE openid='new-openid'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let bound: Option<Uuid> =
        sqlx::query_scalar("SELECT bound_school_id FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(bound, None);
    assert!(
        app.onboarding
            .command("complete", command(ticket, json!([]), Some(school), None))
            .await
            .is_err()
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM school_memberships WHERE account_id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    assert!(
        app.onboarding
            .command(
                "complete",
                command(
                    ticket,
                    json!([]),
                    Some(Uuid::new_v4()),
                    Some(Uuid::new_v4())
                )
            )
            .await
            .is_err()
    );
    app.legal_documents
        .save(&actor, "privacy_policy", 1, Some("隐私新版本".into()))
        .await
        .unwrap();
    let blocked = app
        .onboarding
        .command(
            "complete",
            command(ticket, json!([]), Some(school), Some(Uuid::new_v4())),
        )
        .await
        .unwrap();
    assert_eq!(blocked["state"], "consent_required");
    assert!(blocked.get("token").is_none());
    let current =
        json!([{"kind":"user_agreement","revision":1},{"kind":"privacy_policy","revision":2}]);
    app.onboarding
        .command("confirm", command(ticket, current, None, None))
        .await
        .unwrap();
    let key = Uuid::new_v4();
    let ready = app
        .onboarding
        .command(
            "complete",
            command(ticket, json!([]), Some(school), Some(key)),
        )
        .await
        .unwrap();
    assert_eq!(ready["state"], "ready");
    assert_eq!(ready["user"]["bound_school_id"], school.to_string());
    let repeated = app
        .onboarding
        .command(
            "complete",
            command(ticket, json!([]), Some(school), Some(key)),
        )
        .await
        .unwrap();
    assert_eq!(ready, repeated);
    assert!(
        app.onboarding
            .command(
                "complete",
                command(ticket, json!([]), Some(other), Some(key))
            )
            .await
            .is_err()
    );
    assert!(
        app.onboarding
            .command(
                "complete",
                command(ticket, json!([]), Some(school), Some(Uuid::new_v4()))
            )
            .await
            .is_err()
    );
    assert_eq!(
        app.onboarding
            .command("status", command(ticket, json!([]), None, None))
            .await
            .unwrap(),
        ready
    );
    let ciphertext: Vec<u8> =
        sqlx::query_scalar("SELECT result FROM miniapp_onboarding_attempts WHERE token_hash=$1")
            .bind(linjian_api::business::hash(ticket))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !ciphertext
            .windows(ready["token"].as_str().unwrap().len())
            .any(|window| window == ready["token"].as_str().unwrap().as_bytes())
    );
    let auth = app
        .identity
        .authenticate(ready["token"].as_str().unwrap(), SessionKind::Miniapp)
        .await
        .unwrap();
    assert!(auth.can(school, "member"));
    assert!(!auth.can(other, "member"));
    let acceptances: i64 =
        sqlx::query_scalar("SELECT count(*) FROM miniapp_legal_acceptances WHERE account_id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(acceptances, 3);
    app.legal_documents
        .save(&actor, "user_agreement", 1, None)
        .await
        .unwrap();
    app.legal_documents
        .save(&actor, "privacy_policy", 2, None)
        .await
        .unwrap();
    assert_eq!(
        public(&router, "/api/v1/legal-documents").await.1,
        json!({"data":[]})
    );
    assert_eq!(
        public(&router, "/api/v1/legal-documents/privacy_policy")
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.legal_documents
            .history(&actor, "privacy_policy", 1)
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let start = app
        .onboarding
        .start("test-app", "empty-documents")
        .await
        .unwrap();
    let ticket = start["ticket"].as_str().unwrap();
    app.onboarding
        .command("confirm", command(ticket, json!([]), None, None))
        .await
        .unwrap();
    let ready = app
        .onboarding
        .command(
            "complete",
            command(ticket, json!([]), Some(school), Some(Uuid::new_v4())),
        )
        .await
        .unwrap();
    let account: Uuid = ready["user"]["id"].as_str().unwrap().parse().unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM miniapp_legal_acceptances WHERE account_id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    app.legal_documents
        .save(&actor, "privacy_policy", 3, Some("补配".into()))
        .await
        .unwrap();
    let existing = app
        .onboarding
        .start("test-app", "empty-documents")
        .await
        .unwrap();
    assert_eq!(existing["first_login"], false);
    assert_eq!(existing["state"], "school_required");
    let cancel = app.onboarding.start("test-app", "cancelled").await.unwrap();
    let cancel_ticket = cancel["ticket"].as_str().unwrap();
    app.onboarding
        .command("cancel", command(cancel_ticket, json!([]), None, None))
        .await
        .unwrap();
    assert!(
        app.onboarding
            .command("status", command(cancel_ticket, json!([]), None, None))
            .await
            .is_err()
    );
    let expired = app.onboarding.start("test-app", "expired").await.unwrap();
    let expired_ticket = expired["ticket"].as_str().unwrap();
    sqlx::query("UPDATE miniapp_onboarding_attempts SET expires_at=now()-interval '1 second' WHERE token_hash=$1")
        .bind(linjian_api::business::hash(expired_ticket)).execute(&pool).await.unwrap();
    assert!(
        app.onboarding
            .command(
                "confirm",
                command(
                    expired_ticket,
                    json!([{"kind":"privacy_policy","revision":4}]),
                    None,
                    None
                )
            )
            .await
            .is_err()
    );
    let concurrent = app
        .onboarding
        .start("test-app", "concurrent")
        .await
        .unwrap();
    let concurrent_ticket = concurrent["ticket"].as_str().unwrap();
    let accepted = json!([{"kind":"privacy_policy","revision":4}]);
    let (first, second) = tokio::join!(
        app.onboarding.command(
            "confirm",
            command(concurrent_ticket, accepted.clone(), None, None)
        ),
        app.onboarding
            .command("confirm", command(concurrent_ticket, accepted, None, None))
    );
    first.unwrap();
    second.unwrap();
    let request_key = Uuid::new_v4();
    let (first, second) = tokio::join!(
        app.onboarding.command(
            "complete",
            command(
                concurrent_ticket,
                json!([]),
                Some(school),
                Some(request_key)
            )
        ),
        app.onboarding.command(
            "complete",
            command(
                concurrent_ticket,
                json!([]),
                Some(school),
                Some(request_key)
            )
        )
    );
    let result = first.unwrap();
    assert_eq!(result, second.unwrap());
    let account: Uuid = result["user"]["id"].as_str().unwrap().parse().unwrap();
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE account_id=$1")
        .bind(account)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 1);
    let identities: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM miniapp_identities WHERE app_id='test-app' AND openid='concurrent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(identities, 1);
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.onboarding
            .command("status", command(concurrent_ticket, json!([]), None, None))
            .await
            .is_err()
    );
    sqlx::query("UPDATE schools SET active=true WHERE id=$1")
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE school_memberships SET role='revoked' WHERE account_id=$1 AND school_id=$2",
    )
    .bind(account)
    .bind(school)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        app.onboarding
            .command("status", command(concurrent_ticket, json!([]), None, None))
            .await
            .is_err()
    );
    sqlx::query("UPDATE school_memberships SET role='member' WHERE account_id=$1 AND school_id=$2")
        .bind(account)
        .bind(school)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE miniapp_users SET status='banned' WHERE account_id=$1")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        app.onboarding
            .command("status", command(concurrent_ticket, json!([]), None, None))
            .await
            .is_err()
    );
    assert!(sqlx::query("UPDATE legal_document_revisions SET content='tampered' WHERE kind='privacy_policy' AND revision=4").execute(&pool).await.is_err());
    app.onboarding.0.cleanup().await.unwrap();
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM miniapp_onboarding_attempts WHERE token_hash=ANY($1)",
    )
    .bind(vec![
        linjian_api::business::hash(expired_ticket),
        linjian_api::business::hash(cancel_ticket),
    ])
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(
        public(&router, "/api/v1/auth/miniapp").await.0,
        StatusCode::NOT_FOUND
    );
    let pushes: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pushes, 0);
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
