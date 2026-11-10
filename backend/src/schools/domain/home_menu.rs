use crate::business::{Result, require, text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuStatus {
    Shown,
    Hot,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Admin,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnableStatus {
    Enabled,
    Pending,
    Disabled,
}

pub fn is_public(source: SourceType, enabled: EnableStatus, menu: MenuStatus) -> bool {
    source == SourceType::Admin && enabled == EnableStatus::Enabled && menu != MenuStatus::Hidden
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagInput {
    pub name: String,
    pub menu_status: MenuStatus,
    pub enable_status: EnableStatus,
    pub revision: Option<i64>,
}

impl TagInput {
    pub fn validate(&self) -> Result<()> {
        text(self.name.trim(), 1, 50)?;
        require(
            self.revision.is_none_or(|revision| revision > 0),
            "修订无效",
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderInput {
    pub ids: Vec<Uuid>,
    pub catalog_revision: i64,
}

impl OrderInput {
    pub fn validate(&self) -> Result<()> {
        let unique: std::collections::HashSet<_> = self.ids.iter().collect();
        require(
            self.catalog_revision > 0 && self.ids.len() <= 500 && unique.len() == self.ids.len(),
            "排序列表或修订无效",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_candidates_require_all_three_states() {
        for source in [SourceType::Admin, SourceType::User] {
            for enabled in [
                EnableStatus::Enabled,
                EnableStatus::Pending,
                EnableStatus::Disabled,
            ] {
                for menu in [MenuStatus::Shown, MenuStatus::Hot, MenuStatus::Hidden] {
                    assert_eq!(
                        is_public(source, enabled, menu),
                        source == SourceType::Admin
                            && enabled == EnableStatus::Enabled
                            && menu != MenuStatus::Hidden
                    );
                }
            }
        }
    }

    #[test]
    fn order_rejects_duplicates_and_large_catalogs() {
        let id = Uuid::new_v4();
        assert!(
            OrderInput {
                ids: vec![id, id],
                catalog_revision: 1
            }
            .validate()
            .is_err()
        );
        assert!(
            OrderInput {
                ids: (0..501).map(|_| Uuid::new_v4()).collect(),
                catalog_revision: 1
            }
            .validate()
            .is_err()
        );
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsInput {
    pub latest_label: String,
    pub latest_visible: bool,
    pub revision: i64,
}
impl SettingsInput {
    pub fn validate(&self) -> Result<()> {
        text(&self.latest_label, 1, 50)?;
        require(self.revision > 0, "修订无效")
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CatalogQuery {
    pub search: Option<String>,
    pub menu_status: Option<MenuStatus>,
    pub source_type: Option<SourceType>,
    pub enable_status: Option<EnableStatus>,
    #[serde(rename = "page[number]")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]")]
    pub page_size: Option<i64>,
}
impl CatalogQuery {
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let page = self.page.unwrap_or(1);
        let size = self.page_size.unwrap_or(20);
        require(
            (1..=100_000).contains(&page) && (1..=100).contains(&size),
            "分页参数不合法",
        )?;
        text(self.search.as_deref().unwrap_or(""), 0, 50)?;
        Ok((size, (page - 1) * size))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCommit {
    pub digest: String,
}
