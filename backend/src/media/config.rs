use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub storage: Storage,
    pub upload: Upload,
    pub lifecycle: Lifecycle,
}
#[derive(Clone, Debug)]
pub struct Storage {
    pub filesystem_root: PathBuf,
}
#[derive(Clone, Debug)]
pub struct Upload {
    pub max_image_bytes: usize,
    pub max_width: u32,
    pub max_height: u32,
    pub max_pixels: u64,
    pub decode_concurrency: usize,
}
#[derive(Clone, Debug)]
pub struct Lifecycle {
    pub unbound_ttl_hours: i64,
    pub artwork_unbound_ttl_hours: i64,
    pub draft_lease_days: i64,
    pub cleanup_batch_size: i64,
    pub orphan_grace_hours: i64,
}
impl Config {
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_with_root(PathBuf::from("/data/media"))
    }
    pub fn from_env_with_root(root: PathBuf) -> Result<Self, String> {
        Self::parse(root, |key| match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(_) => Err(format!("{key} must be valid Unicode")),
        })
    }
    fn parse(
        root: PathBuf,
        get: impl Fn(&str) -> Result<Option<String>, String>,
    ) -> Result<Self, String> {
        let root = get("LINJIAN_MEDIA_ROOT")?
            .map(PathBuf::from)
            .unwrap_or(root);
        if !root.is_absolute() {
            return Err("LINJIAN_MEDIA_ROOT must be an absolute directory".into());
        }
        let mut value = Self::local(root);
        fn number<T: std::str::FromStr>(
            get: &impl Fn(&str) -> Result<Option<String>, String>,
            key: &str,
            default: T,
        ) -> Result<T, String> {
            get(key)?
                .map(|v| v.parse().map_err(|_| format!("{key} must be an integer")))
                .unwrap_or(Ok(default))
        }
        macro_rules! setting {
            ($section:ident, $field:ident, $key:literal) => {
                value.$section.$field = number(&get, $key, value.$section.$field)?;
            };
        }
        setting!(upload, max_image_bytes, "LINJIAN_MEDIA_MAX_IMAGE_BYTES");
        setting!(upload, max_width, "LINJIAN_MEDIA_MAX_WIDTH");
        setting!(upload, max_height, "LINJIAN_MEDIA_MAX_HEIGHT");
        setting!(upload, max_pixels, "LINJIAN_MEDIA_MAX_PIXELS");
        setting!(
            upload,
            decode_concurrency,
            "LINJIAN_MEDIA_DECODE_CONCURRENCY"
        );
        setting!(
            lifecycle,
            unbound_ttl_hours,
            "LINJIAN_MEDIA_UNBOUND_TTL_HOURS"
        );
        setting!(
            lifecycle,
            artwork_unbound_ttl_hours,
            "LINJIAN_MEDIA_ARTWORK_UNBOUND_TTL_HOURS"
        );
        setting!(
            lifecycle,
            draft_lease_days,
            "LINJIAN_MEDIA_DRAFT_LEASE_DAYS"
        );
        setting!(
            lifecycle,
            cleanup_batch_size,
            "LINJIAN_MEDIA_CLEANUP_BATCH_SIZE"
        );
        value.lifecycle.orphan_grace_hours =
            Self::orphan_grace_hours_from_env_value(get("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS")?)?;
        value.validate()?;
        Ok(value)
    }
    pub fn orphan_grace_hours_from_env() -> Result<i64, String> {
        let value = match std::env::var("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => return Err("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS 必须为有效文本".into()),
        };
        Self::orphan_grace_hours_from_env_value(value)
    }
    fn orphan_grace_hours_from_env_value(value: Option<String>) -> Result<i64, String> {
        let hours = value
            .map(|v| v.parse::<i64>())
            .transpose()
            .map_err(|_| "LINJIAN_MEDIA_ORPHAN_GRACE_HOURS must be an integer")?
            .unwrap_or(24);
        if !(1..=168).contains(&hours) {
            return Err("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS must be between 1 and 168".into());
        }
        Ok(hours)
    }
    pub fn local(root: PathBuf) -> Self {
        Self {
            storage: Storage {
                filesystem_root: root,
            },
            upload: Upload {
                max_image_bytes: 5 * 1024 * 1024,
                max_width: 8192,
                max_height: 8192,
                max_pixels: 24_000_000,
                decode_concurrency: 2,
            },
            lifecycle: Lifecycle {
                unbound_ttl_hours: 24,
                artwork_unbound_ttl_hours: 168,
                draft_lease_days: 30,
                cleanup_batch_size: 100,
                orphan_grace_hours: 24,
            },
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let u = &self.upload;
        let l = &self.lifecycle;
        if !self.storage.filesystem_root.is_absolute()
            || !(1..=5 * 1024 * 1024).contains(&u.max_image_bytes)
            || !(1..=8192).contains(&u.max_width)
            || !(1..=8192).contains(&u.max_height)
            || !(1..=24_000_000).contains(&u.max_pixels)
            || !(1..=16).contains(&u.decode_concurrency)
            || !(1..=168).contains(&l.unbound_ttl_hours)
            || !(1..=720).contains(&l.artwork_unbound_ttl_hours)
            || !(1..=90).contains(&l.draft_lease_days)
            || !(1..=168).contains(&l.orphan_grace_hours)
            || !(1..=1000).contains(&l.cleanup_batch_size)
        {
            return Err("invalid media storage settings".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(root: &str, values: &[(&str, &str)]) -> Result<Config, String> {
        Config::parse(root.into(), |key| {
            Ok(values
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).into()))
        })
    }
    #[test]
    fn environment_overrides_defaults_in_both_runtime_modes() {
        assert_eq!(
            parse("/data/media", &[]).unwrap().storage.filesystem_root,
            PathBuf::from("/data/media")
        );
        assert_eq!(
            parse("/tmp/linjian-media", &[])
                .unwrap()
                .storage
                .filesystem_root,
            PathBuf::from("/tmp/linjian-media")
        );
        for root in ["/data/media", "/tmp/linjian-media"] {
            let config = parse(
                root,
                &[
                    ("LINJIAN_MEDIA_ROOT", "/tmp/custom"),
                    ("LINJIAN_MEDIA_MAX_IMAGE_BYTES", "1024"),
                    ("LINJIAN_MEDIA_DRAFT_LEASE_DAYS", "7"),
                    ("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS", "48"),
                ],
            )
            .unwrap();
            assert_eq!(config.storage.filesystem_root, PathBuf::from("/tmp/custom"));
            assert_eq!(config.upload.max_image_bytes, 1024);
            assert_eq!(config.lifecycle.draft_lease_days, 7);
            assert_eq!(config.lifecycle.orphan_grace_hours, 48);
        }
    }
    #[test]
    fn invalid_environment_is_rejected_without_fallback() {
        for (key, bad) in [
            ("LINJIAN_MEDIA_ROOT", "relative"),
            ("LINJIAN_MEDIA_ROOT", ""),
            ("LINJIAN_MEDIA_MAX_IMAGE_BYTES", "5242881"),
            ("LINJIAN_MEDIA_MAX_WIDTH", "0"),
            ("LINJIAN_MEDIA_MAX_HEIGHT", "8193"),
            ("LINJIAN_MEDIA_MAX_PIXELS", "24000001"),
            ("LINJIAN_MEDIA_DECODE_CONCURRENCY", "NaN"),
            ("LINJIAN_MEDIA_DECODE_CONCURRENCY", "17"),
            ("LINJIAN_MEDIA_UNBOUND_TTL_HOURS", "-1"),
            ("LINJIAN_MEDIA_ARTWORK_UNBOUND_TTL_HOURS", "721"),
            ("LINJIAN_MEDIA_DRAFT_LEASE_DAYS", "91"),
            ("LINJIAN_MEDIA_CLEANUP_BATCH_SIZE", "1001"),
            ("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS", "0"),
            ("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS", "169"),
            ("LINJIAN_MEDIA_ORPHAN_GRACE_HOURS", "no"),
        ] {
            assert!(parse("/data/media", &[(key, bad)]).is_err(), "{key}");
        }
    }
}
