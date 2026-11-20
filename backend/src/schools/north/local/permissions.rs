use super::Service;
use crate::{
    business::{Actor, Result, require},
    schools::{
        domain::permissions::{
            Action, CATALOG_VERSION, MemberQuery, Recommendation, Save, catalog,
        },
        south::port::permissions::Store,
    },
};
use serde_json::{Value, json};
use uuid::Uuid;

impl<R: super::Store + Store> Service<R> {
    pub async fn permission_catalog(&self, actor: &Actor, school: Uuid) -> Result<Value> {
        self.0.permission_snapshot(actor, school, actor.id).await?;
        Ok(
            json!({"id":"current","catalog_version":CATALOG_VERSION,"permissions":catalog(),"editable":actor.platform_admin,"uneditable_reason":if actor.platform_admin {""} else {"通用权限仅平台可配置"}}),
        )
    }
    pub async fn member_permissions(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
    ) -> Result<Value> {
        self.0.permission_snapshot(actor, school, target).await
    }
    pub async fn management_members(
        &self,
        actor: &Actor,
        school: Uuid,
        query: &MemberQuery,
        candidates: bool,
    ) -> Result<Vec<Value>> {
        self.0
            .management_members(actor, school, query, candidates)
            .await
    }
    pub async fn save_authorization(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        input: Save,
    ) -> Result<Value> {
        require(
            matches!(input.role.as_str(), "admin" | "operator"),
            "授予角色仅支持管理员或运营；取消职责使用独立操作",
        )?;
        input.validate()?;
        self.0
            .save_authorization(actor, school, target, input, None)
            .await
    }
    pub async fn authorization_action(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        input: Action,
    ) -> Result<Value> {
        let role = match input.action.as_str() {
            "remove_management" => "member",
            "revoke" => "revoked",
            _ => return Err(crate::business::Error::Invalid("授权操作无效".into())),
        };
        self.0
            .save_authorization(
                actor,
                school,
                target,
                Save {
                    catalog_version: input.catalog_version,
                    expected_revision: input.expected_revision,
                    role: role.into(),
                    overrides: vec![],
                    reason: input.reason,
                },
                None,
            )
            .await
    }
    pub async fn set_recommendation_permission(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        input: Recommendation,
    ) -> Result<Value> {
        self.0
            .save_authorization(
                actor,
                school,
                target,
                Save {
                    catalog_version: input.catalog_version,
                    expected_revision: input.expected_revision,
                    role: "operator".into(),
                    overrides: vec![],
                    reason: input.reason,
                },
                Some(&input.effect),
            )
            .await
    }
}
