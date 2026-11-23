use crate::business::{Error, Result, require, text};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

pub const CATALOG_VERSION: &str = "2026-11-23.1";

pub fn artwork_context(permission: &str, writing: bool) -> Result<&'static str> {
    match permission {
        "resource.school.write" => Ok("resource.school."),
        "school.profile.update" => Ok("school.profile."),
        "operation.qrcode.create" | "operation.qrcode.update" => Ok("operation.qrcode."),
        "operation.banner.create" | "operation.banner.update" => Ok("operation.banner."),
        "operation.shortcut.create" | "operation.shortcut.update" => Ok("operation.shortcut."),
        "operation.shortcut.read" if !writing => Ok("operation.shortcut."),
        "resource.school.read" if !writing => Ok("resource.school."),
        "school.profile.read" if !writing => Ok("school.profile."),
        "operation.qrcode.read" if !writing => Ok("operation.qrcode."),
        "operation.banner.read" if !writing => Ok("operation.banner."),
        _ => Err(Error::Forbidden),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Definition {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: String,
    pub group: String,
    pub applicable_roles: Vec<String>,
    pub default_by_role: BTreeMap<String, bool>,
    pub requires: Vec<String>,
    pub high_risk: bool,
}

pub fn catalog() -> &'static [Definition] {
    static CATALOG: OnceLock<Vec<Definition>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/school-permission-catalog.json"
        ))
        .expect("permission catalog JSON");
        serde_json::from_value(document["data"]["attributes"]["permissions"].clone())
            .expect("permission catalog definitions")
    })
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub permission_key: String,
    pub effect: Effect,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    pub catalog_version: String,
    pub expected_revision: String,
    pub role: String,
    pub overrides: Vec<Override>,
    pub reason: String,
}

impl Save {
    pub fn validate(&self) -> Result<i64> {
        if self.catalog_version != CATALOG_VERSION {
            return Err(Error::Conflict);
        }
        require(
            matches!(
                self.role.as_str(),
                "admin" | "operator" | "member" | "revoked"
            ),
            "角色无效",
        )?;
        text(&self.reason, 1, 500)?;
        let revision = self
            .expected_revision
            .parse::<i64>()
            .map_err(|_| Error::Invalid("授权版本无效".into()))?;
        require(
            revision >= 0 && revision.to_string() == self.expected_revision,
            "授权版本无效",
        )?;
        require(self.overrides.len() <= catalog().len(), "权限覆盖过多")?;
        let mut seen = BTreeSet::new();
        for item in &self.overrides {
            require(seen.insert(&item.permission_key), "权限覆盖重复")?;
            let definition = catalog()
                .iter()
                .find(|definition| definition.key == item.permission_key)
                .ok_or_else(|| Error::Invalid("未知权限".into()))?;
            require(
                definition.applicable_roles.contains(&self.role),
                "权限不适用于目标角色",
            )?;
        }
        let effective = resolve(&self.role, &self.overrides);
        for definition in catalog()
            .iter()
            .filter(|definition| effective.contains(&definition.key))
        {
            for dependency in &definition.requires {
                require(
                    effective.contains(dependency),
                    &format!("{} 缺少依赖 {}", definition.key, dependency),
                )?;
            }
        }
        Ok(revision)
    }
}

