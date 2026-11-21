use crate::business::{Result, require, text};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const NICKNAME_ATTEMPTS: usize = 32;
pub const MIN_NUMBER: u32 = 100_000_000;
pub const NUMBER_COUNT: u32 = 900_000_000;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationSettings {
    pub nickname_prefix: String,
    pub revision: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationSettingsInput {
    pub nickname_prefix: String,
    pub revision: i64,
}

impl RegistrationSettingsInput {
    pub fn validate(&self) -> Result<String> {
        require(self.revision >= 0, "配置修订号无效")?;
        prefix(&self.nickname_prefix)
    }
}

pub fn prefix(value: &str) -> Result<String> {
    let value = text(value, 1, 91)?;
    require(
        !value.chars().any(char::is_control),
        "昵称前缀不能包含控制字符",
    )?;
    Ok(value)
}

pub fn candidate(prefix: &str, number: u32) -> Result<String> {
    require(
        (MIN_NUMBER..MIN_NUMBER + NUMBER_COUNT).contains(&number),
        "随机昵称数字无效",
    )?;
    Ok(format!("{prefix}{number}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_and_number_boundaries_preserve_unicode() {
        assert_eq!(prefix(" 用户 ").unwrap(), "用户");
        assert!(prefix("").is_err());
        assert!(prefix("用户\n名称").is_err());
        assert!(prefix(&"字".repeat(91)).is_ok());
        assert!(prefix(&"字".repeat(92)).is_err());
        assert!(prefix("👩‍💻").is_ok());
        assert_eq!(candidate("用户", MIN_NUMBER).unwrap(), "用户100000000");
        assert!(candidate("用户", 99_999_999).is_err());
        assert!(candidate("用户", 999_999_999).is_ok());
        assert!(candidate("用户", 1_000_000_000).is_err());
    }
}
