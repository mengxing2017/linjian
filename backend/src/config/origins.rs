use std::collections::BTreeSet;

/// A configured HTTP origin, never a URL path or a request-provided host.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalOrigin(String);

impl CanonicalOrigin {
    pub fn parse(value: &str, local: bool) -> Result<Self, String> {
        let url = reqwest::Url::parse(value).map_err(|_| "invalid origin".to_owned())?;
        let authority = value.split_once("://").ok_or("invalid origin")?.1;
        if value.chars().any(|c| c.is_whitespace() || c.is_control())
            || authority.contains(['/', '\\', '?', '#', '@', '*', '%'])
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("expected an origin without credentials, path, query or fragment".into());
        }
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if url.scheme() != "https" && !(local && loopback && url.scheme() == "http") {
            return Err("HTTPS is required (HTTP loopback is allowed only in local mode)".into());
        }
        Ok(Self(url.origin().ascii_serialization()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn matches(&self, request: &Self, local: bool) -> bool {
        if self == request {
            return true;
        }
        let a = reqwest::Url::parse(self.as_str()).expect("validated origin");
        let b = reqwest::Url::parse(request.as_str()).expect("validated origin");
        let loopback = |url: &reqwest::Url| {
            matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        };
        local
            && loopback(&a)
            && loopback(&b)
            && a.scheme() == b.scheme()
            && a.port_or_known_default() == b.port_or_known_default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct PublicEndpoints {
    pub admin: Option<CanonicalOrigin>,
    pub official_account: Option<CanonicalOrigin>,
    pub community: Option<CanonicalOrigin>,
    pub api: Option<CanonicalOrigin>,
    browser_origins: BTreeSet<CanonicalOrigin>,
}

impl PublicEndpoints {
    pub fn from_env(local: bool) -> Result<Self, String> {
        Self::parse(|key| std::env::var(key).ok(), local)
    }

    pub fn parse(get: impl Fn(&str) -> Option<String>, local: bool) -> Result<Self, String> {
        let origin = |key| {
            get(key)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    CanonicalOrigin::parse(&value, local).map_err(|error| format!("{key}: {error}"))
                })
                .transpose()
        };
        let values: Vec<String> =
            serde_json::from_str(&get("LINJIAN_BROWSER_ORIGINS").unwrap_or_else(|| "[]".into()))
                .map_err(|_| "LINJIAN_BROWSER_ORIGINS: expected a JSON array of origins")?;
        let browser_origins = values
            .iter()
            .map(|value| {
                CanonicalOrigin::parse(value, local)
                    .map_err(|error| format!("LINJIAN_BROWSER_ORIGINS: {error}"))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let result = Self {
            admin: origin("LINJIAN_ADMIN_WEB_ORIGIN")?,
            official_account: origin("LINJIAN_OFFICIAL_ACCOUNT_WEB_ORIGIN")?,
            community: origin("LINJIAN_COMMUNITY_WEB_ORIGIN")?,
            api: origin("LINJIAN_API_PUBLIC_ORIGIN")?,
            browser_origins,
        };
        let declared: BTreeSet<_> = [&result.admin, &result.official_account, &result.community]
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        if declared != result.browser_origins {
            return Err(
                "LINJIAN_BROWSER_ORIGINS must match the configured browser entrances".into(),
            );
        }
        Ok(result)
    }

    pub fn allows_admin(&self, value: &str, local: bool) -> bool {
        CanonicalOrigin::parse(value, local).is_ok_and(|request| {
            self.admin
                .as_ref()
                .is_some_and(|admin| admin.matches(&request, local))
        })
    }

    pub fn local_admin() -> Self {
        let admin = CanonicalOrigin::parse("http://localhost:5173", true).expect("local origin");
        Self {
            admin: Some(admin.clone()),
            browser_origins: [admin].into(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_are_canonical_and_strict() {
        assert_eq!(
            CanonicalOrigin::parse("https://EXAMPLE.com:443", false)
                .unwrap()
                .as_str(),
            "https://example.com"
        );
        for value in [
            "null",
            "https://*.example.com",
            "https://example.com/",
            "https://example.com/x",
            "https://example.com?",
            "https://example.com#",
            "https://u@example.com",
            "https://example.com:99999",
            "https://example.com\\x",
            " https://example.com",
            "http://example.com",
            "http://localhost:5173",
        ] {
            assert!(CanonicalOrigin::parse(value, false).is_err(), "{value}");
        }
    }

    #[test]
    fn trusted_h5_is_not_an_admin_login_source() {
        let parse = |entries: &[(&str, &str)]| {
            PublicEndpoints::parse(
                |key| {
                    entries
                        .iter()
                        .find(|(k, _)| *k == key)
                        .map(|(_, v)| v.to_string())
                },
                false,
            )
        };
        let endpoints = parse(&[("LINJIAN_ADMIN_WEB_ORIGIN", "https://admin.example.com"), ("LINJIAN_OFFICIAL_ACCOUNT_WEB_ORIGIN", "https://m.example.com"), ("LINJIAN_BROWSER_ORIGINS", r#"["https://m.example.com","https://admin.example.com","https://admin.example.com"]"#)]).unwrap();
        assert!(endpoints.allows_admin("https://admin.example.com:443", false));
        assert!(!endpoints.allows_admin("https://m.example.com", false));
        assert!(!endpoints.allows_admin("https://admin.example.com.evil.test", false));
        assert!(
            parse(&[(
                "LINJIAN_BROWSER_ORIGINS",
                r#"["https://unknown.example.com"]"#
            )])
            .is_err()
        );
        assert!(parse(&[("LINJIAN_ADMIN_WEB_ORIGIN", "https://admin.example.com")]).is_err());
        assert!(parse(&[("LINJIAN_BROWSER_ORIGINS", "null")]).is_err());
        assert!(!PublicEndpoints::default().allows_admin("https://admin.example.com", false));
    }

    #[test]
    fn local_aliases_require_matching_scheme_port_and_local_mode() {
        let endpoints = PublicEndpoints::local_admin();
        assert!(endpoints.allows_admin("http://127.0.0.1:5173", true));
        assert!(endpoints.allows_admin("http://[::1]:5173", true));
        for value in [
            "http://localhost:5174",
            "https://localhost:5173",
            "http://localhost.evil:5173",
        ] {
            assert!(!endpoints.allows_admin(value, true));
        }
        assert!(!endpoints.allows_admin("http://127.0.0.1:5173", false));
    }
}
