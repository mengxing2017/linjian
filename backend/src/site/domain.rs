use crate::business::{Error, Result, require};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyrightEndMode {
    Auto,
    Fixed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteConfig {
    pub site_name: Option<String>,
    pub logo_asset_id: Option<Uuid>,
    pub title_subtitle: Option<String>,
    pub login_intro: Option<String>,
    pub icp_number: Option<String>,
    pub police_number: Option<String>,
    pub police_url: Option<String>,
    pub copyright_owner: Option<String>,
    pub copyright_start_year: Option<i32>,
    pub copyright_end_mode: Option<CopyrightEndMode>,
    pub copyright_end_year: Option<i32>,
    pub copyright_notice: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteSettingsInput {
    pub revision: i64,
    pub site_name: Option<String>,
    pub logo_asset_id: Option<Uuid>,
    pub title_subtitle: Option<String>,
    pub login_intro: Option<String>,
    pub icp_number: Option<String>,
    pub police_number: Option<String>,
    pub police_url: Option<String>,
    pub copyright_owner: Option<String>,
    pub copyright_start_year: Option<i32>,
    pub copyright_end_mode: Option<CopyrightEndMode>,
    pub copyright_end_year: Option<i32>,
    pub copyright_notice: Option<String>,
}

const FIELDS: &[&str] = &[
    "revision",
    "site_name",
    "logo_asset_id",
    "title_subtitle",
    "login_intro",
    "icp_number",
    "police_number",
    "police_url",
    "copyright_owner",
    "copyright_start_year",
    "copyright_end_mode",
    "copyright_end_year",
    "copyright_notice",
];

impl SiteSettingsInput {
    /// All settings fields must be present; an absent field is not a clear operation.
    pub fn parse(value: Value) -> Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| Error::Invalid("站点配置无效".into()))?;
        for field in FIELDS {
            require(object.contains_key(*field), "站点配置缺少字段")?;
        }
        serde_json::from_value(value).map_err(|_| Error::Invalid("站点配置字段类型错误".into()))
    }

    pub fn normalize(self, current_year: i32) -> Result<SiteConfig> {
        require(self.revision >= 0, "站点配置版本无效")?;
        let site_name = text(self.site_name.as_deref(), 30, "网站名称")?;
        let title_subtitle = text(self.title_subtitle.as_deref(), 40, "浏览器标题副标题")?;
        let login_intro = text(self.login_intro.as_deref(), 100, "登录页简介")?;
        let icp_number = text(self.icp_number.as_deref(), 64, "ICP备案号")?;
        let police_number = text(self.police_number.as_deref(), 64, "公安联网备案号")?;
        let police_url = match self.police_url.as_deref() {
            Some(value) => {
                let value = value.trim();
                if value.is_empty() {
                    None
                } else {
                    Some(police_url(value)?)
                }
            }
            None => None,
        };
        require(
            police_number.is_some() == police_url.is_some(),
            "公安备案号与查询链接必须同时填写或同时清空",
        )?;
        let copyright_owner = text(self.copyright_owner.as_deref(), 100, "版权所有者")?;
        let copyright_notice = text(self.copyright_notice.as_deref(), 200, "补充版权声明")?;
        let (copyright_start_year, copyright_end_mode, copyright_end_year) =
            match copyright_owner.as_deref() {
                None => {
                    require(
                        self.copyright_start_year.is_none()
                            && self.copyright_end_mode.is_none()
                            && self.copyright_end_year.is_none()
                            && copyright_notice.is_none(),
                        "清空版权所有者时版权信息必须整组清空",
                    )?;
                    (None, None, None)
                }
                Some(_) => {
                    let start = self
                        .copyright_start_year
                        .ok_or_else(|| Error::Invalid("请填写版权起始年".into()))?;
                    require((1900..=current_year).contains(&start), "版权起始年不合法")?;
                    let mode = self
                        .copyright_end_mode
                        .ok_or_else(|| Error::Invalid("请选择版权截止年模式".into()))?;
                    let end = match mode {
                        CopyrightEndMode::Auto => {
                            require(
                                self.copyright_end_year.is_none(),
                                "自动截止年模式不能指定固定年份",
                            )?;
                            None
                        }
                        CopyrightEndMode::Fixed => {
                            let end = self
                                .copyright_end_year
                                .ok_or_else(|| Error::Invalid("请填写固定截止年".into()))?;
                            require((start..=current_year).contains(&end), "固定截止年不合法")?;
                            Some(end)
                        }
                    };
                    (Some(start), Some(mode), end)
                }
            };
        Ok(SiteConfig {
            site_name,
            logo_asset_id: self.logo_asset_id,
            title_subtitle,
            login_intro,
            icp_number,
            police_number,
            police_url,
            copyright_owner,
            copyright_start_year,
            copyright_end_mode,
            copyright_end_year,
            copyright_notice,
        })
    }
}

