use crate::business::{Error, Result, require, text};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct UserView {
    pub id: Uuid,
    pub avatar_url: Option<String>,
    pub avatar_asset_id: Option<Uuid>,
    pub nickname: String,
    pub mobile: Option<String>,
    pub school_id: Option<Uuid>,
    pub school_name: Option<String>,
    pub post_count: i64,
    pub created_at: DateTime<Utc>,
    pub status: String,
    pub revision: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<serde_json::Value>>,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Query {
    #[serde(rename = "page[number]")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]")]
    pub size: Option<i64>,
    pub search: Option<String>,
    pub mobile: Option<String>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub sort: Option<String>,
    pub school_id: Option<Uuid>,
    pub unbound: Option<bool>,
}
impl Query {
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let page = self.page.unwrap_or(1);
        let size = self.size.unwrap_or(30);
        require(
            (1..=100_000).contains(&page) && (1..=100).contains(&size),
            "分页参数无效",
        )?;
        require(
            self.status
                .as_deref()
                .is_none_or(|v| matches!(v, "normal" | "banned")),
            "用户状态无效",
        )?;
        require(
            !(self.unbound == Some(true) && self.school_id.is_some()),
            "学校筛选冲突",
        )?;
        require(
            self.created_from
                .zip(self.created_to)
                .is_none_or(|(a, b)| a <= b),
            "时间范围无效",
        )?;
        require(
            self.search
                .as_ref()
                .is_none_or(|v| v.chars().count() <= 100)
                && self.mobile.as_ref().is_none_or(|v| v.len() <= 11),
            "搜索条件过长",
        )?;
        Ok((size, (page - 1) * size))
    }
    pub fn order(&self) -> Result<&'static str> {
        match self.sort.as_deref().unwrap_or("-created_at") {
            "created_at" => Ok("u.created_at ASC,u.account_id ASC"),
            "-created_at" => Ok("u.created_at DESC,u.account_id DESC"),
            "post_count" => Ok("post_count ASC,u.account_id ASC"),
            "-post_count" => Ok("post_count DESC,u.account_id DESC"),
            _ => Err(Error::Invalid("排序字段无效".into())),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileInput {
    pub nickname: String,
    pub avatar_asset_id: Option<Uuid>,
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobileInput {
    pub mobile: String,
    pub revision: i64,
    #[serde(default)]
    pub reason: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusInput {
    pub status: String,
    pub reason: String,
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchoolInput {
    pub school_id: Uuid,
    pub reason: String,
    pub revision: i64,
}
pub fn mobile(value: &str) -> Result<String> {
    let value = value.trim();
    require(
        value.len() == 11 && value.starts_with('1') && value.bytes().all(|v| v.is_ascii_digit()),
        "手机号格式不正确",
    )?;
    Ok(value.into())
}
pub fn reason(value: &str) -> Result<String> {
    text(value, 1, 500)
}
pub fn masked(value: Option<&str>) -> Option<String> {
    value.map(|v| format!("{}****{}", &v[..3], &v[7..]))
}

pub fn wechat_identity(bytes: &[u8]) -> Result<String> {
    let v: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| Error::Unavailable)?;
    if let Some(code) = v.get("errcode")
        && code.as_i64() != Some(0)
    {
        return Err(Error::Unavailable);
    }
    let openid = v["openid"]
        .as_str()
        .filter(|v| {
            !v.is_empty()
                && v.len() <= 256
                && !v.chars().any(|c| c.is_control() || c.is_whitespace())
        })
        .ok_or(Error::Unavailable)?;
    Ok(openid.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_identity_and_profile_fields_are_rejected() {
        for raw in [
            r#"{"errcode":40029,"openid":"fake"}"#,
            r#"{"openid":""}"#,
            r#"{"openid":"with space"}"#,
            r#"{"errcode":"0","openid":"fake"}"#,
        ] {
            assert!(wechat_identity(raw.as_bytes()).is_err())
        }
        assert_eq!(
            wechat_identity(br#"{"openid":"valid-openid","session_key":"private"}"#).unwrap(),
            "valid-openid"
        );
        assert!(
            serde_json::from_value::<ProfileInput>(
                serde_json::json!({"nickname":"name","revision":1,"status":"normal"})
            )
            .is_err()
        );
        assert!(mobile("13800138000").is_ok());
        assert!(mobile("1380013800x").is_err());
        assert_eq!(masked(Some("13800138000")).unwrap(), "138****8000");
    }
}
