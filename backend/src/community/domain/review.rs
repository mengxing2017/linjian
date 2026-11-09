use crate::business::{Error, Result, hash};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostReview {
    pub id: Uuid,
    pub school_id: Uuid,
    pub school_name: String,
    pub author_id: Uuid,
    pub author_nickname: String,
    pub title: String,
    pub content: String,
    pub images: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub audit_status: String,
    pub visibility: String,
    pub top_sort: i32,
    pub revision: i64,
    pub content_revision: i64,
    pub review_digest: String,
}

/// A versioned fingerprint of the complete review object, independent of publication summaries.
pub fn digest(post: &Value) -> Result<String> {
    let id = crate::business::uuid(post, "id")?;
    let school = crate::business::uuid(post, "school_id")?;
    let author = crate::business::uuid(post, "author_id")?;
    let revision = post["revision"].as_i64().ok_or(Error::Conflict)?;
    let content_revision = post["content_revision"].as_i64().ok_or(Error::Conflict)?;
    Ok(hash(
        &json!([
            "review-v1",
            school,
            id,
            author,
            revision,
            content_revision,
            post["title"],
            post["content"],
            post["images"],
            post["tags"]
        ])
        .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_text_attachments_and_revision_are_bound_but_display_names_are_not() {
        let mut post = json!({"id":Uuid::nil(),"school_id":Uuid::nil(),"author_id":Uuid::nil(),"revision":1,"content_revision":1,"title":"标题","content":"全文末尾","images":[],"tags":[]});
        let original = digest(&post).unwrap();
        for (key, value) in [
            ("content", json!("全文末尾修改")),
            ("images", json!(["image"])),
            ("tags", json!(["tag"])),
            ("revision", json!(2)),
        ] {
            let mut changed = post.clone();
            changed[key] = value;
            assert_ne!(original, digest(&changed).unwrap());
        }
        post["school_name"] = json!("显示名称修改");
        post["author_nickname"] = json!("昵称修改");
        assert_eq!(original, digest(&post).unwrap());
    }
}
