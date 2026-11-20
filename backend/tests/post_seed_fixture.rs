use axum::{body::to_bytes, response::IntoResponse};
use linjian_api::community::domain::post_seed::{
    BatchView, CommitInput, PreviewInput, RevokeInput,
};
use serde_json::Value;

#[tokio::test]
async fn seed_batch_serialization_matches_shared_fixture() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-seed-preview.json"
    ))
    .unwrap();
    let mut attributes = fixture["data"]["attributes"].clone();
    attributes["id"] = fixture["data"]["id"].clone();
    let batch: BatchView = serde_json::from_value(attributes).unwrap();
    let response = linjian_api::http::jsonapi::resource("post-seed-batches", batch).into_response();
    let actual: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap();
    assert_eq!(actual, fixture);
    let input: Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-seed-preview-input.json"
    ))
    .unwrap();
    serde_json::from_value::<PreviewInput>(input["data"]["attributes"].clone())
        .unwrap()
        .validate()
        .unwrap();
    let input: Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-seed-commit-input.json"
    ))
    .unwrap();
    serde_json::from_value::<CommitInput>(input["data"]["attributes"].clone()).unwrap();
    let input: Value = serde_json::from_str(include_str!(
        "../../api-contract/fixtures/post-seed-revoke-input.json"
    ))
    .unwrap();
    serde_json::from_value::<RevokeInput>(input["data"]["attributes"].clone()).unwrap();
}
