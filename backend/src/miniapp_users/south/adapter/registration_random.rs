use crate::{
    business::{Error, Result},
    miniapp_users::{
        domain::registration::{MIN_NUMBER, NUMBER_COUNT},
        south::port::NicknameRandom,
    },
};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RegistrationRandom {
    seed: [u8; 32],
    counter: u64,
}

impl RegistrationRandom {
    pub fn new() -> Result<Self> {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Unavailable)?
            .as_nanos();
        let mut entropy = [0_u8; 32];
        SystemRandom::new()
            .fill(&mut entropy)
            .map_err(|_| Error::Unavailable)?;
        Ok(Self::seeded(time, entropy))
    }

    fn seeded(time: u128, entropy: [u8; 32]) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"linjian:registration-nickname:v1");
        digest.update(time.to_be_bytes());
        digest.update(entropy);
        Self {
            seed: digest.finalize().into(),
            counter: 0,
        }
    }
}

impl NicknameRandom for RegistrationRandom {
    fn next_number(&mut self) -> Result<u32> {
        let cutoff = (1_u64 << 32) / u64::from(NUMBER_COUNT) * u64::from(NUMBER_COUNT);
        loop {
            let mut digest = Sha256::new();
            digest.update(self.seed);
            digest.update(self.counter.to_be_bytes());
            self.counter = self.counter.checked_add(1).ok_or(Error::Unavailable)?;
            let bytes = digest.finalize();
            let sample = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if u64::from(sample) < cutoff {
                return Ok(MIN_NUMBER + sample % NUMBER_COUNT);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_sequences_are_reproducible_bounded_and_advance() {
        let mut first = RegistrationRandom::seeded(123, [7; 32]);
        let mut same = RegistrationRandom::seeded(123, [7; 32]);
        let mut earlier = RegistrationRandom::seeded(122, [7; 32]);
        assert_ne!(earlier.next_number().unwrap(), first.next_number().unwrap());
        same.next_number().unwrap();
        let mut values = std::collections::HashSet::new();
        for _ in 0..1000 {
            let value = first.next_number().unwrap();
            assert_eq!(value, same.next_number().unwrap());
            assert!((MIN_NUMBER..MIN_NUMBER + NUMBER_COUNT).contains(&value));
            values.insert(value);
        }
        assert!(values.len() > 1);
    }
}
