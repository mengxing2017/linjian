use crate::{
    business::{Page, Result, require, text},
    schools::domain::{ImageRef, ResourceAction},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    #[serde(rename = "page[number]")]
    pub number: Option<i64>,
    #[serde(rename = "page[size]")]
    pub size: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub strategy: Option<String>,
    pub action: Option<String>,
    pub feedback_type: Option<String>,
    pub contact: Option<String>,
    pub user: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub defaults: bool,
}
impl Query {
    pub fn page(&self) -> Page {
        Page {
            page: self.number,
            page_size: self.size,
            ..Default::default()
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.page().bounds()?;
        for v in [&self.search, &self.contact, &self.user]
            .into_iter()
            .flatten()
        {
            text(v, 0, 200)?;
        }
        require(
            self.status.as_deref().is_none_or(|s| {
                matches!(
                    s,
                    "" | "normal" | "hidden" | "open" | "resolved" | "dismissed"
                )
            }),
            "状态无效",
        )?;
        require(
            self.strategy
                .as_deref()
                .is_none_or(|s| matches!(s, "" | "always" | "daily" | "interval")),
            "展示时机无效",
        )?;
        require(
            self.feedback_type
                .as_deref()
                .is_none_or(|s| s.is_empty() || feedback_type(s) || s == "legacy"),
            "反馈类型无效",
        )?;
        require(
            self.action.as_deref().is_none_or(|s| {
                matches!(
                    s,
                    "" | "none" | "internal_page" | "external_link" | "phone" | "copy_text"
                )
            }),
            "跳转类型无效",
        )?;
        require(
            self.starts_at.zip(self.ends_at).is_none_or(|(a, b)| a < b),
            "时间范围无效",
        )
    }
}
pub fn feedback_type(s: &str) -> bool {
    matches!(s, "product_suggestion" | "bug_report" | "other")
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceInput {
    #[serde(default)]
    pub revision: i64,
    pub title: String,
    #[serde(default)]
    pub content: String,
    pub image_ref: ImageRef,
    #[serde(default)]
    pub action: ResourceAction,
    pub active: bool,
    #[serde(default)]
    pub sort_order: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    #[serde(default = "daily")]
    pub popup_strategy: String,
    #[serde(default = "one")]
    pub popup_interval_days: i32,
}
fn daily() -> String {
    "daily".into()
}
fn one() -> i32 {
    1
}
impl ResourceInput {
    pub fn validate(&mut self, kind: &str) -> Result<()> {
        self.title = text(&self.title, 1, 100)?;
        self.content = text(&self.content, 0, if kind == "qrcode" { 255 } else { 20000 })?;
        require(self.revision >= 0, "版本无效")?;
        require(
            matches!(
                self.popup_strategy.as_str(),
                "always" | "daily" | "interval"
            ),
            "展示时机无效",
        )?;
        require(
            (1..=365).contains(&self.popup_interval_days),
            "间隔天数须为1至365",
        )?;
        if self.popup_strategy != "interval" {
            self.popup_interval_days = 1;
        }
        require(
            self.starts_at.zip(self.ends_at).is_none_or(|(a, b)| a < b),
            "结束展示时间必须晚于开始",
        )?;
        self.action.validate()?;
        if kind == "qrcode" {
            require(
                matches!(self.action, ResourceAction::None {}),
                "群二维码只提供图片展示",
            )?;
        }
        Ok(())
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomerInput {
    pub revision: i64,
    pub enabled: bool,
    pub business_asset_id: Option<Uuid>,
    pub join_asset_id: Option<Uuid>,
    pub service_asset_id: Option<Uuid>,
    pub fallback_asset_id: Option<Uuid>,
    pub note: String,
}
impl CustomerInput {
    pub fn validate(&mut self) -> Result<()> {
        require(self.revision >= 0, "版本无效")?;
        self.note = text(&self.note, 0, 255)?;
        if self.note.is_empty() {
            self.note = "添加客服微信后，请备注来意".into();
        }
        Ok(())
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackInput {
    pub feedback_type: String,
    pub content: String,
    pub contact: String,
    #[serde(default)]
    pub images: Vec<String>,
    pub submission_key: Uuid,
}
impl FeedbackInput {
    pub fn validate(&mut self) -> Result<()> {
        require(feedback_type(&self.feedback_type), "请选择反馈类型")?;
        self.content = text(&self.content, 1, 500)?;
        self.contact = text(&self.contact, 1, 120)?;
        require(self.images.len() <= 9, "最多上传九张图片")
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BannerEvent {
    pub event_id: Uuid,
    pub resource_id: Uuid,
    pub event_type: String,
    pub display_cycle: Uuid,
    pub occurred_at: DateTime<Utc>,
}
impl BannerEvent {
    pub fn validate(&self) -> Result<()> {
        require(
            matches!(self.event_type.as_str(), "view" | "click"),
            "事件类型无效",
        )?;
        let delta = Utc::now() - self.occurred_at;
        require(
            delta.num_seconds() >= -60 && delta.num_seconds() <= 7 * 86400,
            "统计事件已过期",
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn feedback_requires_known_type_and_contact() {
        let mut v = FeedbackInput {
            feedback_type: "other".into(),
            content: "建议".into(),
            contact: "  ".into(),
            images: vec![],
            submission_key: Uuid::new_v4(),
        };
        assert!(v.validate().is_err());
        v.contact = "微信".into();
        assert!(v.validate().is_ok());
        v.feedback_type = "invented".into();
        assert!(v.validate().is_err());
    }
    #[test]
    fn events_reject_stale_replays() {
        let v = BannerEvent {
            event_id: Uuid::new_v4(),
            resource_id: Uuid::new_v4(),
            event_type: "view".into(),
            display_cycle: Uuid::new_v4(),
            occurred_at: Utc::now() - chrono::Duration::days(8),
        };
        assert!(v.validate().is_err());
    }
}

/// Walk the proxy chain from the socket peer, stopping at the first untrusted hop.
/// Missing peers and malformed chains never promote an unverified header to a client address.
pub fn client_ip(
    peer: Option<std::net::IpAddr>,
    forwarded: Option<&str>,
    trusted: &[std::net::IpAddr],
) -> Option<std::net::IpAddr> {
    let mut current = peer?;
    if !trusted.contains(&current) {
        return Some(current);
    }
    let Some(chain) = forwarded else {
        return Some(current);
    };
    if chain.len() > 2048 {
        return Some(current);
    }
    let hops = chain
        .split(',')
        .map(|s| s.trim().parse::<std::net::IpAddr>())
        .collect::<std::result::Result<Vec<_>, _>>();
    let Ok(hops) = hops else { return Some(current) };
    for hop in hops.into_iter().rev() {
        if !trusted.contains(&current) {
            break;
        }
        current = hop;
    }
    Some(current)
}
#[cfg(test)]
mod ip_tests {
    use super::client_ip;
    #[test]
    fn only_explicit_trusted_hops_can_supply_client_addresses() {
        let proxy = "127.0.0.1".parse().unwrap();
        let client = "192.0.2.1".parse().unwrap();
        let attacker = "203.0.113.1".parse().unwrap();
        assert_eq!(
            client_ip(Some(client), Some("203.0.113.1"), &[proxy]),
            Some(client)
        );
        assert_eq!(
            client_ip(Some(proxy), Some("203.0.113.1, 192.0.2.1"), &[proxy]),
            Some(client)
        );
        assert_eq!(
            client_ip(Some(proxy), Some("203.0.113.1"), &[proxy]),
            Some(attacker)
        );
        assert_eq!(client_ip(None, Some("203.0.113.1"), &[proxy]), None);
        assert_eq!(
            client_ip(Some(proxy), Some("invalid"), &[proxy]),
            Some(proxy)
        );
    }
}
