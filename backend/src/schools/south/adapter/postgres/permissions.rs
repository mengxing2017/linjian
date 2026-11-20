use super::Repository;
use crate::{
    business::{Actor, Error, Result, require},
    schools::domain::permissions::{self, Effect, MemberQuery, Override, Save},
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub async fn fence(
    tx: &mut Transaction<'_, Postgres>,
    school: Uuid,
    exclusive: bool,
) -> Result<()> {
    let query = if exclusive {
        "SELECT school_id FROM school_authorization_guards WHERE school_id=$1 FOR UPDATE"
    } else {
        "SELECT school_id FROM school_authorization_guards WHERE school_id=$1 FOR SHARE"
    };
    sqlx::query(query)
        .bind(school)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::Unavailable)?;
    Ok(())
}

pub async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    school: Uuid,
    key: &str,
) -> Result<()> {
    if !actor.is_admin_browser() {
        return Err(Error::Forbidden);
    }
    fence(tx, school, false).await?;
    let active: bool = sqlx::query_scalar("SELECT active FROM accounts WHERE id=$1 FOR SHARE")
        .bind(actor.id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::Unauthorized)?;
    if !active {
        return Err(Error::Unauthorized);
    }
    let allowed: bool = sqlx::query_scalar("SELECT forum_has_permission($1,$2,$3) AND EXISTS(SELECT 1 FROM sessions WHERE token_hash=$4 AND account_id=$1 AND kind='admin_browser' AND expires_at>now() AND last_seen_at>now()-interval '30 minutes')").bind(actor.id).bind(school).bind(key).bind(&actor.session_hash).fetch_one(&mut **tx).await?;
    if allowed {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}

impl crate::schools::south::port::permissions::Store for Repository {
    async fn permission_snapshot(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
    ) -> Result<Value> {
        if !actor.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        if actor.id != target {
            actor.permission(school, "school.member.read")?;
        }
        if !actor.platform_admin
            && !actor.grants.iter().any(|grant| {
                grant.school_id == school && matches!(grant.role.as_str(), "admin" | "operator")
            })
        {
            return Err(Error::NotFound);
        }
        let value: Option<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',a.id,'school_id',$1::uuid,'username',a.username,'display_name',a.display_name,'active',a.active,'role',coalesce(m.role,'unassigned'),'mode',CASE WHEN a.platform_admin THEN 'platform' ELSE 'school' END,'revision',coalesce(v.revision,0)::text,'catalog_version',$3::text,'overrides',coalesce((SELECT jsonb_agg(jsonb_build_object('permission_key',o.permission_key,'effect',o.effect) ORDER BY o.permission_key) FROM school_member_permission_overrides o WHERE o.school_id=$1 AND o.account_id=a.id),'[]'::jsonb),'effective_permissions',coalesce((SELECT jsonb_agg(p->>'key') FROM jsonb_array_elements(forum_permission_catalog()) p WHERE forum_has_permission(a.id,$1,p->>'key')),'[]'::jsonb),'updated_at',v.updated_at) FROM accounts a LEFT JOIN school_memberships m ON m.account_id=a.id AND m.school_id=$1 LEFT JOIN school_member_authorizations v ON v.account_id=a.id AND v.school_id=$1 WHERE a.id=$2 AND (a.password_hash IS NOT NULL OR a.platform_admin) AND ($4 OR m.account_id IS NOT NULL)").bind(school).bind(target).bind(permissions::CATALOG_VERSION).bind(actor.platform_admin).fetch_optional(&self.0).await?;
        value.ok_or(Error::NotFound)
    }

    async fn management_members(
        &self,
        actor: &Actor,
        school: Uuid,
        query: &MemberQuery,
        candidates: bool,
    ) -> Result<Vec<Value>> {
        if candidates {
            actor.platform()?;
        } else {
            actor.permission(school, "school.member.read")?;
        }
        let (limit, offset) = query.bounds()?;
        if let Some(role) = &query.role {
            require(
                matches!(role.as_str(), "admin" | "operator" | "member" | "revoked"),
                "角色筛选无效",
            )?;
        }
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',a.id,'username',a.username,'display_name',a.display_name,'active',a.active,'platform_admin',a.platform_admin,'role',coalesce(m.role,'unassigned'),'revision',coalesce(v.revision,0)::text,'override_count',(SELECT count(*) FROM school_member_permission_overrides o WHERE o.school_id=$1 AND o.account_id=a.id),'updated_at',v.updated_at) FROM accounts a LEFT JOIN school_memberships m ON m.account_id=a.id AND m.school_id=$1 LEFT JOIN school_member_authorizations v ON v.account_id=a.id AND v.school_id=$1 WHERE (a.password_hash IS NOT NULL OR a.platform_admin) AND (a.username ILIKE $2 OR a.display_name ILIKE $2) AND (($3 AND (m.role IS NULL OR m.role NOT IN ('admin','operator'))) OR (NOT $3 AND (($4::text IS NULL AND m.role IN ('admin','operator')) OR m.role=$4))) AND ($5::boolean IS NULL OR $5=EXISTS(SELECT 1 FROM school_member_permission_overrides o WHERE o.school_id=$1 AND o.account_id=a.id)) ORDER BY a.username,a.id LIMIT $6 OFFSET $7").bind(school).bind(format!("%{}%",query.search.as_deref().unwrap_or(""))).bind(candidates).bind(&query.role).bind(query.has_overrides).bind(limit).bind(offset).fetch_all(&self.0).await?)
    }

    async fn save_authorization(
        &self,
        actor: &Actor,
        school: Uuid,
        target: Uuid,
        mut input: Save,
        restricted: Option<&str>,
    ) -> Result<Value> {
        let expected = input.validate()?;
        if !actor.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        if restricted.is_none() {
            actor.platform()?;
        }
        let mut tx = self.0.begin().await?;
        fence(&mut tx, school, true).await?;
        let mut accounts = vec![actor.id, target];
        accounts.sort();
        accounts.dedup();
        for id in accounts {
            sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
        }
        if restricted.is_some() && !actor.platform_admin {
            authorize(
                &mut tx,
                actor,
                school,
                "school.member.recommendation.assign",
            )
            .await?;
            authorize(&mut tx, actor, school, "post.recommend").await?;
            require(actor.id != target, "不能为本人配置推荐权限")?;
        } else {
            super::preview::platform(&mut tx, actor).await?;
            let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions WHERE token_hash=$1 AND account_id=$2 AND kind='admin_browser' AND expires_at>now() AND last_seen_at>now()-interval '30 minutes')").bind(&actor.session_hash).bind(actor.id).fetch_one(&mut *tx).await?;
            if !valid {
                return Err(Error::Unauthorized);
            }
        }
        let eligible: bool = sqlx::query_scalar("SELECT active AND password_hash IS NOT NULL AND NOT platform_admin FROM accounts WHERE id=$1").bind(target).fetch_one(&mut *tx).await?;
        require(eligible, "目标必须是有效非平台后台账号")?;
        let role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM school_memberships WHERE school_id=$1 AND account_id=$2 FOR UPDATE",
        )
        .bind(school)
        .bind(target)
        .fetch_optional(&mut *tx)
        .await?;
        let revision: Option<i64> = sqlx::query_scalar("SELECT revision FROM school_member_authorizations WHERE school_id=$1 AND account_id=$2 FOR UPDATE").bind(school).bind(target).fetch_optional(&mut *tx).await?;
        if revision.unwrap_or(0) != expected {
            return Err(Error::Conflict);
        }
        let before: Vec<Override> = sqlx::query("SELECT permission_key,effect FROM school_member_permission_overrides WHERE school_id=$1 AND account_id=$2 ORDER BY permission_key").bind(school).bind(target).fetch_all(&mut *tx).await?.iter().map(|row| Override {permission_key:row.get("permission_key"),effect:if row.get::<String,_>("effect")=="allow" {Effect::Allow} else {Effect::Deny}}).collect();
        if let Some(effect) = restricted {
            require(
                role.as_deref() == Some("operator"),
                "仅本校运营可配置推荐权限",
            )?;
            input.role = "operator".into();
            input.overrides = before
                .iter()
                .filter(|item| item.permission_key != "post.recommend")
                .cloned()
                .collect();
            match effect {
                "inherit" => {}
                "allow" | "deny" => input.overrides.push(Override {
                    permission_key: "post.recommend".into(),
                    effect: if effect == "allow" {
                        Effect::Allow
                    } else {
                        Effect::Deny
                    },
                }),
                _ => return Err(Error::Invalid("覆盖状态无效".into())),
            }
            input.validate()?;
        }
        if matches!(input.role.as_str(), "member" | "revoked") {
            require(role.is_some(), "成员关系不存在")?;
            require(input.overrides.is_empty(), "基础或撤销关系不能携带权限覆盖")?;
            if input.role == "revoked" {
                let bound: bool = sqlx::query_scalar("SELECT coalesce(bound_school_id=$1,false) AND EXISTS(SELECT 1 FROM miniapp_users WHERE account_id=$2) FROM accounts WHERE id=$2").bind(school).bind(target).fetch_one(&mut *tx).await?;
                require(!bound, "当前社区归属请通过用户管理处理")?;
            }
        }
        let active: bool = sqlx::query_scalar("SELECT active FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_one(&mut *tx)
            .await?;
        require(active, "学校不可用")?;
        let next = expected.checked_add(1).ok_or(Error::Conflict)?;
        sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,$3) ON CONFLICT(school_id,account_id) DO UPDATE SET role=EXCLUDED.role").bind(school).bind(target).bind(&input.role).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO school_member_authorizations(school_id,account_id,revision,updated_by) VALUES($1,$2,$3,$4) ON CONFLICT(school_id,account_id) DO UPDATE SET revision=EXCLUDED.revision,updated_by=EXCLUDED.updated_by,updated_at=now()").bind(school).bind(target).bind(next).bind(actor.id).execute(&mut *tx).await?;
        sqlx::query(
            "DELETE FROM school_member_permission_overrides WHERE school_id=$1 AND account_id=$2",
        )
        .bind(school)
        .bind(target)
        .execute(&mut *tx)
        .await?;
        for item in &input.overrides {
            sqlx::query("INSERT INTO school_member_permission_overrides(school_id,account_id,permission_key,effect,updated_by) VALUES($1,$2,$3,$4,$5)").bind(school).bind(target).bind(&item.permission_key).bind(if item.effect==Effect::Allow {"allow"} else {"deny"}).bind(actor.id).execute(&mut *tx).await?;
        }
        crate::audit::record(&mut tx,actor,school,"school.member.authorization.save",target,json!({"before_role":role,"after_role":input.role,"before_overrides":before,"after_overrides":input.overrides,"before_effective":permissions::resolve(role.as_deref().unwrap_or(""),&before),"after_effective":permissions::resolve(&input.role,&input.overrides),"before_revision":expected.to_string(),"revision":next.to_string(),"reason":input.reason})).await?;
        tx.commit().await?;
        self.permission_snapshot(actor, school, target).await
    }
}
