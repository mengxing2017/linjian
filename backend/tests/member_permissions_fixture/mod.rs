use linjian_api::{
    business::{Actor, Result},
    schools::{
        domain::permissions::{CATALOG_VERSION, MemberQuery, Recommendation},
        north::local::Service,
        south::adapter::postgres::Repository,
    },
};
use serde_json::Value;
use uuid::Uuid;

pub(crate) trait SchoolFixtures {
    async fn grant(&self, actor: &Actor, school: Uuid, target: Uuid, role: &str) -> Result<()>;
    async fn recommendation_capability(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        enabled: bool,
    ) -> Result<()>;
    async fn members(&self, actor: &Actor, school: Uuid) -> Result<Vec<Value>>;
}
impl SchoolFixtures for Service<Repository> {
    async fn grant(&self, actor: &Actor, school: Uuid, target: Uuid, role: &str) -> Result<()> {
        actor.platform()?;
        let mut transaction = self.0.0.begin().await?;
        sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,$3) ON CONFLICT(school_id,account_id) DO UPDATE SET role=EXCLUDED.role").bind(school).bind(target).bind(role).execute(&mut *transaction).await?;
        sqlx::query(
            "DELETE FROM school_member_permission_overrides WHERE school_id=$1 AND account_id=$2",
        )
        .bind(school)
        .bind(target)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("INSERT INTO school_member_authorizations(school_id,account_id,revision,updated_by) VALUES($1,$2,1,$3) ON CONFLICT(school_id,account_id) DO UPDATE SET revision=school_member_authorizations.revision+1").bind(school).bind(target).bind(actor.id).execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
    async fn recommendation_capability(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        enabled: bool,
    ) -> Result<()> {
        let snapshot = self.member_permissions(actor, school, target).await?;
        self.set_recommendation_permission(
            actor,
            school,
            target,
            Recommendation {
                catalog_version: CATALOG_VERSION.into(),
                expected_revision: snapshot["revision"].as_str().unwrap().into(),
                effect: if enabled { "allow" } else { "inherit" }.into(),
                reason: "测试配置推荐权限".into(),
            },
        )
        .await?;
        Ok(())
    }
    async fn members(&self, actor: &Actor, school: Uuid) -> Result<Vec<Value>> {
        self.management_members(actor, school, &MemberQuery::default(), false)
            .await
    }
}
