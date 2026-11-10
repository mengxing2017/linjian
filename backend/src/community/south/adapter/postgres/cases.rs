use super::{Repository, reviews::AUTHORIZATION};
use crate::{
    business::{Actor, Error, Page, Result, require, text},
    community::domain::case::{CaseManagementView, ResolveCaseInput},
};
use serde_json::{Value, json};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

const JOINS: &str = " FROM cases c JOIN schools s ON s.id=c.school_id LEFT JOIN posts p ON p.school_id=c.school_id AND p.id=c.post_id LEFT JOIN accounts a ON a.id=c.account_id LEFT JOIN miniapp_users u ON u.account_id=c.account_id ";
const TITLE: &str = "CASE WHEN c.kind='feedback' THEN '意见反馈' ELSE coalesce(nullif(p.title,''),'举报内容不可用') END";
const REPORTER: &str =
    "coalesce(nullif(btrim(u.nickname),''),nullif(btrim(a.display_name),''),'未知用户')";
fn projection() -> String {
    format!(
        "to_jsonb(c)||jsonb_build_object('title',{TITLE},'reporter_name',{REPORTER},'school_name',s.name)"
    )
}
async fn management(
    connection: &mut PgConnection,
    school: Uuid,
    id: Uuid,
) -> Result<CaseManagementView> {
    let value: Value = sqlx::query_scalar(&format!(
        "SELECT {}{JOINS} WHERE c.deleted_at IS NULL AND c.school_id=$1 AND c.id=$2",
        projection()
    ))
    .bind(school)
    .bind(id)
    .fetch_one(connection)
    .await?;
    serde_json::from_value(value).map_err(|_| Error::Unavailable)
}
impl Repository {
    pub async fn cases(&self, a: &Actor, school: Uuid, page: Page) -> Result<Vec<Value>> {
        crate::schools::south::adapter::postgres::Repository(self.0.clone())
            .require_active(school)
            .await?;
        let managing = a.can(school, "operator");
        if managing {
            let allowed: bool = sqlx::query_scalar(AUTHORIZATION)
                .bind(a.id)
                .bind(school)
                .fetch_one(&self.0)
                .await?;
            if !allowed {
                return Err(Error::Forbidden);
            }
        }
        require(
            page.status
                .as_deref()
                .is_none_or(|v| matches!(v, "" | "open" | "resolved" | "dismissed")),
            "处理状态无效",
        )?;
        let search = text(page.search.as_deref().unwrap_or(""), 0, 200)?;
        let (limit, offset) = page.fetch_bounds()?;
        let selection = if managing {
            projection()
        } else {
            "to_jsonb(c)-'handled_at'-'handled_by'-'submitted_ip'-'deleted_by'-'submission_key'"
                .into()
        };
        let rows: Vec<Value> = sqlx::query_scalar(&format!("SELECT {selection}{JOINS} WHERE c.deleted_at IS NULL AND ($8::text IS NULL OR c.kind=$8) AND c.school_id=$1 AND ($2 OR c.account_id=$3) AND ($4::text IS NULL OR $4='' OR c.state=$4) AND (c.content ILIKE '%'||$5||'%' OR ($2 AND ({TITLE} ILIKE '%'||$5||'%' OR {REPORTER} ILIKE '%'||$5||'%'))) ORDER BY c.created_at DESC,c.id DESC LIMIT $6 OFFSET $7"))
            .bind(school).bind(managing).bind(a.id).bind(page.status).bind(search).bind(limit).bind(offset).bind(page.kind).fetch_all(&self.0).await?;
        if managing {
            rows.into_iter()
                .map(|row| {
                    let view: CaseManagementView =
                        serde_json::from_value(row).map_err(|_| Error::Unavailable)?;
                    serde_json::to_value(view).map_err(|_| Error::Unavailable)
                })
                .collect()
        } else {
            Ok(rows)
        }
    }
    pub async fn resolve(
        &self,
        a: &Actor,
        school: Uuid,
        id: Uuid,
        mut input: ResolveCaseInput,
    ) -> Result<CaseManagementView> {
        a.authorize(school, "operator")?;
        input.validate()?;
        let mut tx = self.0.begin().await?;
        // Lock live authorization rows to serialize handling with account/role revocation.
        let account =
            sqlx::query("SELECT active,platform_admin FROM accounts WHERE id=$1 FOR SHARE")
                .bind(a.id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::Forbidden)?;
        let role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM school_memberships WHERE account_id=$1 AND school_id=$2 FOR SHARE",
        )
        .bind(a.id)
        .bind(school)
        .fetch_optional(&mut *tx)
        .await?;
        if !account.get::<bool, _>("active")
            || !(account.get::<bool, _>("platform_admin")
                || role.is_some_and(|r| matches!(r.as_str(), "operator" | "admin")))
        {
            return Err(Error::Forbidden);
        }
        crate::miniapp_users::south::adapter::postgres::guard_write(&mut tx, a, school).await?;
        let previous: Value = sqlx::query_scalar(
            "SELECT to_jsonb(c) FROM cases c WHERE school_id=$1 AND id=$2 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(school)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Conflict)?;
        if previous["revision"].as_i64() != Some(input.revision) {
            return Err(Error::Conflict);
        }
        if previous["state"] != input.state.as_str() || previous["resolution"] != input.resolution {
            sqlx::query("UPDATE cases SET state=$3,resolution=$4,handled_at=clock_timestamp(),updated_at=clock_timestamp(),handled_by=$5,revision=revision+1 WHERE school_id=$1 AND id=$2")
                .bind(school).bind(id).bind(input.state.as_str()).bind(&input.resolution).bind(a.id).execute(&mut *tx).await?;
            let current = management(&mut tx, school, id).await?;
            crate::audit::record(&mut tx,a,school,"case.resolve",id,json!({"previous_state":previous["state"],"state":current.record.state,"previous_revision":previous["revision"],"revision":current.record.revision,"previous_handled_at":previous["handled_at"],"handled_at":current.handled_at})).await?;
        }
        let current = management(&mut tx, school, id).await?;
        tx.commit().await?;
        Ok(current)
    }
}
