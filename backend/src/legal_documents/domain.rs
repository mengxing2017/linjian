use crate::business::{Error, Result, require};
use serde::Deserialize;

pub fn kind(value: &str) -> Result<&str> {
    match value {
        "user_agreement" | "privacy_policy" => Ok(value),
        _ => Err(Error::NotFound),
    }
}
pub fn content(value: &str) -> Result<String> {
    let meaningful = value.chars().any(|character| {
        !character.is_whitespace()
            && !matches!(character, '\u{200b}'..='\u{200f}' | '\u{2060}' | '\u{feff}')
    });
    require(
        meaningful && value.chars().count() <= 50_000,
        "正文不能为空或超过 50000 字符",
    )?;
    Ok(value.to_owned())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    pub revision: i64,
    pub content: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clear {
    pub revision: i64,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_plain_text_without_destroying_layout() {
        for value in ["", " \n\u{3000}", "\u{200b}\u{feff}"] {
            assert!(content(value).is_err());
        }
        assert!(content(&"😀".repeat(50_000)).is_ok());
        assert!(content(&"😀".repeat(50_001)).is_err());
        assert_eq!(
            content("正文\n<script>text</script>").unwrap(),
            "正文\n<script>text</script>"
        );
        assert!(kind("legal").is_err());
    }
}
