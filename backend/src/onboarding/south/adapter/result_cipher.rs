use crate::business::{Error, Result};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use sha2::{Digest, Sha256};
fn key(ticket: &str) -> Result<aead::LessSafeKey> {
    let bytes = Sha256::digest(format!("linjian:onboarding:response:{ticket}"));
    Ok(aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_256_GCM, &bytes).map_err(|_| Error::Unavailable)?,
    ))
}
pub fn encrypt(ticket: &str, value: &serde_json::Value) -> Result<Vec<u8>> {
    let mut nonce = [0; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| Error::Unavailable)?;
    let mut body = serde_json::to_vec(value).map_err(|_| Error::Unavailable)?;
    key(ticket)?
        .seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(b"linjian:onboarding:v1"),
            &mut body,
        )
        .map_err(|_| Error::Unavailable)?;
    Ok([nonce.as_slice(), body.as_slice()].concat())
}
pub fn decrypt(ticket: &str, value: &[u8]) -> Result<serde_json::Value> {
    if value.len() < 28 {
        return Err(Error::Unavailable);
    }
    let nonce =
        aead::Nonce::try_assume_unique_for_key(&value[..12]).map_err(|_| Error::Unavailable)?;
    let mut body = value[12..].to_vec();
    let plaintext = key(ticket)?
        .open_in_place(nonce, aead::Aad::from(b"linjian:onboarding:v1"), &mut body)
        .map_err(|_| Error::Unauthorized)?;
    serde_json::from_slice(plaintext).map_err(|_| Error::Unavailable)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_is_authenticated_and_bound_to_ticket() {
        let expected = serde_json::json!({"token":"private"});
        let bytes = encrypt("ticket", &expected).unwrap();
        assert_eq!(decrypt("ticket", &bytes).unwrap(), expected);
        assert!(decrypt("other", &bytes).is_err());
    }
}
