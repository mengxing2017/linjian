use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug)]
pub enum Error {
    Unauthorized,
    MiniappBanned,
    SchoolUnavailable,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    Policy(&'static str),
    Invalid(String),
    Unavailable,
    Upstream(i64),
    Database(String),
}
pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}
pub fn text(value: &str, min: usize, max: usize) -> Result<String> {
    let s = value.trim();
    require((min..=max).contains(&s.chars().count()), "文本长度不合法")?;
    Ok(s.into())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grant {
    pub school_id: Uuid,
    pub role: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionKind {
    #[default]
    AdminBrowser,
    Miniapp,
}
impl SessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdminBrowser => "admin_browser",
            Self::Miniapp => "miniapp",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub platform_admin: bool,
    pub bound_school_id: Option<Uuid>,
    pub grants: Vec<Grant>,
    #[serde(skip)]
    pub permissions: std::collections::BTreeMap<Uuid, Vec<String>>,
    #[serde(skip)]
    pub csrf: String,
    #[serde(skip)]
    pub session_hash: String,
    #[serde(skip)]
    pub request_id: String,
    #[serde(skip)]
    pub session_kind: SessionKind,
    #[serde(skip)]
    pub login_origin: Option<String>,
}
impl Actor {
    pub fn has_permission(&self, school: Uuid, permission: &str) -> bool {
        crate::schools::domain::permissions::catalog()
            .iter()
            .any(|definition| definition.key == permission)
            && self.is_admin_browser()
            && (self.platform_admin
                || self
                    .permissions
                    .get(&school)
                    .is_some_and(|permissions| permissions.iter().any(|key| key == permission)))
    }
    pub fn permission(&self, school: Uuid, permission: &str) -> Result<()> {
        if self.has_permission(school, permission) {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
    pub fn is_admin_browser(&self) -> bool {
        self.session_kind == SessionKind::AdminBrowser
    }

    pub fn can(&self, school: Uuid, role: &str) -> bool {
        (self.is_admin_browser() || role == "member")
            && (self.platform_admin
                || self.grants.iter().any(|g| {
                    g.school_id == school
                        && (role == "member"
                            || g.role == "admin"
                            || (role == "operator" && g.role == "operator"))
                }))
    }
    pub fn authorize(&self, school: Uuid, role: &str) -> Result<()> {
        if self.can(school, role) {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
    pub fn platform(&self) -> Result<()> {
        if self.is_admin_browser() && self.platform_admin {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}
#[derive(Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[serde(rename = "page[number]", default, deserialize_with = "query_option")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]", default, deserialize_with = "query_option")]
    pub page_size: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub visibility: Option<String>,
    #[serde(default, deserialize_with = "query_option")]
    pub mine: Option<bool>,
    pub kind: Option<String>,
    pub tag: Option<String>,
}

fn query_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + std::str::FromStr,
    T::Err: std::fmt::Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum QueryValue<T> {
        Value(T),
        Text(String),
    }

    Option::<QueryValue<T>>::deserialize(deserializer)?
        .map(|value| match value {
            QueryValue::Value(value) => Ok(value),
            QueryValue::Text(value) => value.parse().map_err(serde::de::Error::custom),
        })
        .transpose()
}

impl Page {
    /// Fetch one extra row to determine the next-page state without guessing totals.
    pub fn fetch_bounds(&self) -> Result<(i64, i64)> {
        self.bounds().map(|(size, offset)| (size + 1, offset))
    }
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let size = self.page_size.unwrap_or(30);
        let page = self.page.unwrap_or(1);
        require(
            (1..=100).contains(&size) && (1..=100_000).contains(&page),
            "分页参数不合法",
        )?;
        Ok((size, (page - 1) * size))
    }
}
#[derive(Deserialize)]
pub struct Revision {
    pub revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub revision: i64,
    pub decision: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub review_digest: Option<String>,
}
#[derive(Deserialize)]
pub struct Named {
    pub name: String,
}
pub fn uuid(value: &Value, key: &str) -> Result<Uuid> {
    value[key]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::Invalid(format!("缺少有效的 {key}")))
}
pub fn hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
pub fn secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

#[cfg(test)]
mod query_tests {
    use super::*;
    use axum::extract::Query;

    #[test]
    fn page_query_parses_pagination_and_boolean() {
        for mine in [true, false] {
            let uri = format!("/?page[number]=2&page[size]=10&mine={mine}")
                .parse()
                .unwrap();
            let Query(page) = Query::<Page>::try_from_uri(&uri).unwrap();
            assert_eq!(page.page, Some(2));
            assert_eq!(page.page_size, Some(10));
            assert_eq!(page.mine, Some(mine));
            assert_eq!(page.bounds().unwrap(), (10, 10));
        }
    }

    #[test]
    fn page_query_preserves_native_values_and_defaults() {
        let page: Page = serde_json::from_value(serde_json::json!({
            "page[number]": 2,
            "page[size]": 10,
            "mine": false
        }))
        .unwrap();
        assert_eq!(page.bounds().unwrap(), (10, 10));
        assert_eq!(page.mine, Some(false));
        let page: Page = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(page.bounds().unwrap(), (30, 0));
        assert_eq!(page.mine, None);
    }

    #[test]
    fn page_query_rejects_invalid_values() {
        for query in [
            "page[number]=abc",
            "page[size]=1.5",
            "page[number]=9223372036854775808",
            "mine=1",
            "mine=invalid",
            "page[number]=",
            "unexpected=value",
            "page[number]=1&page[number]=2",
        ] {
            let uri = format!("/?{query}").parse().unwrap();
            assert!(Query::<Page>::try_from_uri(&uri).is_err(), "{query}");
        }
    }

    #[test]
    fn page_query_preserves_bounds_validation() {
        for query in [
            "page[number]=0",
            "page[number]=100001",
            "page[size]=0",
            "page[size]=101",
            "page[size]=-1",
        ] {
            let uri = format!("/?{query}").parse().unwrap();
            let Query(page) = Query::<Page>::try_from_uri(&uri).unwrap();
            assert!(page.bounds().is_err(), "{query}");
        }
    }
}