pub fn resolve(role: &str, overrides: &[Override]) -> BTreeSet<String> {
    catalog()
        .iter()
        .filter(|definition| {
            definition
                .applicable_roles
                .iter()
                .any(|candidate| candidate == role)
        })
        .filter(|definition| {
            overrides
                .iter()
                .find(|item| item.permission_key == definition.key)
                .map(|item| item.effect == Effect::Allow)
                .unwrap_or(*definition.default_by_role.get(role).unwrap_or(&false))
        })
        .map(|definition| definition.key.clone())
        .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub catalog_version: String,
    pub expected_revision: String,
    pub action: String,
    pub reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recommendation {
    pub catalog_version: String,
    pub expected_revision: String,
    pub effect: String,
    pub reason: String,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberQuery {
    #[serde(rename = "page[number]")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]")]
    pub page_size: Option<i64>,
    pub search: Option<String>,
    pub role: Option<String>,
    pub has_overrides: Option<bool>,
}

impl MemberQuery {
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let page = crate::business::Page {
            page: self.page,
            page_size: self.page_size,
            ..Default::default()
        };
        page.fetch_bounds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(role: &str, overrides: Vec<Override>) -> Save {
        Save {
            catalog_version: CATALOG_VERSION.into(),
            expected_revision: "0".into(),
            role: role.into(),
            overrides,
            reason: "职责配置".into(),
        }
    }
    #[test]
    fn simulation_is_an_operating_capability_for_administrators_and_operators() {
        for role in ["admin", "operator"] {
            assert!(resolve(role, &[]).contains("content.simulation.manage"));
            let denied = input(
                role,
                vec![Override {
                    permission_key: "content.simulation.manage".into(),
                    effect: Effect::Deny,
                }],
            );
            denied.validate().unwrap();
            assert!(!resolve(role, &denied.overrides).contains("content.simulation.manage"));
        }
        assert!(!resolve("member", &[]).contains("content.simulation.manage"));
    }
    #[test]
    fn catalog_is_complete_unique_and_dependencies_are_known() {
        assert_eq!(catalog().len(), 61);
        assert!(catalog().iter().all(|definition| !matches!(
            definition.key.as_str(),
            "audit.school.read" | "source.binding.read"
        )));
        let keys: BTreeSet<_> = catalog().iter().map(|definition| &definition.key).collect();
        assert_eq!(keys.len(), 61);
        for definition in catalog() {
            assert!(
                definition
                    .requires
                    .iter()
                    .all(|dependency| keys.contains(dependency))
            );
            let mut pending = definition.requires.clone();
            let mut visited = BTreeSet::new();
            while let Some(key) = pending.pop() {
                assert_ne!(key, definition.key, "cyclic permission dependency");
                if visited.insert(key.clone()) {
                    pending.extend(
                        catalog()
                            .iter()
                            .find(|item| item.key == key)
                            .unwrap()
                            .requires
                            .clone(),
                    );
                }
            }
        }
        input("admin", vec![]).validate().unwrap();
        input("operator", vec![]).validate().unwrap();
        assert!(resolve("member", &[]).is_empty());
        assert!(resolve("revoked", &[]).is_empty());
    }
    #[test]
    fn input_fixture_preserves_large_decimal_revisions_and_rejects_extra_fields() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/school-member-authorization-input.json"
        ))
        .unwrap();
        let input: Save = serde_json::from_value(fixture["data"]["attributes"].clone()).unwrap();
        assert_eq!(input.validate().unwrap(), 9_007_199_254_740_993);
        let mut attributes = fixture["data"]["attributes"].clone();
        attributes["enabled"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Save>(attributes).is_err());
    }
    #[test]
    fn deny_overrides_administrator_and_dependencies_are_not_implicitly_granted() {
        let denied = Override {
            permission_key: "content.post.review".into(),
            effect: Effect::Deny,
        };
        let save = input("admin", vec![denied]);
        save.validate().unwrap();
        assert!(!resolve("admin", &save.overrides).contains("content.post.review"));
        assert!(
            input(
                "admin",
                vec![Override {
                    permission_key: "content.post.read".into(),
                    effect: Effect::Deny
                }]
            )
            .validate()
            .is_err()
        );
    }
    #[test]
    fn rejects_unknown_duplicate_and_inapplicable_permissions() {
        for key in ["unknown", "school.member.recommendation.assign"] {
            assert!(
                input(
                    "operator",
                    vec![Override {
                        permission_key: key.into(),
                        effect: Effect::Allow
                    }]
                )
                .validate()
                .is_err()
            );
        }
        let item = Override {
            permission_key: "post.recommend".into(),
            effect: Effect::Allow,
        };
        assert!(
            input("operator", vec![item.clone(), item])
                .validate()
                .is_err()
        );
        let mut save = input("operator", vec![]);
        save.catalog_version = "outdated".into();
        assert!(matches!(save.validate(), Err(Error::Conflict)));
    }
    #[test]
    fn every_applicable_leaf_supports_allow_deny_and_inherit() {
        for role in ["admin", "operator"] {
            for definition in catalog().iter().filter(|item| {
                item.applicable_roles
                    .iter()
                    .any(|candidate| candidate == role)
            }) {
                let mut selected = BTreeSet::new();
                let mut pending = vec![definition.key.clone()];
                while let Some(key) = pending.pop() {
                    if selected.insert(key.clone()) {
                        pending.extend(
                            catalog()
                                .iter()
                                .find(|item| item.key == key)
                                .unwrap()
                                .requires
                                .clone(),
                        );
                    }
                }
                let overrides: Vec<Override> = catalog()
                    .iter()
                    .filter(|item| {
                        item.applicable_roles
                            .iter()
                            .any(|candidate| candidate == role)
                    })
                    .map(|item| Override {
                        permission_key: item.key.clone(),
                        effect: if selected.contains(&item.key) {
                            Effect::Allow
                        } else {
                            Effect::Deny
                        },
                    })
                    .collect();
                input(role, overrides.clone()).validate().unwrap();
                assert!(resolve(role, &overrides).contains(&definition.key));
                let denied: Vec<Override> = overrides
                    .iter()
                    .map(|item| Override {
                        permission_key: item.permission_key.clone(),
                        effect: if item.permission_key == definition.key {
                            Effect::Deny
                        } else {
                            item.effect.clone()
                        },
                    })
                    .collect();
                assert!(!resolve(role, &denied).contains(&definition.key));
                let inherited: Vec<Override> = denied
                    .into_iter()
                    .filter(|item| item.permission_key != definition.key)
                    .collect();
                assert_eq!(
                    resolve(role, &inherited).contains(&definition.key),
                    definition.default_by_role[role]
                );
            }
        }
    }
}
