use crate::business::{Error, Result};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PhoneCipher {
    key: Arc<aead::LessSafeKey>,
    pub version: String,
}

impl PhoneCipher {
    pub fn from_env() -> Result<Self> {
        let key = std::env::var("LINJIAN_PHONE_ENCRYPTION_KEY").unwrap_or_default();
        let version =
            std::env::var("LINJIAN_PHONE_ENCRYPTION_KEY_ID").unwrap_or_else(|_| "v1".into());
        Self::new(&key, version)
    }

    pub fn new(hex: &str, version: String) -> Result<Self> {
        let unavailable = || Error::Phone("phone_provider_unavailable");
        if hex.len() != 64
            || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            || version.is_empty()
            || version.len() > 64
            || version.chars().any(char::is_control)
        {
            return Err(unavailable());
        }
        let key: Vec<u8> = (0..64)
            .step_by(2)
            .map(|offset| {
                u8::from_str_radix(&hex[offset..offset + 2], 16).map_err(|_| unavailable())
            })
            .collect::<Result<_>>()?;
        let key = aead::UnboundKey::new(&aead::AES_256_GCM, &key).map_err(|_| unavailable())?;
        Ok(Self {
            key: Arc::new(aead::LessSafeKey::new(key)),
            version,
        })
    }

    pub fn encrypt(&self, account: Uuid, number: &str) -> Result<Vec<u8>> {
        let mut nonce = [0; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| Error::Unavailable)?;
        let mut ciphertext = number.as_bytes().to_vec();
        let associated = format!("linjian:phone:{account}:{}", self.version);
        self.key
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(associated.as_bytes()),
                &mut ciphertext,
            )
            .map_err(|_| Error::Unavailable)?;
        Ok([nonce.as_slice(), ciphertext.as_slice()].concat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn randomized_authenticated_encryption_is_bound_to_account_and_version() {
        let cipher = PhoneCipher::new(&"12".repeat(32), "test-v1".into()).unwrap();
        let id = Uuid::new_v4();
        let encrypted = cipher.encrypt(id, "13800138000").unwrap();
        assert_ne!(encrypted, cipher.encrypt(id, "13800138000").unwrap());
        assert!(!encrypted.windows(11).any(|window| window == b"13800138000"));
        let mut body = encrypted[12..].to_vec();
        let nonce: [u8; 12] = encrypted[..12].try_into().unwrap();
        let aad = format!("linjian:phone:{id}:test-v1");
        assert_eq!(
            cipher
                .key
                .open_in_place(
                    aead::Nonce::assume_unique_for_key(nonce),
                    aead::Aad::from(aad.as_bytes()),
                    &mut body
                )
                .unwrap(),
            b"13800138000"
        );
        let mut body = encrypted[12..].to_vec();
        assert!(
            cipher
                .key
                .open_in_place(
                    aead::Nonce::assume_unique_for_key(nonce),
                    aead::Aad::from(b"other-account"),
                    &mut body
                )
                .is_err()
        );
        for key in ["", "z", &"x".repeat(64)] {
            assert!(PhoneCipher::new(key, "v1".into()).is_err());
        }
    }
}
