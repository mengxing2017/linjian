use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use linjian_api::{bootstrap, http};
use tower::ServiceExt;
#[tokio::test]
async fn routing_has_no_sender_or_shared_session_dependency() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgresql://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let router = http::app_router(bootstrap::build_application(pool));
    for path in [
        "/api/v1/platform/execution-devices",
        "/api/v1/schools/s/mobile-execution/shared-tasks",
        "/api/v1/schools/s/broadcast-tasks",
        "/api/v1/schools/s/groups",
    ] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/session")
                .header("Cookie", "wekeeper_session=old-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/system")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let doc: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(doc["data"]["attributes"]["application"], "linjian");
}

#[tokio::test]
async fn post_review_jsonapi_matches_shared_fixture() {
    use axum::response::IntoResponse;
    use linjian_api::{community::domain::review::PostReview, content_publication::domain::Retry};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../api-contract/fixtures/post-review.json")).unwrap();
    let mut attributes = fixture["data"]["attributes"].clone();
    attributes["id"] = fixture["data"]["id"].clone();
    let preview: PostReview = serde_json::from_value(attributes).unwrap();
    let response =
        linjian_api::http::jsonapi::singleton("post-reviews", preview.id, preview).into_response();
    let doc: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(doc, fixture);
    for raw in [
        include_str!("../../api-contract/fixtures/post-review-human.json"),
        include_str!("../../api-contract/fixtures/post-review-automatic.json"),
        include_str!("../../api-contract/fixtures/post-review-legacy.json"),
    ] {
        let fixture: serde_json::Value = serde_json::from_str(raw).unwrap();
        let mut attributes = fixture["data"]["attributes"].clone();
        attributes["id"] = fixture["data"]["id"].clone();
        let review: PostReview = serde_json::from_value(attributes).unwrap();
        let response = linjian_api::http::jsonapi::singleton("post-reviews", review.id, review)
            .into_response();
        let document: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(document, fixture);
    }
    let decision: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-review-decision-input.json"
    ))
    .unwrap();
    let input: linjian_api::business::Decision =
        serde_json::from_value(decision["data"]["attributes"].clone()).unwrap();
    assert_eq!(
        input.review_digest.as_deref(),
        Some("review-fixture-digest")
    );
    assert!(
        serde_json::from_value::<linjian_api::business::Decision>(
            serde_json::json!({"revision":2,"decision":"approved","source_preview_digest":"old"})
        )
        .is_err()
    );
    let retry: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/source-preparation-retry-input.json"
    ))
    .unwrap();
    let input: Retry = serde_json::from_value(retry["data"]["attributes"].clone()).unwrap();
    assert_eq!(input.revision, 3);
    assert!(!input.reason.is_empty());
}

#[tokio::test]
async fn miniapp_user_contract_is_separate_from_backend_accounts() {
    use axum::response::IntoResponse;
    use linjian_api::miniapp_users::domain::{
        MobileInput, ProfileInput, SchoolInput, StatusInput, UserView,
    };
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/miniapp-user.json"
    ))
    .unwrap();
    let mut attrs = fixture["data"]["attributes"].clone();
    attrs["id"] = fixture["data"]["id"].clone();
    let user: UserView = serde_json::from_value(attrs).unwrap();
    let response = linjian_api::http::jsonapi::resource("miniapp-users", user).into_response();
    let actual: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(actual, fixture);
    let status: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/miniapp-user-status-input.json"
    ))
    .unwrap();
    assert!(serde_json::from_value::<StatusInput>(status["data"]["attributes"].clone()).is_ok());
    let profile: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/miniapp-user-profile-input.json"
    ))
    .unwrap();
    assert!(serde_json::from_value::<ProfileInput>(profile["data"]["attributes"].clone()).is_ok());
    let mobile: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/miniapp-user-mobile-input.json"
    ))
    .unwrap();
    assert!(serde_json::from_value::<MobileInput>(mobile["data"]["attributes"].clone()).is_ok());
    let school: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/miniapp-user-school-input.json"
    ))
    .unwrap();
    assert!(serde_json::from_value::<SchoolInput>(school["data"]["attributes"].clone()).is_ok());
}

#[tokio::test]
async fn school_profile_fixture_matches_rust_and_excludes_active() {
    use axum::response::IntoResponse;
    use linjian_api::schools::domain::profile::{SchoolInput, SchoolView};
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/school-profile.json"
    ))
    .unwrap();
    let mut attributes = expected["data"]["attributes"].clone();
    attributes["id"] = expected["data"]["id"].clone();
    let school: SchoolView = serde_json::from_value(attributes).unwrap();
    assert!(school.directory().get("config").is_none());
    let response = linjian_api::http::jsonapi::resource("schools", school).into_response();
    let actual: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(actual, expected);
    let input: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/school-profile-input.json"
    ))
    .unwrap();
    SchoolInput::parse(input["data"]["attributes"].clone())
        .unwrap()
        .validate()
        .unwrap();
}
