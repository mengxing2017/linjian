use crate::business::{Error, Page, Result, hash, require, text};
use serde::Deserialize;
use uuid::Uuid;

pub const NORMALIZATION_VERSION: &str = "plain-text-v1";
pub const COMPOSITION_VERSION: &str = "summary-20-ad-tail-v1";

pub fn normalize(value: &str) -> Result<String> {
    let value = value.trim_matches(char::is_whitespace);
    require(
        (1..=50).contains(&value.chars().count()),
        "广告文案须为 1–50 个 Unicode 字符",
    )?;
    require(
        !value.chars().any(|character| {
            character.is_control() || matches!(character, '\u{2028}' | '\u{2029}' | '\u{feff}')
        }),
        "广告文案不能包含换行或控制字符",
    )?;
    Ok(value.into())
}

pub fn digest(
    school: Uuid,
    advertisement: Uuid,
    revision: Uuid,
    number: i64,
    content: &str,
) -> String {
    hash(
        &serde_json::to_string(&(
            school,
            advertisement,
            revision,
            number,
            NORMALIZATION_VERSION,
            content,
        ))
        .expect("serializable advertisement digest"),
    )
}

pub fn version(value: &str) -> Result<i64> {
    require(
        !value.is_empty() && value.bytes().all(|character| character.is_ascii_digit()),
        "版本必须为十进制字符串",
    )?;
    value
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| Error::Invalid("版本无效".into()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub content: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub content: String,
    pub expected_config_version: String,
    pub expected_revision_version: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionAction {
    pub expected_config_version: String,
    pub expected_revision_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigAction {
    pub expected_config_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub expected_config_version: String,
    pub expected_revision_version: String,
    pub review_digest: String,
    pub decision: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Remove {
    pub expected_config_version: String,
    pub reason: String,
}

pub enum Command {
    Create(Create),
    Draft(Draft),
    Submit(RevisionAction),
    Withdraw(RevisionAction),
    Delete(ConfigAction),
    Decide(Decision),
    Remove(Remove),
}

impl Command {
    pub fn platform(&self) -> bool {
        matches!(self, Self::Decide(_) | Self::Remove(_))
    }
    pub fn validate(&mut self) -> Result<()> {
        let config_version = match self {
            Self::Create(input) => {
                input.content = normalize(&input.content)?;
                return Ok(());
            }
            Self::Draft(input) => {
                input.content = normalize(&input.content)?;
                if let Some(revision) = &input.expected_revision_version {
                    version(revision)?;
                }
                &input.expected_config_version
            }
            Self::Submit(input) | Self::Withdraw(input) => {
                version(&input.expected_revision_version)?;
                &input.expected_config_version
            }
            Self::Delete(input) => &input.expected_config_version,
            Self::Decide(input) => {
                require(
                    matches!(input.decision.as_str(), "approve" | "reject"),
                    "审核决定无效",
                )?;
                require(
                    input.review_digest.len() == 64
                        && input
                            .review_digest
                            .bytes()
                            .all(|character| character.is_ascii_hexdigit()),
                    "审核指纹无效",
                )?;
                version(&input.expected_revision_version)?;
                input.reason = text(&input.reason, usize::from(input.decision == "reject"), 500)?;
                &input.expected_config_version
            }
            Self::Remove(input) => {
                input.reason = text(&input.reason, 1, 500)?;
                &input.expected_config_version
            }
        };
        version(config_version)?;
        Ok(())
    }
    pub fn expected_config_version(&self) -> Option<&str> {
        match self {
            Self::Create(_) => None,
            Self::Draft(input) => Some(&input.expected_config_version),
            Self::Submit(input) | Self::Withdraw(input) => Some(&input.expected_config_version),
            Self::Delete(input) => Some(&input.expected_config_version),
            Self::Decide(input) => Some(&input.expected_config_version),
            Self::Remove(input) => Some(&input.expected_config_version),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    #[serde(flatten)]
    pub page: Page,
    pub school_id: Option<Uuid>,
    pub created_from: Option<chrono::DateTime<chrono::Utc>>,
    pub created_to: Option<chrono::DateTime<chrono::Utc>>,
}

impl Filters {
    pub fn validate(&self) -> Result<()> {
        self.page.bounds()?;
        require(
            self.page.status.as_deref().is_none_or(|status| {
                matches!(
                    status,
                    "draft" | "pending" | "approved" | "rejected" | "withdrawn"
                )
            }),
            "广告审核状态无效",
        )?;
        require(
            self.created_from
                .zip(self.created_to)
                .is_none_or(|(from, to)| from <= to),
            "时间范围无效",
        )
    }
}

pub struct Composition {
    pub content: String,
    pub result: &'static str,
    pub base_length: usize,
    pub final_length: usize,
}

pub fn append(base: &str, advertisement: Option<&str>) -> Composition {
    let base_length = base.chars().count();
    let (content, result) = match advertisement {
        None => (base.to_owned(), "no_approved_advertisement"),
        Some(value) if base_length + 1 + value.chars().count() <= 200 => {
            (format!("{base}\n{value}"), "applied")
        }
        Some(_) => (base.to_owned(), "message_budget_exceeded"),
    };
    Composition {
        final_length: content.chars().count(),
        content,
        result,
        base_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_validation_and_scalar_budget() {
        assert_eq!(normalize("\u{3000}广告😀\u{0085}").unwrap(), "广告😀");
        for value in ["", "  ", "a\nb", "a\rb", "a\tb", "a\u{2028}b", "\u{feff}a"] {
            assert!(normalize(value).is_err(), "{value:?}");
        }
        assert!(normalize(&"😀".repeat(50)).is_ok());
        assert!(normalize(&"😀".repeat(51)).is_err());
        assert_eq!(normalize("e\u{301}").unwrap().chars().count(), 2);
    }

    #[test]
    fn append_preserves_base_and_never_truncates() {
        let base = "原文\n#opaque-link";
        assert_eq!(append(base, None).content, base);
        assert_eq!(
            append(base, Some("广告😀")).content,
            format!("{base}\n广告😀")
        );
        assert_eq!(append(&"字".repeat(198), Some("😀")).final_length, 200);
        assert_eq!(append(&"字".repeat(197), Some("😀")).final_length, 199);
        let base = "字".repeat(199);
        let result = append(&base, Some("😀"));
        assert_eq!(result.content, base);
        assert_eq!(result.result, "message_budget_exceeded");
    }

    #[test]
    fn shared_contract_rule_fixtures() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../api-contract/fixtures/message-advertisement-rules.json"
        ))
        .unwrap();
        for sample in fixtures["normalization"].as_array().unwrap() {
            let actual = normalize(sample["input"].as_str().unwrap()).ok();
            assert_eq!(actual.as_deref(), sample["normalized"].as_str());
        }
        for sample in fixtures["composition"].as_array().unwrap() {
            let actual = append(
                sample["base"].as_str().unwrap(),
                sample["advertisement"].as_str(),
            );
            assert_eq!(actual.content, sample["content"].as_str().unwrap());
            assert_eq!(actual.result, sample["result"].as_str().unwrap());
        }
    }
}
