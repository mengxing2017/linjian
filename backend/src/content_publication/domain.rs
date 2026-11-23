use crate::business::{Error, Result, hash, require};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

pub const ALGORITHM: &str = "plain-v1";
pub const MESSAGE_VERSION: &str = "summary-20-v1";
pub fn page_title(title: &str, content: &str) -> String {
    let source = if title.trim().is_empty() {
        content
    } else {
        title
    };
    let cleaned = source
        .chars()
        .filter(|character| !character.is_control() || character.is_whitespace())
        .collect::<String>();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "校园动态".into()
    } else {
        cleaned.chars().take(20).collect()
    }
}
/// Current editors and readers use plain text. Never guess HTML from angle brackets.
pub fn summary(content: &str, title: &str) -> String {
    let clean = |s: &str| {
        s.chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut value = clean(content);
    if value.is_empty() {
        value = clean(title);
    }
    if value.chars().count() > 20 {
        value = value.chars().take(18).chain(['…', '…']).collect();
    }
    value
}
/// Build the source body locally; links are opaque content and never truncated.
pub fn message(
    content: &str,
    title: &str,
    link: Option<&str>,
) -> std::result::Result<String, Failure> {
    let clean = |value: &str| {
        value
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut source = clean(content);
    if source.is_empty() {
        source = clean(title);
    }
    if source.is_empty() {
        return Err(Failure::blocked("content_empty"));
    }
    let reserved = link.map_or(0, |v| v.chars().count() + 1);
    let budget = 200usize.saturating_sub(reserved).min(20);
    let truncated = source.chars().count() > budget;
    if budget == 0 || (truncated && budget < 3) {
        return Err(Failure::blocked("message_link_too_long"));
    }
    let text = if truncated {
        source.chars().take(budget - 2).collect::<String>() + "……"
    } else {
        source
    };
    Ok(match link {
        Some(link) => format!("{text}\n{link}"),
        None => text,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub app_id: String,
    pub link_type: String,
    pub permanent: bool,
    pub days: i64,
    pub min_remaining: i64,
    pub version: String,
}
impl Policy {
    pub fn from_env() -> Result<Self> {
        let read = |key: &str, default: &str| std::env::var(key).unwrap_or_else(|_| default.into());
        Self::parse(
            read("LINJIAN_WX_APP_ID", ""),
            read("LINJIAN_MINIAPP_LINK_TYPE", "short_link"),
            &read("LINJIAN_MINIAPP_SHORT_LINK_PERMANENT", "false"),
            &read("LINJIAN_MINIAPP_URL_LINK_DAYS", "30"),
            &read("LINJIAN_MINIAPP_LINK_MIN_REMAINING_SECONDS", "86400"),
        )
    }
    pub fn parse(
        app_id: String,
        link_type: String,
        permanent: &str,
        days: &str,
        minimum: &str,
    ) -> Result<Self> {
        let permanent = permanent
            .parse::<bool>()
            .map_err(|_| Error::Invalid("小程序永久链接配置无效".into()))?;
        let days = days
            .parse::<i64>()
            .map_err(|_| Error::Invalid("小程序链接有效期配置无效".into()))?;
        let min_remaining = minimum
            .parse::<i64>()
            .map_err(|_| Error::Invalid("小程序链接剩余时间配置无效".into()))?;
        require(
            matches!(link_type.as_str(), "short_link" | "url_link"),
            "小程序链接类型无效",
        )?;
        require(
            (1..=30).contains(&days)
                && min_remaining > 0
                && min_remaining < (if link_type == "short_link" { 30 } else { days }) * 86400,
            "小程序链接有效期范围无效",
        )?;
        require(
            link_type != "url_link" || !permanent,
            "URL Link 不支持永久模式",
        )?;
        let version = hash(
            &json!([
                app_id,
                link_type,
                permanent,
                days,
                min_remaining,
                ALGORITHM,
                "pages/index/detail",
                "page-title-v2"
            ])
            .to_string(),
        );
        Ok(Self {
            app_id,
            link_type,
            permanent,
            days,
            min_remaining,
            version,
        })
    }
    pub fn cache_key(&self, school: Uuid, post: Uuid, title: &str, content: &str) -> String {
        hash(
            &json!([
                self,
                school,
                post,
                "page-title-v2",
                page_title(title, content)
            ])
            .to_string(),
        )
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub id: Uuid,
    pub post_revision: i64,
    pub content_revision: i64,
    pub summary: String,
    pub summary_algorithm: String,
    pub preview_digest: String,
    pub source_projection: String,
}
pub fn preview(post: &Value) -> Result<Preview> {
    let id = crate::business::uuid(post, "id")?;
    let title = post["title"].as_str().unwrap_or("");
    let content = post["content"].as_str().unwrap_or("");
    let summary = summary(content, title);
    let projection = post["source_projection"].as_str().unwrap_or("message_v1");
    let revision = post["revision"].as_i64().ok_or(Error::Conflict)?;
    let content_revision = post["content_revision"].as_i64().unwrap_or(1);
    let digest = hash(
        &json!([
            post["school_id"],
            id,
            revision,
            content_revision,
            title,
            content,
            post["images"],
            post["tags"],
            ALGORITHM,
            summary,
            projection
        ])
        .to_string(),
    );
    Ok(Preview {
        id,
        post_revision: revision,
        content_revision,
        summary,
        summary_algorithm: ALGORITHM.into(),
        preview_digest: digest,
        source_projection: projection.into(),
    })
}
#[derive(Clone)]
pub struct Preparation {
    pub id: Uuid,
    pub token: Uuid,
    pub school: Uuid,
    pub post: Uuid,
    pub title: String,
    pub content: String,
    pub needs_link: bool,
    pub occurred_at: DateTime<Utc>,
    pub route_id: String,
    pub policy: Policy,
    pub attempts: i32,
}
#[derive(Clone)]
pub struct Link {
    pub value: String,
    pub generated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Copy, Debug)]
pub struct Failure {
    pub code: &'static str,
    pub retry: bool,
    pub delay: i64,
}
impl Failure {
    pub fn blocked(code: &'static str) -> Self {
        Self {
            code,
            retry: false,
            delay: 0,
        }
    }
    pub fn transient(code: &'static str) -> Self {
        Self {
            code,
            retry: true,
            delay: 5,
        }
    }
    pub fn upstream(code: i64) -> Self {
        match code {
            40001 | 40014 | 42001 => Self::blocked("token_invalid"),
            43104 | 48001 => Self::blocked("permission_denied"),
            40066 => Self::blocked("page_not_published"),
            40225 => Self::blocked("invalid_page_title"),
            85400 => Self::blocked("permanent_quota_exhausted"),
            45009 => Self {
                code: "daily_quota_exceeded",
                retry: true,
                delay: 86400,
            },
            -1 => Self::transient("upstream_unavailable"),
            _ => Self::blocked("wechat_rejected"),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retry {
    pub reason: String,
    pub revision: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_titles_use_snapshot_body_and_isolated_versioned_cache_keys() {
        assert_eq!(page_title("", " 正文\n次行 "), "正文 次行");
        assert_eq!(page_title("标题", "正文"), "标题");
        assert_eq!(page_title("", ""), "校园动态");
        assert_eq!(page_title("", &"😀".repeat(30)).chars().count(), 20);
        let policy =
            Policy::parse("app".into(), "short_link".into(), "false", "30", "86400").unwrap();
        let school = Uuid::new_v4();
        let post = Uuid::new_v4();
        let original = policy.cache_key(school, post, "", "正文");
        assert_ne!(original, policy.cache_key(school, post, "", "修改正文"));
        assert_ne!(
            original,
            policy.cache_key(school, Uuid::new_v4(), "", "正文")
        );
        assert_ne!(original, policy.cache_key(Uuid::new_v4(), post, "", "正文"));
    }
    #[test]
    fn summary_is_stable_unicode_plain_text_and_bounded() {
        assert_eq!(summary(" a\r\n\t b\u{0000} ", "title"), "a b");
        assert_eq!(summary(" \t ", " 标题 "), "标题");
        assert_eq!(summary("<b>原文</b> **保留**", ""), "<b>原文</b> **保留**");
        assert_eq!(summary(&"😀".repeat(20), "").chars().count(), 20);
        assert_eq!(
            summary(&"😀".repeat(21), ""),
            format!("{}……", "😀".repeat(18))
        );
    }
    #[test]
    fn complete_messages_use_twenty_scalar_summaries_and_opaque_links() {
        for text in [
            "之前有人说军训是 1.10 开始 训……",
            "谁有裁纸刀和钢尺 借我用一晚 照片要……",
            "出个瑜伽垫 紫色的 加厚那种。开学跟……",
            "同班女生说想练人像 让我给她当模特 ……",
        ] {
            assert_eq!(text.chars().count(), 20);
            assert_eq!(
                message(text, "ignored", Some("mp://aHXgY7WQcLjmxYB")).unwrap(),
                format!("{text}\nmp://aHXgY7WQcLjmxYB")
            );
        }
        for count in [18, 19, 20] {
            let text = "😀".repeat(count);
            assert_eq!(message(&text, "", None).unwrap(), text);
        }
        assert_eq!(
            message(&"😀".repeat(21), "", None).unwrap(),
            "😀".repeat(18) + "……"
        );
        assert_eq!(message(" \n", "标题", None).unwrap(), "标题");
        assert_eq!(message(" ", "", None).unwrap_err().code, "content_empty");
        let link = "x".repeat(196);
        let text = message("abcdef", "", Some(&link)).unwrap();
        assert_eq!(text, format!("a……\n{link}"));
        assert_eq!(text.chars().count(), 200);
        assert_eq!(
            message("abcdef", "", Some(&"x".repeat(197)))
                .unwrap_err()
                .code,
            "message_link_too_long"
        );
        assert_eq!(
            message("abc", "", Some("https://example.com/?q=1#part")).unwrap(),
            "abc\nhttps://example.com/?q=1#part"
        );
    }
    #[test]
    fn policies_cannot_create_permanent_url_links_or_impossible_windows() {
        assert!(Policy::parse("app".into(), "url_link".into(), "true", "30", "86400").is_err());
        assert!(Policy::parse("app".into(), "url_link".into(), "false", "1", "86400").is_err());
        let policy =
            Policy::parse("app".into(), "short_link".into(), "false", "30", "86400").unwrap();
        assert_ne!(
            policy.cache_key(Uuid::nil(), Uuid::nil(), "标题", "正文"),
            policy.cache_key(Uuid::nil(), Uuid::nil(), "新标题", "正文")
        );
    }
}
