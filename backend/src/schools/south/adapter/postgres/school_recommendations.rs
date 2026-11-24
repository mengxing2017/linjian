use super::Repository;
use crate::{
    business::{Actor, Error, Result, hash, require},
    schools::{
        domain::{
            profile::SchoolView,
            school_recommendations::{LIMIT, Query, Recommendation, Scene},
        },
        south::port::school_recommendations::Store,
    },
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

async fn ranked(
    transaction: &mut Transaction<'_, Postgres>,
    query: &Query,
    member: Option<Uuid>,
) -> Result<Vec<Recommendation>> {
    let rows = sqlx::query(&format!(
        "WITH candidates AS ({view} WHERE s.active AND s.latitude IS NOT NULL AND s.longitude IS NOT NULL AND ($3::uuid IS NULL OR EXISTS(SELECT 1 FROM school_memberships m WHERE m.school_id=s.id AND m.account_id=$3 AND m.role<>'revoked'))), distances AS (SELECT candidates.*, 2*6371008.8*asin(sqrt(least(1::float8,greatest(0::float8,power(sin(radians(latitude-$1)/2),2)+cos(radians($1))*cos(radians(latitude))*power(sin(radians(longitude-$2)/2),2))))) AS distance_m FROM candidates) SELECT to_jsonb(distances)-'distance_m' AS school,distance_m FROM distances ORDER BY distance_m,id LIMIT $4",
        view = super::profiles::VIEW
    ))
        .bind(query.latitude).bind(query.longitude).bind(member).bind(LIMIT)
        .fetch_all(&mut **transaction).await?;
    rows.into_iter()
        .map(|row| {
            let school: SchoolView =
                serde_json::from_value(row.get("school")).map_err(|_| Error::Unavailable)?;
            Ok(Recommendation {
                school: school.directory(),
                distance_m: row.get("distance_m"),
            })
        })
        .collect()
}

impl Store for Repository {
    async fn school_recommendations(
        &self,
        actor: &Actor,
        query: &Query,
    ) -> Result<Vec<Recommendation>> {
        let mut transaction = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *transaction)
            .await?;
        crate::miniapp_users::south::adapter::postgres::guard_session(&mut transaction, actor)
            .await?;
        let account = sqlx::query("SELECT a.bound_school_id FROM accounts a JOIN miniapp_users u ON u.account_id=a.id WHERE a.id=$1 AND a.active AND u.status='normal'")
            .bind(actor.id).fetch_optional(&mut *transaction).await?.ok_or(Error::Unauthorized)?;
        let bound: Option<Uuid> = account.get("bound_school_id");
        require(
            (query.scene == Scene::Binding) == bound.is_none(),
            "学校状态已变化，请刷新后重试",
        )?;
        let rows = ranked(&mut transaction, query, bound.map(|_| actor.id)).await?;
        transaction.commit().await?;
        Ok(rows)
    }

    async fn onboarding_school_recommendations(
        &self,
        ticket: &str,
        query: &Query,
    ) -> Result<Vec<Recommendation>> {
        let mut transaction = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *transaction)
            .await?;
        let attempt = sqlx::query("SELECT a.id,a.active,a.bound_school_id,u.status FROM miniapp_onboarding_attempts t LEFT JOIN miniapp_identities i ON i.app_id=t.app_id AND i.openid=t.openid LEFT JOIN accounts a ON a.id=i.account_id LEFT JOIN miniapp_users u ON u.account_id=a.id WHERE t.token_hash=$1 AND NOT t.cancelled AND NOT t.completed AND t.expires_at>now()")
            .bind(hash(ticket)).fetch_optional(&mut *transaction).await?.ok_or(Error::Unauthorized)?;
        if attempt.get::<Option<Uuid>, _>("id").is_some() {
            require(
                attempt.get::<Option<bool>, _>("active") == Some(true)
                    && attempt.get::<Option<String>, _>("status").as_deref() == Some("normal"),
                "账号暂不可用",
            )?;
            require(
                attempt.get::<Option<Uuid>, _>("bound_school_id").is_none(),
                "账号已绑定学校，请继续原登录流程",
            )?;
        }
        let rows = ranked(&mut transaction, query, None).await?;
        transaction.commit().await?;
        Ok(rows)
    }
}
