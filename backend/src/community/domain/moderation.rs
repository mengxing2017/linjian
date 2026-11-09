use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditMode {
    Manual,
    Auto,
}
impl AuditMode {
    pub fn from_config(value: &serde_json::Value, key: &str) -> Self {
        if value[key] == "auto" {
            Self::Auto
        } else {
            Self::Manual
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionSource {
    Human,
    Automatic,
    Legacy,
}
impl DecisionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Automatic => "automatic",
            Self::Legacy => "legacy",
        }
    }
}
