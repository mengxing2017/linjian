use crate::business::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    Comment,
    Like,
    Follow,
    Moderation,
}

impl NotificationKind {
    pub fn parse(value: Option<&str>) -> Result<Option<Self>> {
        match value {
            None => Ok(None),
            Some("comment") => Ok(Some(Self::Comment)),
            Some("like") => Ok(Some(Self::Like)),
            Some("follow") => Ok(Some(Self::Follow)),
            Some("moderation") => Ok(Some(Self::Moderation)),
            Some(_) => Err(Error::Invalid("消息分类无效".into())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Like => "like",
            Self::Follow => "follow",
            Self::Moderation => "moderation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::Page;

    #[test]
    fn notification_query_accepts_only_explicit_supported_kinds() {
        assert_eq!(NotificationKind::parse(None).unwrap(), None);
        for value in ["comment", "like", "follow", "moderation"] {
            assert_eq!(
                NotificationKind::parse(Some(value))
                    .unwrap()
                    .unwrap()
                    .as_str(),
                value
            );
        }
        for value in ["", " ", "Comment", "like ", "all", "collect", "unknown"] {
            assert!(matches!(
                NotificationKind::parse(Some(value)),
                Err(Error::Invalid(_))
            ));
        }
    }

    #[tokio::test]
    async fn notification_query_serialization_matches_shared_fixtures() {
        use axum::{body::to_bytes, response::IntoResponse};
        use serde_json::{Value, json};

        let row = |id: &str, kind: &str, object_id: Value, created_at: &str, read_at: Value| {
            json!({
                "id": format!("00000000-0000-4000-8000-0000000000{id}"),
                "school_id": "00000000-0000-4000-8000-000000000001",
                "account_id": "00000000-0000-4000-8000-000000000002",
                "kind": kind, "object_id": object_id, "created_at": created_at, "read_at": read_at
            })
        };
        let object = json!("00000000-0000-4000-8000-000000000003");
        let created = "2026-11-24T02:30:00Z";
        for (rows, page, fixture) in [
            (
                vec![
                    row("11", "like", object.clone(), created, Value::Null),
                    row(
                        "12",
                        "future",
                        Value::Null,
                        "2026-11-23T02:30:00Z",
                        json!("2026-11-23T03:00:00Z"),
                    ),
                ],
                Page {
                    page_size: Some(20),
                    ..Default::default()
                },
                include_str!("../../../../api-contract/fixtures/notifications-all.json"),
            ),
            (
                vec![
                    row("13", "comment", object.clone(), created, Value::Null),
                    row("15", "comment", object.clone(), created, Value::Null),
                ],
                Page {
                    page: Some(2),
                    page_size: Some(1),
                    ..Default::default()
                },
                include_str!("../../../../api-contract/fixtures/notifications-comment.json"),
            ),
            (
                vec![row(
                    "14",
                    "moderation",
                    object.clone(),
                    created,
                    Value::Null,
                )],
                Page {
                    page_size: Some(20),
                    ..Default::default()
                },
                include_str!("../../../../api-contract/fixtures/notifications-moderation.json"),
            ),
            (
                vec![],
                Page {
                    page_size: Some(20),
                    ..Default::default()
                },
                include_str!("../../../../api-contract/fixtures/notifications-empty.json"),
            ),
        ] {
            let response =
                crate::http::jsonapi::collection("notifications", rows, &page).into_response();
            let document: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap())
                    .unwrap();
            assert_eq!(document, serde_json::from_str::<Value>(fixture).unwrap());
        }
    }
}
