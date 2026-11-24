use linjian_api::community::domain::{PostInput, post_tags};
use serde_json::Value;

#[test]
fn candidate_and_retained_tag_commands_match_typescript_fixtures() {
    for (document, retained) in [
        (
            include_str!("../../api-contract/fixtures/post-tags-input.json"),
            vec![],
        ),
        (
            include_str!("../../api-contract/fixtures/post-tags-retained-input.json"),
            vec!["历史标签".into()],
        ),
    ] {
        let document: Value = serde_json::from_str(document).unwrap();
        let input: PostInput =
            serde_json::from_value(document["data"]["attributes"].clone()).unwrap();
        input.validate(uuid::Uuid::nil()).unwrap();
        post_tags::validate_selection(&input.tags, &["校园".into()], &retained).unwrap();
        assert_eq!(input.candidate_tag_ids.len(), 1);
        assert_eq!(
            serde_json::to_value(input).unwrap(),
            document["data"]["attributes"]
        );
    }
}

#[tokio::test]
async fn tag_validation_error_matches_the_client_fixture() {
    use axum::{body::to_bytes, response::IntoResponse};
    let error = post_tags::validate_selection(&["自定义".into()], &[], &[]).unwrap_err();
    let response = error.into_response();
    assert_eq!(response.status(), 422);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let actual: Value = serde_json::from_slice(&body).unwrap();
    let expected: Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-tags-validation-error.json"
    ))
    .unwrap();
    assert_eq!(actual, expected);
}