fn text(value: Option<&str>, max: usize, label: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    require(
        !value
            .chars()
            .any(|c| c.is_control() || c == '<' || c == '>'),
        &format!("{label}不能包含换行、控制字符或 HTML"),
    )?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    require(
        value.chars().count() <= max,
        &format!("{label}超过长度限制"),
    )?;
    Ok(Some(value.to_owned()))
}

fn police_url(value: &str) -> Result<String> {
    require(
        !value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\'),
        "公安备案查询链接无效",
    )?;
    require(value.chars().count() <= 2048, "公安备案查询链接过长")?;
    let parsed =
        url::Url::parse(value).map_err(|_| Error::Invalid("公安备案查询链接无效".into()))?;
    require(parsed.scheme() == "https", "公安备案查询链接必须使用 HTTPS")?;
    require(
        parsed.username().is_empty() && parsed.password().is_none(),
        "公安备案查询链接无效",
    )?;
    require(
        parsed
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("beian.mps.gov.cn")),
        "公安备案查询链接必须指向 beian.mps.gov.cn",
    )?;
    require(
        parsed.port().is_none() || parsed.port() == Some(443),
        "公安备案查询链接端口无效",
    )?;
    Ok(value.to_owned())
}

impl SiteConfig {
    pub fn validate(&self, current_year: i32) -> Result<()> {
        require(
            self.police_number.is_some() == self.police_url.is_some(),
            "公安备案号与查询链接必须同时填写或同时清空",
        )?;
        match self.copyright_owner.as_deref() {
            None => require(
                self.copyright_start_year.is_none()
                    && self.copyright_end_mode.is_none()
                    && self.copyright_end_year.is_none()
                    && self.copyright_notice.is_none(),
                "版权信息必须整组填写或整组清空",
            ),
            Some(_) => {
                let start = self
                    .copyright_start_year
                    .ok_or_else(|| Error::Invalid("请填写版权起始年".into()))?;
                require((1900..=current_year).contains(&start), "版权起始年不合法")?;
                match self.copyright_end_mode {
                    Some(CopyrightEndMode::Auto) => require(
                        self.copyright_end_year.is_none(),
                        "自动截止年模式不能指定固定年份",
                    ),
                    Some(CopyrightEndMode::Fixed) => {
                        let end = self
                            .copyright_end_year
                            .ok_or_else(|| Error::Invalid("请填写固定截止年".into()))?;
                        require((start..=current_year).contains(&end), "固定截止年不合法")
                    }
                    None => Err(Error::Invalid("请选择版权截止年模式".into())),
                }
            }
        }
    }

    pub fn copyright_years(&self, current_year: i32) -> Option<String> {
        let start = self.copyright_start_year?;
        let end = match self.copyright_end_mode? {
            CopyrightEndMode::Auto => current_year,
            CopyrightEndMode::Fixed => self.copyright_end_year?,
        };
        Some(if end <= start {
            start.to_string()
        } else {
            format!("{start}\u{2013}{end}")
        })
    }

    pub fn public(&self, revision: i64, current_year: i32) -> PublicSiteSettings {
        PublicSiteSettings {
            revision,
            site_name: self.site_name.clone(),
            logo_url: self
                .logo_asset_id
                .map(|_| format!("/api/v1/site-branding/logo?v={revision}")),
            title_subtitle: self.title_subtitle.clone(),
            login_intro: self.login_intro.clone(),
            icp_number: self.icp_number.clone(),
            police_number: self.police_number.clone(),
            police_url: self.police_url.clone(),
            copyright_owner: self.copyright_owner.clone(),
            copyright_years: self.copyright_years(current_year),
            copyright_notice: self.copyright_notice.clone(),
        }
    }

