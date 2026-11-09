use crate::{
    business::{Actor, Error, Result, text},
    content_publication::{
        domain::{Failure, Link, Policy, Preparation, Retry},
        south::port::PreparationStore,
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
impl Repository {
    pub async fn retry(&self, a: &Actor, school: Uuid, id: Uuid, input: Retry) -> Result<()> {
        if !a.browser || !a.can(school, "operator") {
            return Err(Error::NotFound);
        }
        let reason = text(&input.reason, 1, 500)?;
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(school)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum_outbox WHERE id=$1 AND school_id=$2 AND request_body IS NULL AND forum_publication_is_current(id))").bind(id).bind(school).fetch_one(&mut *tx).await?;
        if !exists {
            return Err(Error::Conflict);
        }
        let n=sqlx::query("UPDATE forum_publication_preparations SET state='pending',last_error_code=NULL,next_attempt_at=now(),revision=revision+1 WHERE event_id=$1 AND revision=$2 AND state IN ('blocked','retry_wait') AND (lease_until IS NULL OR lease_until<=now())").bind(id).bind(input.revision).execute(&mut *tx).await?.rows_affected();
        if n == 0 {
            return Err(Error::Conflict);
        }
        crate::audit::record(
            &mut tx,
            a,
            school,
            "source_preparation.retry",
            id,
            json!({"reason":reason}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn cached_link(&self, w: &Preparation) -> Result<Option<Link>> {
        let key = w.policy.cache_key(w.school, w.post, &w.title);
        let row=sqlx::query("SELECT link,generated_at,expires_at FROM miniapp_link_assets WHERE cache_key=$1 AND (expires_at IS NULL OR expires_at>now()+make_interval(secs=>$2)) ORDER BY generated_at DESC,id DESC LIMIT 1")
            .bind(&key).bind(w.policy.min_remaining as f64).fetch_optional(&self.0).await?;
        Ok(row.map(|r| Link {
            value: r.get("link"),
            generated_at: r.get("generated_at"),
            expires_at: r.get("expires_at"),
        }))
    }
    pub async fn reserve_link(&self, w: &Preparation) -> Result<bool> {
        let result=sqlx::query("INSERT INTO miniapp_link_leases(cache_key,lease_token,lease_until) VALUES($1,$2,now()+interval '90 seconds') ON CONFLICT(cache_key) DO UPDATE SET lease_token=excluded.lease_token,lease_until=excluded.lease_until WHERE miniapp_link_leases.lease_until<now()")
            .bind(w.policy.cache_key(w.school,w.post,&w.title)).bind(w.token).execute(&self.0).await?;
        Ok(result.rows_affected() > 0)
    }
}
impl PreparationStore for Repository {
    async fn claim(&self) -> Result<Option<Preparation>> {
        let enabled = std::env::var("LINJIAN_MINIAPP_SOURCE_ENABLED")
            .unwrap_or_else(|_| "true".into())
            .parse::<bool>()
            .map_err(|_| Error::Invalid("小程序来源开关配置无效".into()))?;
        if !enabled {
            return Ok(None);
        }
        let current = Policy::from_env()?;
        let configured = !current.app_id.is_empty()
            && std::env::var("LINJIAN_WX_SECRET").is_ok_and(|v| !v.is_empty());
        let mut tx = self.0.begin().await?;
        // Never lock posts after schools: content writers already own the post lock.
        let school:Option<Uuid>=sqlx::query_scalar("SELECT s.id FROM schools s JOIN school_source_bindings b ON b.school_id=s.id WHERE s.active AND b.active AND EXISTS(SELECT 1 FROM forum_outbox o JOIN forum_publication_preparations p ON p.event_id=o.id JOIN forum_publication_snapshots snap ON snap.id=p.snapshot_id WHERE o.school_id=s.id AND o.connection_version IS NOT NULL AND o.request_body IS NULL AND o.delivery_state='preparing_content' AND ((p.state IN ('pending','retry_wait') AND p.next_attempt_at<=now()) OR (p.state='generating' AND p.lease_until<now()) OR (p.state='blocked' AND p.last_error_code='miniapp_not_configured' AND $1 AND p.next_attempt_at<=now()))) ORDER BY (SELECT min(p.next_attempt_at) FROM forum_publication_preparations p JOIN forum_outbox o ON o.id=p.event_id WHERE o.school_id=s.id AND o.delivery_state='preparing_content'),s.id FOR UPDATE OF s SKIP LOCKED LIMIT 1")
            .bind(configured).fetch_optional(&mut *tx).await?;
        let Some(school) = school else {
            return Ok(None);
        };
        let row=sqlx::query("SELECT o.id,o.post_id,snap.title,snap.link_policy,p.resolved_policy,p.attempts FROM forum_outbox o JOIN forum_publication_preparations p ON p.event_id=o.id JOIN forum_publication_snapshots snap ON snap.id=p.snapshot_id WHERE o.school_id=$1 AND o.connection_version IS NOT NULL AND o.request_body IS NULL AND o.delivery_state='preparing_content' AND ((p.state IN ('pending','retry_wait') AND p.next_attempt_at<=now()) OR (p.state='generating' AND p.lease_until<now()) OR (p.state='blocked' AND p.last_error_code='miniapp_not_configured' AND $2 AND p.next_attempt_at<=now())) ORDER BY p.next_attempt_at,o.created_at,o.id FOR UPDATE OF o,p SKIP LOCKED LIMIT 1")
            .bind(school).bind(configured).fetch_optional(&mut *tx).await?;
        let Some(row) = row else { return Ok(None) };
        let id: Uuid = row.get("id");
        let valid: bool = sqlx::query_scalar("SELECT forum_publication_is_current($1)")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        if !valid {
            sqlx::query("UPDATE forum_outbox SET delivery_state='superseded' WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(None);
        }
        let mut policy: Policy = serde_json::from_value(
            row.get::<Option<Value>, _>("resolved_policy")
                .unwrap_or(row.get("link_policy")),
        )
        .map_err(|_| Error::Conflict)?;
        // A missing AppID at approval is unresolved, not permission to switch a known AppID.
        if policy.app_id.is_empty() && !current.app_id.is_empty() {
            policy = Policy::parse(
                current.app_id,
                policy.link_type,
                &policy.permanent.to_string(),
                &policy.days.to_string(),
                &policy.min_remaining.to_string(),
            )?;
        }
        let token = Uuid::new_v4();
        sqlx::query("UPDATE forum_publication_preparations SET state='generating',lease_token=$2,lease_until=now()+interval '90 seconds',attempts=attempts+1,revision=revision+1,resolved_policy=$3 WHERE event_id=$1")
            .bind(id).bind(token).bind(if policy.app_id.is_empty(){None}else{Some(json!(policy))}).execute(&mut *tx).await?;
        let work = Preparation {
            id,
            token,
            school,
            post: row.get("post_id"),
            title: row.get("title"),
            policy,
            attempts: row.get::<i32, _>("attempts") + 1,
        };
        tx.commit().await?;
        Ok(Some(work))
    }
    async fn finish(
        &self,
        w: &Preparation,
        result: std::result::Result<Link, Failure>,
    ) -> Result<()> {
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
            .bind(w.school)
            .fetch_one(&mut *tx)
            .await?;
        let owned:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum_publication_preparations p JOIN forum_outbox o ON o.id=p.event_id WHERE p.event_id=$1 AND p.lease_token=$2 AND p.lease_until>now() AND p.state='generating' AND o.request_body IS NULL AND o.delivery_state<>'superseded')")
            .bind(w.id).bind(w.token).fetch_one(&mut *tx).await?;
        if !owned {
            return Ok(());
        }
        match result {
            Ok(link) => {
                if link.value.is_empty()
                    || link.value.chars().count() > 4096
                    || link.value.chars().any(|c| c.is_control())
                {
                    return Err(Error::Invalid("小程序链接响应无效".into()));
                }
                let cache_key = w.policy.cache_key(w.school, w.post, &w.title);
                let existing:Option<Uuid>=sqlx::query_scalar("SELECT id FROM miniapp_link_assets WHERE cache_key=$1 AND link=$2 AND (expires_at IS NULL OR expires_at>now()+make_interval(secs=>$3)) ORDER BY generated_at DESC LIMIT 1")
                    .bind(&cache_key).bind(&link.value).bind(w.policy.min_remaining as f64).fetch_optional(&mut *tx).await?;
                let asset = existing.unwrap_or_else(Uuid::now_v7);
                if existing.is_none() {
                    sqlx::query("INSERT INTO miniapp_link_assets(id,cache_key,app_id,policy,page_path,canonical_query,page_title,link_type,permanent,link,generated_at,expires_at) VALUES($1,$2,$3,$4,'pages/index/detail',$5,$6,$7,$8,$9,$10,$11)")
                        .bind(asset).bind(&cache_key).bind(&w.policy.app_id).bind(json!(w.policy)).bind(format!("school={}&id={}",w.school,w.post)).bind(w.title.chars().take(20).collect::<String>()).bind(&w.policy.link_type).bind(w.policy.permanent).bind(&link.value).bind(link.generated_at).bind(link.expires_at).execute(&mut *tx).await?;
                }
                sqlx::query("SELECT forum_finalize_prepared_source($1,$2,$3)")
                    .bind(w.id)
                    .bind(w.token)
                    .bind(asset)
                    .execute(&mut *tx)
                    .await?;
            }
            Err(error) => {
                let retry =
                    error.retry && (w.attempts < 10 || error.code == "daily_quota_exceeded");
                let delay = crate::outbox::domain::retry_seconds(w.attempts - 1).max(error.delay)
                    + (Uuid::new_v4().as_u128() % 5) as i64;
                sqlx::query("UPDATE forum_publication_preparations SET state=$3,last_error_code=$4,next_attempt_at=now()+make_interval(secs=>$5),lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=$1 AND lease_token=$2")
                    .bind(w.id).bind(w.token).bind(if retry{"retry_wait"}else{"blocked"}).bind(error.code).bind(delay as f64).execute(&mut *tx).await?;
            }
        }
        sqlx::query("DELETE FROM miniapp_link_leases WHERE cache_key=$1 AND lease_token=$2")
            .bind(w.policy.cache_key(w.school, w.post, &w.title))
            .bind(w.token)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
