use crate::business::{Error, Result};
use chrono::{DateTime, FixedOffset};
use serde_json::Value;

pub fn project(value: Value) -> Result<Value> {
    let published_at = value["display_published_at"]
        .as_str()
        .ok_or(Error::Unavailable)?;
    let published_at =
        DateTime::parse_from_rfc3339(published_at).map_err(|_| Error::Unavailable)?;
    let published_at_text = published_at
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M")
        .to_string();
    let mut fields = serde_json::Map::new();
    for key in [
        "id",
        "school_id",
        "author_id",
        "title",
        "content",
        "images",
        "tags",
        "audit_status",
        "visibility",
        "browse",
        "revision",
        "display_published_at",
        "author_name",
        "like_count",
        "collect_count",
        "comment_count",
    ] {
        if let Some(field) = value.get(key) {
            fields.insert(key.into(), field.clone());
        }
    }
    fields.insert("published_at_text".into(), Value::String(published_at_text));
    Ok(Value::Object(fields))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn public_writes_do_not_expose_internal_provenance_or_review_evidence() {
        let value = project(
            json!({"id":"post","created_via":"seed","seed_batch_id":"batch","created_by":"actor","approved_snapshot_id":"snapshot","source_projection":"message_v1","display_published_at":"2026-01-01T00:00:00Z","created_at":"2026-11-20T00:00:00Z","updated_at":"2026-11-20T01:00:00Z"}),
        ).unwrap();
        assert_eq!(value["published_at_text"], "2026-01-01 08:00");
        for field in [
            "created_via",
            "seed_batch_id",
            "created_by",
            "approved_snapshot_id",
            "source_projection",
            "is_initial_content",
            "created_at",
            "updated_at",
        ] {
            assert!(value.get(field).is_none());
        }
    }
    #[test]
    fn publication_text_uses_the_instant_not_the_original_offset_or_actor() {
        for source in ["miniapp", "web", "seed"] {
            for timestamp in [
                "2026-11-19T17:30:00Z",
                "2026-11-20T01:30:00+08:00",
                "2026-11-19T12:30:00-05:00",
            ] {
                let value = project(json!({"created_via":source,"display_published_at":timestamp}))
                    .unwrap();
                assert_eq!(value["published_at_text"], "2026-11-20 01:30");
            }
        }
        for timestamp in [Value::Null, json!("invalid")] {
            assert!(project(json!({"display_published_at":timestamp})).is_err());
        }
    }
    #[test]
    fn public_post_matches_the_shared_fixture_without_creation_metadata() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/post-created.json"
        ))
        .unwrap();
        let mut input = fixture["data"]["attributes"].clone();
        input["id"] = fixture["data"]["id"].clone();
        input["created_at"] = json!("2026-11-20T00:00:00Z");
        input["updated_at"] = json!("2026-11-20T01:00:00Z");
        let mut expected = fixture["data"]["attributes"].clone();
        expected["id"] = fixture["data"]["id"].clone();
        assert_eq!(project(input).unwrap(), expected);
    }
}