    pub fn admin(
        &self,
        revision: i64,
        current_year: i32,
        updated_at: DateTime<Utc>,
        updated_by_name: Option<String>,
        history: Vec<SiteSettingsHistory>,
    ) -> AdminSiteSettings {
        AdminSiteSettings {
            revision,
            site_name: self.site_name.clone(),
            logo_asset_id: self.logo_asset_id,
            logo_url: self
                .logo_asset_id
                .map(|_| format!("/api/v1/site-branding/logo?v={revision}")),
            title_subtitle: self.title_subtitle.clone(),
            login_intro: self.login_intro.clone(),
            icp_number: self.icp_number.clone(),
            police_number: self.police_number.clone(),
            police_url: self.police_url.clone(),
            copyright_owner: self.copyright_owner.clone(),
            copyright_start_year: self.copyright_start_year,
            copyright_end_mode: self.copyright_end_mode,
            copyright_end_year: self.copyright_end_year,
            copyright_years: self.copyright_years(current_year),
            copyright_notice: self.copyright_notice.clone(),
            updated_at,
            updated_by_name,
            history,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicSiteSettings {
    pub revision: i64,
    pub site_name: Option<String>,
    pub logo_url: Option<String>,
    pub title_subtitle: Option<String>,
    pub login_intro: Option<String>,
    pub icp_number: Option<String>,
    pub police_number: Option<String>,
    pub police_url: Option<String>,
    pub copyright_owner: Option<String>,
    pub copyright_years: Option<String>,
    pub copyright_notice: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteSettingsHistory {
    pub id: Uuid,
    pub actor_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub revision: i64,
    pub changed: Vec<String>,
    pub cleared: Vec<String>,
    pub previous: Value,
    pub current: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminSiteSettings {
    pub revision: i64,
    pub site_name: Option<String>,
    pub logo_asset_id: Option<Uuid>,
    pub logo_url: Option<String>,
    pub title_subtitle: Option<String>,
    pub login_intro: Option<String>,
    pub icp_number: Option<String>,
    pub police_number: Option<String>,
    pub police_url: Option<String>,
    pub copyright_owner: Option<String>,
    pub copyright_start_year: Option<i32>,
    pub copyright_end_mode: Option<CopyrightEndMode>,
    pub copyright_end_year: Option<i32>,
    pub copyright_years: Option<String>,
    pub copyright_notice: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: Option<String>,
    pub history: Vec<SiteSettingsHistory>,
}

#[derive(Clone, Debug)]
pub struct StoredSiteSettings {
    pub id: Uuid,
    pub revision: i64,
    pub config: SiteConfig,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: Option<String>,
}

impl StoredSiteSettings {
    pub fn public(&self, current_year: i32) -> PublicSiteSettings {
        self.config.public(self.revision, current_year)
    }

    pub fn admin(&self, current_year: i32, history: Vec<SiteSettingsHistory>) -> AdminSiteSettings {
        self.config.admin(
            self.revision,
            current_year,
            self.updated_at,
            self.updated_by_name.clone(),
            history,
        )
    }
}

pub fn business_year() -> i32 {
    use chrono::{Datelike, FixedOffset};
    Utc::now()
        .with_timezone(&FixedOffset::east_opt(8 * 3600).expect("valid offset"))
        .year()
}

pub fn diff(previous: &SiteConfig, current: &SiteConfig) -> (Vec<String>, Vec<String>) {
    let previous = serde_json::to_value(previous).unwrap_or(Value::Null);
    let current = serde_json::to_value(current).unwrap_or(Value::Null);
    let mut changed = Vec::new();
    let mut cleared = Vec::new();
    for field in FIELDS.iter().filter(|field| **field != "revision") {
        let before = previous.get(*field).unwrap_or(&Value::Null);
        let after = current.get(*field).unwrap_or(&Value::Null);
        if before == after {
            continue;
        }
        changed.push((*field).to_owned());
        if !before.is_null() && after.is_null() {
            cleared.push((*field).to_owned());
        }
    }
    (changed, cleared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input(value: Value) -> SiteSettingsInput {
        SiteSettingsInput::parse(value).unwrap()
    }

    #[test]
    fn all_fields_are_required_and_empty_values_normalize_to_null() {
        assert!(
            SiteSettingsInput::parse(json!({"revision":0,"logo_asset_id":null,"site_name":null}))
                .is_err(),
            "an absent field must not be treated as a clear operation"
        );
        let value = input(json!({
            "revision":0,"logo_asset_id":null,"site_name":"  示例校园  ","title_subtitle":"  ","login_intro":null,
            "icp_number":null,"police_number":null,"police_url":null,"copyright_owner":null,
            "copyright_start_year":null,"copyright_end_mode":null,"copyright_end_year":null,
            "copyright_notice":null
        }));
        let config = value.normalize(2026).unwrap();
        assert_eq!(config.site_name.as_deref(), Some("示例校园"));
        assert_eq!(config.title_subtitle, None);
    }

    #[test]
    fn text_and_police_link_are_explicit() {
        let base = json!({
            "revision":0,"logo_asset_id":null,"site_name":"<b>x</b>","title_subtitle":null,"login_intro":null,
            "icp_number":null,"police_number":null,"police_url":null,"copyright_owner":null,
            "copyright_start_year":null,"copyright_end_mode":null,"copyright_end_year":null,
            "copyright_notice":null
        });
        assert!(input(base).normalize(2026).is_err());

        for url in [
            "http://beian.mps.gov.cn/x",
            "https://evil.test/x",
            "https://beian.mps.gov.cn.evil.test/x",
            "https://user@beian.mps.gov.cn/x",
            "https://beian.mps.gov.cn:8443/x",
        ] {
            let mut value = json!({
                "revision":0,"logo_asset_id":null,"site_name":null,"title_subtitle":null,"login_intro":null,
                "icp_number":null,"police_number":"京公网安备 11000000000000号","police_url":url,
                "copyright_owner":null,"copyright_start_year":null,"copyright_end_mode":null,
                "copyright_end_year":null,"copyright_notice":null
            });
            value["police_url"] = json!(url);
            assert!(input(value).normalize(2026).is_err(), "must reject {url}");
        }

        let value = input(json!({
            "revision":0,"logo_asset_id":null,"site_name":"示例校园","title_subtitle":null,"login_intro":null,
            "icp_number":"京ICP备00000000号-1","police_number":"京公网安备 11000000000000号",
            "police_url":"https://beian.mps.gov.cn/#/query/webSearch?code=1",
            "copyright_owner":null,"copyright_start_year":null,"copyright_end_mode":null,
            "copyright_end_year":null,"copyright_notice":null
        }));
        let config = value.normalize(2026).unwrap();
        assert_eq!(
            config.police_url.as_deref(),
            Some("https://beian.mps.gov.cn/#/query/webSearch?code=1")
        );
    }

    #[test]
    fn copyright_group_and_years_are_consistent() {
        let value = input(json!({
            "revision":0,"logo_asset_id":null,"site_name":null,"title_subtitle":null,"login_intro":null,
            "icp_number":null,"police_number":null,"police_url":null,
            "copyright_owner":"示例运营主体","copyright_start_year":2024,
            "copyright_end_mode":"auto","copyright_end_year":null,"copyright_notice":"保留权利"
        }));
        let config = value.normalize(2026).unwrap();
        assert_eq!(
            config.copyright_years(2026).as_deref(),
            Some("2024\u{2013}2026")
        );
        assert_eq!(config.copyright_years(2024).as_deref(), Some("2024"));

        for invalid in [
            json!({"copyright_owner":null,"copyright_start_year":2024,"copyright_end_mode":null,"copyright_end_year":null,"copyright_notice":null}),
            json!({"copyright_owner":"主体","copyright_start_year":2024,"copyright_end_mode":"fixed","copyright_end_year":null,"copyright_notice":null}),
            json!({"copyright_owner":"主体","copyright_start_year":2025,"copyright_end_mode":"fixed","copyright_end_year":2024,"copyright_notice":null}),
            json!({"copyright_owner":"主体","copyright_start_year":2027,"copyright_end_mode":"auto","copyright_end_year":null,"copyright_notice":null}),
        ] {
            let mut base = json!({
                "revision":0,"logo_asset_id":null,"site_name":null,"title_subtitle":null,"login_intro":null,
                "icp_number":null,"police_number":null,"police_url":null,
                "copyright_owner":null,"copyright_start_year":null,"copyright_end_mode":null,
                "copyright_end_year":null,"copyright_notice":null
            });
            for (key, item) in invalid.as_object().unwrap() {
                base[key] = item.clone();
            }
            assert!(input(base).normalize(2026).is_err());
        }
    }

    #[test]
    fn public_projection_omits_management_fields() {
        let config = SiteConfig {
            site_name: Some("示例校园".into()),
            copyright_owner: Some("示例运营主体".into()),
            copyright_start_year: Some(2022),
            copyright_end_mode: Some(CopyrightEndMode::Fixed),
            copyright_end_year: Some(2024),
            ..Default::default()
        };
        let public = config.public(7, 2026);
        assert_eq!(public.revision, 7);
        assert_eq!(public.copyright_years.as_deref(), Some("2022\u{2013}2024"));
        let value = serde_json::to_value(public).unwrap();
        assert!(value.get("updated_by_name").is_none());
        assert!(value.get("copyright_end_mode").is_none());
        assert!(value.get("logo_asset_id").is_none());
    }
}
