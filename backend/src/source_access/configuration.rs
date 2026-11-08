//! Server-only deployment configuration. Never serialize or log credential material.
use super::domain::{Fault, Result, ensure, identifier, origin, secret};
pub struct ConnectionConfig {
    pub origin: String,
    pub key_id: String,
    secret: String,
}
impl ConnectionConfig {
    pub fn from_env() -> Result<Option<Self>> {
        Self::read(|name| std::env::var(name).unwrap_or_default())
    }
    fn read(get: impl Fn(&str) -> String) -> Result<Option<Self>> {
        let base = get("LINJIAN_WEKEEPER_ORIGIN");
        let key = get("LINJIAN_WEKEEPER_KEY_ID");
        let material = get("LINJIAN_WEKEEPER_SECRET");
        if base.is_empty() && key.is_empty() && material.is_empty() {
            return Ok(None);
        }
        ensure(
            !base.is_empty() && !key.is_empty() && !material.is_empty(),
            "wekeeper_configuration_incomplete",
        )?;
        let parsed = origin(&base).map_err(|_| Fault("wekeeper_origin_invalid"))?;
        ensure(
            parsed.origin().ascii_serialization() == base,
            "wekeeper_origin_invalid",
        )?;
        ensure(identifier(&key), "wekeeper_key_id_invalid")?;
        secret(&material).map_err(|_| Fault("wekeeper_secret_invalid"))?;
        Ok(Some(Self {
            origin: base,
            key_id: key,
            secret: material,
        }))
    }
    pub fn material(&self, base: &str, key: &str) -> Result<String> {
        ensure(
            self.origin == base && self.key_id == key,
            "credential_unavailable",
        )?;
        Ok(self.secret.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config(base: &str, key: &str, material: &str) -> Result<Option<ConnectionConfig>> {
        ConnectionConfig::read(|name| {
            match name {
                "LINJIAN_WEKEEPER_ORIGIN" => base,
                "LINJIAN_WEKEEPER_KEY_ID" => key,
                "LINJIAN_WEKEEPER_SECRET" => material,
                _ => unreachable!(),
            }
            .into()
        })
    }
    #[test]
    fn empty_disables_and_partial_configuration_is_rejected() {
        assert!(config("", "", "").unwrap().is_none());
        for (base, key, material) in [
            ("https://keeper.example.com", "", ""),
            ("", "key", ""),
            ("", "", "value"),
        ] {
            assert_eq!(
                config(base, key, material).err().unwrap().0,
                "wekeeper_configuration_incomplete"
            );
        }
    }
    #[test]
    fn validates_origin_key_and_secret_without_exposing_values() {
        let material = crate::source_access::domain::encode(&[1; 32]);
        let c = config("https://keeper.example.com", "key-1", &material)
            .unwrap()
            .unwrap();
        assert_eq!(
            c.material("https://keeper.example.com", "key-1").unwrap(),
            material
        );
        assert!(c.material("https://other.example.com", "key-1").is_err());
        assert!(c.material("https://keeper.example.com", "key-2").is_err());
        for base in [
            "http://keeper.example.com",
            "https://keeper.example.com/path",
            "https://keeper.example.com/",
            "https://keeper.example.com?x=1",
        ] {
            assert_eq!(
                config(base, "key-1", &material).err().unwrap().0,
                "wekeeper_origin_invalid"
            );
        }
        assert_eq!(
            config("https://keeper.example.com", "bad key", &material)
                .err()
                .unwrap()
                .0,
            "wekeeper_key_id_invalid"
        );
        assert_eq!(
            config("https://keeper.example.com", "key-1", "bad")
                .err()
                .unwrap()
                .0,
            "wekeeper_secret_invalid"
        );
    }
}
