use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use linjian_api::{bootstrap, http};
use tower::ServiceExt;
#[tokio::test]
async fn blocked_words_jsonapi_matches_shared_fixture() {
    use axum::response::IntoResponse;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/blocked-words.json"
    ))
    .unwrap();
    let mut catalog = fixture["data"]["attributes"].clone();
    catalog["id"] = fixture["data"]["id"].clone();
    let response =
        linjian_api::http::jsonapi::resource("blocked-word-catalog", catalog).into_response();
    let actual: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(actual, fixture);
}
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

#[tokio::test]
async fn case_jsonapi_matches_shared_fixtures_and_rejects_spoofed_handling() {
    use axum::response::IntoResponse;
    use linjian_api::community::domain::case::{CaseManagementView, ResolveCaseInput};
    use serde_json::json;
    for raw in [
        include_str!("../../api-contract/fixtures/case-open.json"),
        include_str!("../../api-contract/fixtures/case-resolved.json"),
        include_str!("../../api-contract/fixtures/case-dismissed.json"),
        include_str!("../../api-contract/fixtures/case-feedback.json"),
        include_str!("../../api-contract/fixtures/case-legacy.json"),
    ] {
        let fixture: serde_json::Value = serde_json::from_str(raw).unwrap();
        let mut attributes = fixture["data"]["attributes"].clone();
        attributes["id"] = fixture["data"]["id"].clone();
        let case: CaseManagementView = serde_json::from_value(attributes).unwrap();
        let response =
            linjian_api::http::jsonapi::singleton("cases", case.record.id, case).into_response();
        let document: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(document, fixture);
    }
    let input = json!({"revision":1,"state":"resolved","resolution":" 已处理 "});
    let mut valid: ResolveCaseInput = serde_json::from_value(input.clone()).unwrap();
    valid.validate().unwrap();
    assert_eq!(valid.resolution, "已处理");
    for key in ["handled_at", "handled_by", "reporter_name", "school_name"] {
        let mut spoofed = input.clone();
        spoofed[key] = json!("forged");
        assert!(serde_json::from_value::<ResolveCaseInput>(spoofed).is_err());
    }
    for invalid in [
        json!({"revision":0,"state":"resolved","resolution":"已处理"}),
        json!({"revision":1,"state":"open","resolution":"已处理"}),
        json!({"revision":1,"state":"dismissed","resolution":" "}),
        json!({"revision":1,"state":"resolved","resolution":"a".repeat(2001)}),
    ] {
        assert!(
            serde_json::from_value::<ResolveCaseInput>(invalid)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}

#[tokio::test]
async fn post_management_and_import_shared_fixtures() {
    use axum::response::IntoResponse;
    use linjian_api::community::domain::{
        post_import::PostImportPreview,
        post_management::{PostManagementView, ReorderPostsInput},
        review::PostReview,
    };
    for raw in [
        include_str!("../../api-contract/fixtures/post-management.json"),
        include_str!("../../api-contract/fixtures/post-management-hidden.json"),
        include_str!("../../api-contract/fixtures/post-management-offline.json"),
    ] {
        let fixture: serde_json::Value = serde_json::from_str(raw).unwrap();
        let mut attrs = fixture["data"]["attributes"].clone();
        let is_top = attrs
            .as_object_mut()
            .unwrap()
            .remove("is_top")
            .unwrap()
            .as_bool()
            .unwrap();
        let cover = attrs
            .as_object_mut()
            .unwrap()
            .remove("cover_image")
            .unwrap();
        let cover_image = cover.as_str().map(str::to_owned);
        attrs["id"] = fixture["data"]["id"].clone();
        let post: PostReview = serde_json::from_value(attrs).unwrap();
        let response = linjian_api::http::jsonapi::resource(
            "post-management",
            PostManagementView {
                post,
                is_top,
                cover_image,
            },
        )
        .into_response();
        let doc: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(doc, fixture);
    }
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-order-input.json"
    ))
    .unwrap();
    let input: ReorderPostsInput =
        serde_json::from_value(fixture["data"]["attributes"].clone()).unwrap();
    input.validate().unwrap();
    for raw in [
        include_str!("../../api-contract/fixtures/post-import-preview.json"),
        include_str!("../../api-contract/fixtures/post-import-result.json"),
    ] {
        let fixture: serde_json::Value = serde_json::from_str(raw).unwrap();
        let mut attrs = fixture["data"]["attributes"].clone();
        attrs["id"] = fixture["data"]["id"].clone();
        let job: PostImportPreview = serde_json::from_value(attrs).unwrap();
        let response = linjian_api::http::jsonapi::resource("post-imports", job).into_response();
        let doc: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(doc, fixture);
    }
}

#[test]
fn comment_visibility_contract_keeps_moderation_separate() {
    use linjian_api::community::domain::SetCommentVisibility;
    use serde_json::json;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/comment-visibility-input.json"
    ))
    .unwrap();
    let attrs = fixture["data"]["attributes"].clone();
    let input: SetCommentVisibility = serde_json::from_value(attrs.clone()).unwrap();
    assert_eq!(serde_json::to_value(input).unwrap(), attrs);
    for invalid in [
        json!({"revision":1,"visibility":"deleted"}),
        json!({"revision":1,"visibility":"normal"}),
        json!({"revision":1,"visibility":"hidden","audit_status":"approved"}),
    ] {
        assert!(serde_json::from_value::<SetCommentVisibility>(invalid).is_err());
    }
}
