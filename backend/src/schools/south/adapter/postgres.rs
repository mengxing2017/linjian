use crate::business::{Actor, Error, Result};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
impl Repository {
    pub async fn list(&self, a: &Actor) -> Result<Vec<Value>> {
        self.school_list(a).await
    }
    pub async fn bind(&self, a: &Actor, school: Uuid) -> Result<()> {
        let mut tx = crate::media::south::adapter::lifecycle::begin(&self.0).await?;
        if !a.is_admin_browser() {
            let (_, status, _) =
                crate::miniapp_users::south::adapter::postgres::lock_user(&mut tx, a.id).await?;
            if status != "normal" {
                return Err(Error::MiniappBanned);
            }
            crate::miniapp_users::south::adapter::postgres::guard_session(&mut tx, a).await?;
        }
        let current: Option<Uuid> =
            sqlx::query_scalar("SELECT bound_school_id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(a.id)
                .fetch_one(&mut *tx)
                .await?;
        if current.is_some_and(|s| s != school) {
            return Err(Error::Forbidden);
        }
        sqlx::query("SELECT id FROM schools WHERE id=$1 AND active FOR SHARE")
            .bind(school)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::SchoolUnavailable)?;
        if !a.is_admin_browser() {
            sqlx::query("UPDATE miniapp_users SET revision=revision+1,updated_at=now() WHERE account_id=$1 AND $2::uuid IS NULL").bind(a.id).bind(current).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE accounts SET bound_school_id=$2 WHERE id=$1")
            .bind(a.id)
            .bind(school)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO school_memberships VALUES($1,$2,'member') ON CONFLICT DO NOTHING")
            .bind(school)
            .bind(a.id)
            .execute(&mut *tx)
            .await?;
        crate::audit::record(&mut tx, a, school, "school.bind", a.id, json!({})).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn members(&self, school: Uuid) -> Result<Vec<Value>> {
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',a.id,'username',a.username,'display_name',a.display_name,'role',m.role,'can_recommend',m.role='admin' OR EXISTS(SELECT 1 FROM school_member_capabilities c WHERE c.school_id=m.school_id AND c.account_id=m.account_id AND c.capability='post.recommend')) FROM school_memberships m JOIN accounts a ON a.id=m.account_id WHERE school_id=$1 ORDER BY a.username").bind(school).fetch_all(&self.0).await?)
    }
    pub async fn grant(&self, a: &Actor, school: Uuid, id: Uuid, role: &str) -> Result<()> {
        let mut tx = crate::media::south::adapter::lifecycle::begin(&self.0).await?;
        a.platform()?;
        preview::platform(&mut tx, a).await?;
        if role == "revoked" {
            sqlx::query(
                "UPDATE school_memberships SET role='revoked' WHERE school_id=$1 AND account_id=$2",
            )
            .bind(school)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("INSERT INTO school_memberships VALUES($1,$2,$3) ON CONFLICT(school_id,account_id) DO UPDATE SET role=EXCLUDED.role").bind(school).bind(id).bind(role).execute(&mut *tx).await?;
        }
        crate::audit::record(&mut tx, a, school, "school.grant", id, json!({"role":role})).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn resources(&self, school: Uuid, operator: bool) -> Result<Vec<Value>> {
        if !operator {
            return crate::operations::south::adapter::postgres::effective_resources(
                &self.0, school,
            )
            .await;
        }
        Ok(sqlx::query_scalar("SELECT to_jsonb(r) FROM school_resources r WHERE deleted_at IS NULL AND kind IN ('service','legal') AND (school_id=$1 OR school_id IS NULL) ORDER BY sort_order DESC,id").bind(school).fetch_all(&self.0).await?)
    }
    pub async fn click(&self, _school: Uuid, _id: Uuid) -> Result<()> {
        Err(Error::Invalid("请使用运营中心广告事件接口上报点击".into()))
    }
}

impl crate::schools::south::port::Store for Repository {
    async fn list(&self, a: &Actor) -> Result<Vec<Value>> {
        Repository::list(self, a).await
    }
    async fn bind(&self, a: &Actor, school: Uuid) -> Result<()> {
        Repository::bind(self, a, school).await
    }
    async fn members(&self, school: Uuid) -> Result<Vec<Value>> {
        Repository::members(self, school).await
    }
    async fn grant(&self, a: &Actor, school: Uuid, id: Uuid, role: &str) -> Result<()> {
        Repository::grant(self, a, school, id, role).await
    }
    async fn resources(&self, school: Uuid, operator: bool) -> Result<Vec<Value>> {
        Repository::resources(self, school, operator).await
    }
    async fn click(&self, school: Uuid, id: Uuid) -> Result<()> {
        Repository::click(self, school, id).await
    }
}

pub mod resources;

pub mod preview;

pub mod profiles;

pub mod blocked_words;
pub mod home_menu;

impl Repository {
    pub async fn recommendation_capability(
        &self,
        a: &Actor,
        school: Uuid,
        id: Uuid,
        enabled: bool,
    ) -> Result<()> {
        use sqlx::Row;
        a.authorize(school, "admin")?;
        let mut tx = self.0.begin().await?;
        preview::platform_or_school_admin(&mut tx, a, school).await?;
        let row = sqlx::query(
            "SELECT role FROM school_memberships WHERE school_id=$1 AND account_id=$2 FOR UPDATE",
        )
        .bind(school)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(crate::business::Error::NotFound)?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_one(&mut *tx)
            .await?;
        crate::business::require(
            row.get::<String, _>("role") == "operator",
            "仅运营成员可授予独立推荐权限",
        )?;
        if enabled {
            sqlx::query("INSERT INTO school_member_capabilities(school_id,account_id,capability,granted_by) VALUES($1,$2,'post.recommend',$3) ON CONFLICT DO NOTHING").bind(school).bind(id).bind(a.id).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM school_member_capabilities WHERE school_id=$1 AND account_id=$2 AND capability='post.recommend'").bind(school).bind(id).execute(&mut *tx).await?;
        }
        crate::audit::record(
            &mut tx,
            a,
            school,
            "school.recommendation_capability",
            id,
            json!({"enabled":enabled}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
