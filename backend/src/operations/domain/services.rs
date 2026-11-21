use crate::{
    business::{Page, Result, require, text},
    schools::domain::{ImageRef, ResourceAction},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelAction {
    MiniProgram { app_id: String, path: String },
    WebView { url: String },
    CopyLink { url: String },
    QrCode { asset_id: Uuid },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub key: String,
    pub label: String,
    pub action: ChannelAction,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInput {
    #[serde(default)]
    pub school_ids: Vec<Uuid>,
    pub partner_id: Option<Uuid>,
    pub cooperation_id: Option<Uuid>,
    pub revision: i64,
    pub name: String,
    pub provider: String,
    pub purpose: String,
    pub disclosure: String,
    pub contact: String,
    pub complaint: String,
    pub authorization: String,
    pub qualification: String,
    pub channels: Vec<Channel>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShortcutAction {
    None,
    InternalPage {
        page: String,
        object_id: Option<Uuid>,
    },
    TargetChannel {
        version_id: Uuid,
        channel: String,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShortcutInput {
    pub revision: i64,
    pub title: String,
    pub subtitle: String,
    pub badge: String,
    pub image_ref: ImageRef,
    pub category_id: Option<Uuid>,
    pub cooperation_id: Option<Uuid>,
    pub action: ShortcutAction,
    pub active: bool,
    pub featured: bool,
    pub sort_order: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    #[serde(
        default,
        deserialize_with = "category_overlay",
        skip_serializing_if = "Option::is_none"
    )]
    pub category_id: Option<Option<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_ref: Option<ImageRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
}
fn category_overlay<'de, Deserializer: serde::Deserializer<'de>>(
    deserializer: Deserializer,
) -> std::result::Result<Option<Option<Uuid>>, Deserializer::Error> {
    Ok(Some(Option::<Uuid>::deserialize(deserializer)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideInput {
    pub revision: i64,
    pub token: String,
    pub presentation: Presentation,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionInput {
    pub target_id: Option<Uuid>,
    pub target_revision: Option<i64>,
    pub revision: i64,
    pub decision: String,
    pub reason: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryInput {
    pub revision: i64,
    pub name: String,
    pub active: bool,
    pub sort_order: i32,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PartnerInput {
    pub revision: i64,
    pub name: String,
    pub contact: String,
    pub active: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CooperationInput {
    #[serde(default)]
    pub school_ids: Vec<Uuid>,
    pub revision: i64,
    pub partner_id: Uuid,
    pub name: String,
    pub model: String,
    pub disclosure: String,
    pub channel_code: String,
    pub active: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketingInput {
    pub revision: i64,
    pub name: String,
    pub ad_unit_id: String,
    pub format: String,
    pub reward_scene: Option<String>,
    pub qualification: String,
    pub active: bool,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CatalogQuery {
    #[serde(rename = "page[number]")]
    pub number: Option<i64>,
    #[serde(rename = "page[size]")]
    pub size: Option<i64>,
    pub search: Option<String>,
    pub category_id: Option<Uuid>,
    pub at: Option<DateTime<Utc>>,
}
impl CatalogQuery {
    pub fn page(&self) -> Page {
        Page {
            page: self.number,
            page_size: self.size,
            ..Default::default()
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricInput {
    pub event_id: Uuid,
    pub resource_id: Uuid,
    pub event_type: String,
    pub display_cycle: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub channel: Option<String>,
    pub target_version_id: Option<Uuid>,
    pub placement: String,
    pub attempt_id: Option<Uuid>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeInput {
    pub event_id: Uuid,
    pub click_id: Uuid,
    pub outcome: String,
    pub occurred_at: DateTime<Utc>,
    pub stage: String,
    pub attempt_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchInput {
    pub token: String,
    pub items: Vec<BatchItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchItem {
    pub id: Uuid,
    pub revision: i64,
    pub active: Option<bool>,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportInput {
    pub partner_id: Option<Uuid>,
    #[serde(default)]
    pub school_ids: Vec<Uuid>,
    pub resource_id: Option<Uuid>,
    pub target_version_id: Option<Uuid>,
    pub cooperation_id: Option<Uuid>,
    pub placement: Option<String>,
    pub channel: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftPreviewInput {
    pub token: String,
    pub at: DateTime<Utc>,
    pub resource_id: Option<Uuid>,
    pub platform_default: bool,
    pub draft: ShortcutInput,
}
pub enum ServiceCommand {
    Options,
    ManageDetail {
        entity: String,
        id: Uuid,
    },
    ManageList {
        entity: String,
        query: CatalogQuery,
    },
    Save {
        entity: String,
        id: Option<Uuid>,
        input: serde_json::Value,
    },
    Delete {
        entity: String,
        id: Uuid,
        revision: i64,
    },
    TargetDecision {
        id: Uuid,
        input: DecisionInput,
    },
    Catalog(CatalogQuery),
    Detail(Uuid),
    Resolve {
        id: Uuid,
        version: Option<Uuid>,
        channel: Option<String>,
    },
    Override {
        id: Uuid,
        input: OverrideInput,
    },
    Reset {
        id: Uuid,
        revision: i64,
    },
    Batch {
        reorder: bool,
        input: BatchInput,
    },
    Preview(CatalogQuery),
    DraftPreview(DraftPreviewInput),
    Metric(MetricInput),
    Outcome(OutcomeInput),
    Media {
        id: Uuid,
        asset: Uuid,
        version: Option<Uuid>,
        channel: Option<String>,
    },
    Report(ReportInput),
    ReadReport {
        id: Uuid,
        export: bool,
        page: Page,
    },
    Marketing,
    MarketingSettings,
    MarketingEvent(MarketingEvent),
    MarketingSetting {
        id: Uuid,
        revision: i64,
        enabled: bool,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketingEvent {
    pub event_id: Uuid,
    pub slot_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
}
pub fn https_url(value: &str) -> Result<()> {
    let parsed =
        url::Url::parse(value).map_err(|_| crate::business::Error::Invalid("链接无效".into()))?;
    require(
        value.len() <= 2048
            && parsed.scheme() == "https"
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.fragment().is_none()
            && parsed.port().is_none(),
        "仅支持无凭据的 HTTPS 链接",
    )?;
    let host = parsed.host_str().unwrap_or("");
    require(
        host.contains('.')
            && host != "localhost"
            && !host.ends_with(".localhost")
            && !host.ends_with(".local")
            && host.parse::<std::net::IpAddr>().is_err(),
        "须配置经过核验的业务域名",
    )
}
fn timestamp(value: DateTime<Utc>) -> Result<()> {
    let seconds = (Utc::now() - value).num_seconds();
    require((-60..=7 * 86400).contains(&seconds), "事件时间超出接收窗口")
}
pub fn decode<T: serde::de::DeserializeOwned>(value: &serde_json::Value) -> Result<T> {
    serde_json::from_value(value.clone())
        .map_err(|_| crate::business::Error::Invalid("配置字段或类型无效".into()))
}
impl TargetInput {
    pub fn validate(&self) -> Result<()> {
        require(self.revision >= 0, "版本无效")?;
        require(self.school_ids.len() <= 200, "目标学校范围过多")?;
        for value in [
            &self.name,
            &self.provider,
            &self.purpose,
            &self.disclosure,
            &self.contact,
            &self.complaint,
            &self.authorization,
            &self.qualification,
        ] {
            text(value, 1, 2000)?;
        }
        require(
            !self.channels.is_empty() && self.channels.len() <= 8,
            "须配置一至八个服务渠道",
        )?;
        let mut keys = std::collections::BTreeSet::new();
        for channel in &self.channels {
            require(
                !channel.key.is_empty()
                    && channel.key.len() <= 40
                    && channel
                        .key
                        .bytes()
                        .all(|value| value.is_ascii_alphanumeric() || value == b'_')
                    && keys.insert(&channel.key),
                "渠道标识无效或重复",
            )?;
            text(&channel.label, 1, 40)?;
            match &channel.action {
                ChannelAction::MiniProgram { app_id, path } => {
                    require(
                        app_id.len() == 18
                            && app_id.starts_with("wx")
                            && app_id[2..].bytes().all(|value| value.is_ascii_hexdigit()),
                        "小程序 AppID 无效",
                    )?;
                    require(
                        path.len() <= 1024
                            && path.starts_with("pages/")
                            && !path.contains(['#', '\\', '\n', '\r'])
                            && !path.contains(".."),
                        "小程序路径无效",
                    )?;
                }
                ChannelAction::WebView { url } | ChannelAction::CopyLink { url } => https_url(url)?,
                ChannelAction::QrCode { .. } => {}
            }
        }
        Ok(())
    }
}
impl ShortcutInput {
    pub fn validate(&self) -> Result<()> {
        require(self.revision >= 0, "版本无效")?;
        text(&self.title, 1, 40)?;
        text(&self.subtitle, 0, 80)?;
        text(&self.badge, 0, 12)?;
        require(
            self.starts_at
                .zip(self.ends_at)
                .is_none_or(|(start, end)| start < end),
            "时间范围无效",
        )?;
        match &self.action {
            ShortcutAction::InternalPage { page, object_id } => ResourceAction::InternalPage {
                page: page.clone(),
                object_id: *object_id,
            }
            .validate(),
            ShortcutAction::TargetChannel { channel, .. } => {
                text(channel, 1, 40)?;
                Ok(())
            }
            ShortcutAction::None => Ok(()),
        }
    }
}
impl ServiceCommand {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::DraftPreview(input) => input.draft.validate()?,
            Self::Catalog(query) | Self::Preview(query) | Self::ManageList { query, .. } => {
                require(
                    !matches!(self, Self::Catalog(_)) || query.at.is_none(),
                    "成员展示不允许指定预览时间",
                )?;
                query.page().bounds()?;
                if let Some(value) = &query.search {
                    text(value, 0, 100)?;
                }
            }
            Self::Save { entity, input, .. } => match entity.as_str() {
                "home-shortcuts" => decode::<ShortcutInput>(input)?.validate()?,
                "operation-service-targets" => decode::<TargetInput>(input)?.validate()?,
                "operation-service-categories" => {
                    let value: CategoryInput = decode(input)?;
                    text(&value.name, 1, 40)?;
                    require(value.revision >= 0, "版本无效")?;
                }
                "operation-partners" => {
                    let value: PartnerInput = decode(input)?;
                    text(&value.name, 1, 100)?;
                    text(&value.contact, 1, 200)?;
                    require(value.revision >= 0, "版本无效")?;
                }
                "operation-cooperations" => {
                    let value: CooperationInput = decode(input)?;
                    require(value.school_ids.len() <= 200, "合作学校范围过多")?;
                    text(&value.name, 1, 100)?;
                    text(&value.disclosure, 1, 500)?;
                    require(
                        value.revision >= 0
                            && matches!(value.model.as_str(), "service" | "sponsor" | "cps")
                            && value
                                .starts_at
                                .zip(value.ends_at)
                                .is_none_or(|(start, end)| start < end),
                        "合作模式或期限无效",
                    )?;
                    require(
                        value.channel_code.len() <= 80
                            && value.channel_code.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                            }),
                        "仅允许非个人化渠道代码",
                    )?;
                }
                "marketing-slots" => {
                    let value: MarketingInput = decode(input)?;
                    text(&value.name, 1, 100)?;
                    text(&value.qualification, 1, 2000)?;
                    require(
                        value.revision >= 0
                            && matches!(value.format.as_str(), "custom" | "rewarded")
                            && value.ad_unit_id.starts_with("adunit-")
                            && value.ad_unit_id.len() <= 80
                            && value
                                .ad_unit_id
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
                        "官方单元配置无效",
                    )?;
                    require(
                        if value.format == "rewarded" {
                            value.reward_scene.as_deref() == Some("service_theme")
                        } else {
                            value.reward_scene.is_none()
                        },
                        "仅支持无资产价值的本次服务区主题权益",
                    )?;
                }
                _ => return Err(crate::business::Error::NotFound),
            },
            Self::Metric(value) => {
                require(
                    matches!(value.event_type.as_str(), "view" | "click"),
                    "事件类型无效",
                )?;
                timestamp(value.occurred_at)?;
                require(
                    matches!(
                        value.placement.as_str(),
                        "home_banner" | "home_shortcut" | "more_services" | "service_detail"
                    ),
                    "展示位置无效",
                )?;
                require(
                    (value.event_type == "click") == value.attempt_id.is_some(),
                    "点击必须关联独立 attempt，曝光不得指定 attempt",
                )?;
            }
            Self::MarketingEvent(value) => {
                require(
                    matches!(
                        value.event_type.as_str(),
                        "loaded"
                            | "error"
                            | "show_api_success"
                            | "closed_early"
                            | "client_completed"
                    ),
                    "营销事件类型无效",
                )?;
                timestamp(value.occurred_at)?;
            }
            Self::Outcome(value) => {
                require(
                    matches!(
                        value.outcome.as_str(),
                        "api_success"
                            | "cancel"
                            | "fail"
                            | "copy_success"
                            | "qr_displayed"
                            | "webview_loaded"
                            | "webview_failed"
                    ),
                    "阶段结果无效",
                )?;
                timestamp(value.occurred_at)?;
                require(
                    matches!(
                        value.stage.as_str(),
                        "navigation_api" | "clipboard_api" | "qr_display" | "webview_load"
                    ),
                    "动作阶段无效",
                )?;
            }
            Self::TargetDecision { input, .. } => {
                require(
                    input.revision >= 0
                        && matches!(
                            input.decision.as_str(),
                            "submit" | "approve" | "reject" | "withdraw"
                        ),
                    "审核动作无效",
                )?;
                text(&input.reason, 1, 1000)?;
            }
            Self::Override { input, .. } => {
                require(input.revision >= 0, "版本无效")?;
                for (value, minimum, maximum) in [
                    (&input.presentation.title, 1, 40),
                    (&input.presentation.subtitle, 0, 80),
                    (&input.presentation.badge, 0, 12),
                ] {
                    if let Some(value) = value {
                        text(value, minimum, maximum)?;
                    }
                }
            }
            Self::Batch { input, .. } => {
                require(
                    !input.items.is_empty() && input.items.len() <= 1000,
                    "批量范围无效",
                )?;
                let unique: std::collections::BTreeSet<_> =
                    input.items.iter().map(|value| value.id).collect();
                require(
                    unique.len() == input.items.len()
                        && input.items.iter().all(|value| value.revision >= 0),
                    "重复项目或版本无效",
                )?;
            }
            Self::ReadReport { page, .. } => {
                page.bounds()?;
            }
            Self::Report(input) => {
                require(input.school_ids.len() <= 200, "报表最多选择两百所学校")?;
                require(
                    input.placement.as_deref().is_none_or(|value| {
                        matches!(
                            value,
                            "home_banner" | "home_shortcut" | "more_services" | "service_detail"
                        )
                    }),
                    "报表展示位置无效",
                )?;
                if let Some(channel) = &input.channel {
                    text(channel, 1, 40)?;
                }
                require(
                    input.starts_at < input.ends_at
                        && (input.ends_at - input.starts_at).num_seconds() <= 365 * 86400,
                    "报表范围须在一年内",
                )?;
                for time in [input.starts_at, input.ends_at] {
                    require(
                        (time.timestamp() + 8 * 3600).rem_euclid(86400) == 0
                            && time.timestamp_subsec_nanos() == 0,
                        "报表边界必须为上海自然日零点",
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_are_not_arbitrary_scheme_executors() {
        for value in [
            "http://example.com",
            "https://127.0.0.1",
            "https://user:secret@example.com",
            "weixin://x",
            "https://a.local/x",
            "https://example.com/#x",
        ] {
            assert!(https_url(value).is_err(), "{value}");
        }
        assert!(https_url("https://service.example.com/help?channel=school").is_ok());
    }
    #[test]
    fn shortcut_cannot_supply_raw_external_payloads() {
        assert!(
            serde_json::from_value::<ShortcutAction>(
                serde_json::json!({"kind":"web_view","url":"https://example.com"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<ChannelAction>(
                serde_json::json!({"kind":"copy_link","url":"https://example.com","secret":"x"})
            )
            .is_err()
        );
    }
    #[test]
    fn members_cannot_use_future_preview_time() {
        let query = CatalogQuery {
            at: Some(Utc::now() + chrono::Duration::days(1)),
            ..Default::default()
        };
        assert!(ServiceCommand::Catalog(query).validate().is_err());
    }
    #[test]
    fn category_overlay_distinguishes_inherit_clear_and_replace() {
        let inherit: Presentation = decode(&serde_json::json!({})).unwrap();
        let clear: Presentation = decode(&serde_json::json!({"category_id":null})).unwrap();
        let category = Uuid::new_v4();
        let replace: Presentation = decode(&serde_json::json!({"category_id":category})).unwrap();
        assert_eq!(inherit.category_id, None);
        assert_eq!(clear.category_id, Some(None));
        assert_eq!(replace.category_id, Some(Some(category)));
        assert_eq!(
            serde_json::to_value(clear).unwrap(),
            serde_json::json!({"category_id":null})
        );
    }
    #[test]
    fn unified_metrics_reject_stale_events_and_missing_click_attempts() {
        let event = MetricInput {
            event_id: Uuid::new_v4(),
            resource_id: Uuid::new_v4(),
            event_type: "view".into(),
            display_cycle: Uuid::new_v4(),
            occurred_at: Utc::now() - chrono::Duration::days(8),
            channel: None,
            target_version_id: None,
            placement: "home_banner".into(),
            attempt_id: None,
        };
        assert!(ServiceCommand::Metric(event.clone()).validate().is_err());
        let click = MetricInput {
            event_type: "click".into(),
            occurred_at: Utc::now(),
            ..event
        };
        assert!(ServiceCommand::Metric(click).validate().is_err());
    }
}
