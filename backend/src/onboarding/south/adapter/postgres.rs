use super::result_cipher;
use crate::{
    business::{Error, Result, SessionKind, hash, require, secret},
    onboarding::{domain::Command, south::port::Store},
};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
impl Store for Repository {
    async fn start(&self, app: &str, openid: &str) -> Result<Value> {
        let ticket = secret();
        let mut transaction = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73109))")
            .bind(json!([app, openid]).to_string())
            .execute(&mut *transaction)
            .await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_onboarding_attempts WHERE app_id=$1 AND openid=$2 AND expires_at>now() AND NOT cancelled").bind(app).bind(openid).fetch_one(&mut *transaction).await?;
        if count >= 10 {
            return Err(Error::RateLimited);
        }
        sqlx::query(
            "INSERT INTO miniapp_onboarding_attempts(token_hash,app_id,openid) VALUES($1,$2,$3)",
        )
        .bind(hash(&ticket))
        .bind(app)
        .bind(openid)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.command(
            "status",
            Command {
                ticket,
                acceptances: vec![],
                school_id: None,
                request_key: None,
            },
        )
        .await
    }
    async fn schools(&self, query: &str, page: i64) -> Result<Vec<Value>> {
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name,'short_name',short_name,'province',province,'city',city) FROM schools WHERE active AND ($1='' OR strpos(lower(name||' '||short_name||' '||province||' '||city),lower($1))>0) ORDER BY name,id LIMIT 20 OFFSET $2").bind(query).bind((page-1)*20).fetch_all(&self.0).await?)
    }
    async fn command(&self, action: &str, input: Command) -> Result<Value> {
        let mut transaction = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *transaction)
            .await?;
        let identity = sqlx::query("SELECT app_id,openid FROM miniapp_onboarding_attempts WHERE token_hash=$1 AND NOT cancelled AND expires_at>now()").bind(hash(&input.ticket)).fetch_optional(&mut *transaction).await?.ok_or(Error::Unauthorized)?;
        let app: String = identity.get("app_id");
        let openid: String = identity.get("openid");
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73109))")
            .bind(json!([app, openid]).to_string())
            .execute(&mut *transaction)
            .await?;
        let mut account: Option<Uuid> = sqlx::query_scalar(
            "SELECT account_id FROM miniapp_identities WHERE app_id=$1 AND openid=$2",
        )
        .bind(&app)
        .bind(&openid)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(account) = account {
            let (_, status, _) = crate::miniapp_users::south::adapter::postgres::lock_user(
                &mut transaction,
                account,
            )
            .await?;
            if status != "normal" {
                return Err(Error::MiniappBanned);
            }
            sqlx::query("SELECT id FROM accounts WHERE id=$1 AND active AND account_kind='ordinary' FOR UPDATE").bind(account).fetch_optional(&mut *transaction).await?.ok_or(Error::Unauthorized)?;
        }
        let attempt = sqlx::query("SELECT *,result_expires_at>now() AS recoverable FROM miniapp_onboarding_attempts WHERE token_hash=$1 AND NOT cancelled AND expires_at>now() FOR UPDATE").bind(hash(&input.ticket)).fetch_optional(&mut *transaction).await?.ok_or(Error::Unauthorized)?;
        if action == "cancel" {
            sqlx::query("UPDATE miniapp_onboarding_attempts SET cancelled=true,result=NULL WHERE token_hash=$1").bind(hash(&input.ticket)).execute(&mut *transaction).await?;
            transaction.commit().await?;
            return Ok(json!({"state":"cancelled"}));
        }
        if attempt.get::<bool, _>("completed") {
            if action == "complete"
                && input.school_id.is_some()
                && input.school_id != attempt.get::<Option<Uuid>, _>("completion_school")
            {
                return Err(Error::Conflict);
            }
            if action == "complete"
                && attempt.get::<Option<String>, _>("completion_key")
                    != input.request_key.map(|key| key.to_string())
            {
                return Err(Error::Conflict);
            }
            if !attempt
                .get::<Option<bool>, _>("recoverable")
                .unwrap_or(false)
            {
                return Err(Error::Unauthorized);
            }
            let value =
                result_cipher::decrypt(&input.ticket, &attempt.get::<Vec<u8>, _>("result"))?;
            let session_valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions s JOIN accounts a ON a.id=s.account_id JOIN schools school ON school.id=a.bound_school_id AND school.active JOIN school_memberships m ON m.account_id=a.id AND m.school_id=school.id AND m.role<>'revoked' WHERE s.token_hash=$1 AND s.kind='miniapp' AND s.expires_at>now() AND s.last_seen_at>now()-interval '30 minutes' AND a.bound_school_id=$2)").bind(hash(value["token"].as_str().ok_or(Error::Unauthorized)?)).bind(attempt.get::<Option<Uuid>,_>("completion_school")).fetch_one(&mut *transaction).await?;
            if !session_valid {
                return Err(Error::Unauthorized);
            }
            return Ok(value);
        }
        sqlx::query("SELECT id FROM legal_document_catalog FOR SHARE")
            .execute(&mut *transaction)
            .await?;
        let documents: Vec<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',kind,'kind',kind,'revision',revision,'updated_at',updated_at) FROM legal_documents WHERE content IS NOT NULL ORDER BY kind DESC").fetch_all(&mut *transaction).await?;
        let previous: bool = if let Some(account) = account {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM miniapp_onboarding WHERE account_id=$1 AND completed_at IS NOT NULL)").bind(account).fetch_one(&mut *transaction).await?
        } else {
            false
        };
        if action == "confirm" && !previous {
            for document in &documents {
                require(
                    input.acceptances.iter().any(|value| {
                        value.kind == document["kind"].as_str().unwrap_or("")
                            && Some(value.revision) == document["revision"].as_i64()
                    }),
                    "请重新阅读并勾选当前版本的全部协议",
                )?;
            }
            let id = crate::miniapp_users::south::adapter::postgres::Repository::register_in(
                &mut transaction,
                &app,
                &openid,
                crate::miniapp_users::south::adapter::registration_random::RegistrationRandom::new,
            )
            .await?;
            account = Some(id);
            for document in &documents {
                sqlx::query("INSERT INTO miniapp_legal_acceptances(account_id,kind,revision) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(id).bind(document["kind"].as_str()).bind(document["revision"].as_i64()).execute(&mut *transaction).await?;
            }
            sqlx::query(
                "INSERT INTO miniapp_onboarding(account_id) VALUES($1) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .execute(&mut *transaction)
            .await?;
            sqlx::query("UPDATE miniapp_onboarding_attempts SET account_id=$2 WHERE token_hash=$1")
                .bind(hash(&input.ticket))
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        let mut accepted = Vec::<Value>::new();
        let mut bound: Option<Uuid> = None;
        if let Some(account) = account {
            bound = sqlx::query_scalar("SELECT bound_school_id FROM accounts WHERE id=$1")
                .bind(account)
                .fetch_one(&mut *transaction)
                .await?;
            accepted = sqlx::query_scalar("SELECT jsonb_build_object('kind',kind,'revision',revision) FROM miniapp_legal_acceptances WHERE account_id=$1").bind(account).fetch_all(&mut *transaction).await?;
        }
        let covered = previous
            || documents.iter().all(|document| {
                accepted.iter().any(|value| {
                    value["kind"] == document["kind"] && value["revision"] == document["revision"]
                })
            });
        if action == "complete" && covered {
            let account = account.ok_or(Error::Unauthorized)?;
            let school = bound
                .or(input.school_id)
                .ok_or_else(|| Error::Invalid("请选择学校".into()))?;
            if bound.is_some() && input.school_id.is_some_and(|value| value != school) {
                return Err(Error::Forbidden);
            }
            sqlx::query("SELECT id FROM schools WHERE id=$1 AND active FOR SHARE")
                .bind(school)
                .fetch_optional(&mut *transaction)
                .await?
                .ok_or(Error::SchoolUnavailable)?;
            let role: Option<String> = sqlx::query_scalar("SELECT role FROM school_memberships WHERE school_id=$1 AND account_id=$2 FOR SHARE").bind(school).bind(account).fetch_optional(&mut *transaction).await?;
            if role.as_deref() == Some("revoked") {
                return Err(Error::Forbidden);
            }
            sqlx::query("UPDATE accounts SET bound_school_id=$2 WHERE id=$1")
                .bind(account)
                .bind(school)
                .execute(&mut *transaction)
                .await?;
            sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'member') ON CONFLICT(school_id,account_id) DO NOTHING").bind(school).bind(account).execute(&mut *transaction).await?;
            sqlx::query("UPDATE miniapp_users SET revision=revision+1,updated_at=now() WHERE account_id=$1 AND $2").bind(account).bind(bound.is_none()).execute(&mut *transaction).await?;
            sqlx::query("INSERT INTO miniapp_onboarding(account_id,completed_at) VALUES($1,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=coalesce(miniapp_onboarding.completed_at,excluded.completed_at)").bind(account).execute(&mut *transaction).await?;
            let token = crate::identity::south::adapter::postgres::Repository::session_in(
                &mut transaction,
                account,
                SessionKind::Miniapp,
                None,
            )
            .await?;
            let user: Value = sqlx::query_scalar("SELECT jsonb_build_object('id',a.id,'username',a.username,'display_name',u.nickname,'platform_admin',false,'bound_school_id',a.bound_school_id,'grants',jsonb_build_array(jsonb_build_object('school_id',a.bound_school_id,'role','member'))) FROM accounts a JOIN miniapp_users u ON u.account_id=a.id WHERE a.id=$1").bind(account).fetch_one(&mut *transaction).await?;
            let result = json!({"state":"ready","token":token,"user":user});
            let key = input
                .request_key
                .ok_or_else(|| Error::Invalid("缺少提交标识".into()))?;
            sqlx::query("UPDATE miniapp_onboarding_attempts SET completed=true,completion_key=$2,result=$3,result_expires_at=now()+interval '2 minutes',completion_school=$4 WHERE token_hash=$1").bind(hash(&input.ticket)).bind(key.to_string()).bind(result_cipher::encrypt(&input.ticket,&result)?).bind(school).execute(&mut *transaction).await?;
            sqlx::query("INSERT INTO audit_events(id,actor_id,school_id,action,detail) VALUES($1,$2,$3,'onboarding.complete','{}')").bind(Uuid::now_v7()).bind(account).bind(school).execute(&mut *transaction).await?;
            transaction.commit().await?;
            return Ok(result);
        }
        let result = json!({"state":if covered {"school_required"} else {"consent_required"},"ticket":input.ticket,"documents":documents,"accepted":accepted,"first_login":!previous,"school_id":bound,"expires_at":attempt.get::<chrono::DateTime<chrono::Utc>,_>("expires_at")});
        transaction.commit().await?;
        Ok(result)
    }
}
impl Repository {
    pub async fn cleanup(&self) -> Result<()> {
        sqlx::query("UPDATE miniapp_onboarding_attempts SET result=NULL WHERE result IS NOT NULL AND result_expires_at<now()").execute(&self.0).await?;
        sqlx::query("DELETE FROM miniapp_onboarding_attempts WHERE expires_at<now() OR cancelled")
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
