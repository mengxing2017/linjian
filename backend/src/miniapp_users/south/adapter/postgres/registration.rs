use super::Repository;
use crate::{
    business::{Actor, Error, Result, hash},
    miniapp_users::{
        domain::registration::{
            self, NICKNAME_ATTEMPTS, RegistrationSettings, RegistrationSettingsInput,
        },
        south::port::{NicknameRandom, RegistrationSettingsStore},
    },
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

const SETTINGS_VIEW: &str = "SELECT jsonb_build_object('nickname_prefix',s.nickname_prefix,'revision',s.revision,'updated_at',s.updated_at,'updated_by_name',a.display_name) FROM miniapp_registration_settings s LEFT JOIN accounts a ON a.id=s.updated_by WHERE s.id=1";
pub const NICKNAME_LOCK: i64 = 78_234_610_247;

impl Repository {
    pub async fn register_with_random<Random, Factory>(
        &self,
        app: &str,
        openid: &str,
        factory: Factory,
    ) -> Result<Uuid>
    where
        Random: NicknameRandom,
        Factory: FnOnce() -> Result<Random> + Send,
    {
        let mut tx = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await?;
        let id = Self::register_in(&mut tx, app, openid, factory).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn register_in<Random, Factory>(
        tx: &mut sqlx::PgConnection,
        app: &str,
        openid: &str,
        factory: Factory,
    ) -> Result<Uuid>
    where
        Random: NicknameRandom,
        Factory: FnOnce() -> Result<Random> + Send,
    {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73109))")
            .bind(json!([app, openid]).to_string())
            .execute(&mut *tx)
            .await?;
        if let Some(id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT account_id FROM miniapp_identities WHERE app_id=$1 AND openid=$2",
        )
        .bind(app)
        .bind(openid)
        .fetch_optional(&mut *tx)
        .await?
        {
            return Ok(id);
        }
        let lock_started = std::time::Instant::now();
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(NICKNAME_LOCK)
            .execute(&mut *tx)
            .await?;
        tracing::debug!(
            wait_millis = lock_started.elapsed().as_millis(),
            "registration nickname lock acquired"
        );
        let prefix: String = sqlx::query_scalar(
            "SELECT nickname_prefix FROM miniapp_registration_settings WHERE id=1",
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Unavailable)?;
        let prefix = registration::prefix(&prefix)?;
        let mut random = factory()?;
        let mut nickname = None;
        for attempt in 0..NICKNAME_ATTEMPTS {
            let value = registration::candidate(&prefix, random.next_number()?)?;
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM miniapp_users WHERE nickname=$1)")
                    .bind(&value)
                    .fetch_one(&mut *tx)
                    .await?;
            if !exists {
                nickname = Some(value);
                break;
            }
            tracing::info!(attempt, "registration nickname collision");
        }
        let nickname = nickname.ok_or_else(|| {
            tracing::warn!("registration nickname attempts exhausted");
            Error::Unavailable
        })?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,username,display_name) VALUES($1,$2,$3)")
            .bind(id)
            .bind(format!(
                "miniapp_{}",
                hash(&json!([app, openid]).to_string())
            ))
            .bind(&nickname)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,$2)")
            .bind(id)
            .bind(&nickname)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO miniapp_identities(app_id,openid,account_id) VALUES($1,$2,$3)")
            .bind(app)
            .bind(openid)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        Ok(id)
    }
}

impl RegistrationSettingsStore for Repository {
    async fn registration_settings(&self) -> Result<RegistrationSettings> {
        let value: serde_json::Value = sqlx::query_scalar(SETTINGS_VIEW)
            .fetch_optional(&self.0)
            .await?
            .ok_or(Error::Unavailable)?;
        serde_json::from_value(value).map_err(|_| Error::Unavailable)
    }

    async fn update_registration_settings(
        &self,
        actor: &Actor,
        input: RegistrationSettingsInput,
    ) -> Result<RegistrationSettings> {
        let prefix = input.validate()?;
        let mut tx = self.0.begin().await?;
        crate::schools::south::adapter::postgres::preview::platform(&mut tx, actor).await?;
        let row = sqlx::query("SELECT nickname_prefix,revision FROM miniapp_registration_settings WHERE id=1 FOR UPDATE")
            .fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
        let revision: i64 = row.get("revision");
        if revision != input.revision {
            return Err(Error::Conflict);
        }
        let previous: String = row.get("nickname_prefix");
        sqlx::query("UPDATE miniapp_registration_settings SET nickname_prefix=$1,revision=revision+1,updated_at=now(),updated_by=$2 WHERE id=1")
            .bind(&prefix).bind(actor.id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO audit_events(id,actor_id,action,detail,request_id) VALUES($1,$2,'miniapp_user.registration_settings.update',$3,$4)")
            .bind(Uuid::now_v7()).bind(actor.id)
            .bind(json!({"from":{"nickname_prefix":previous,"revision":revision},"to":{"nickname_prefix":prefix,"revision":revision+1}}))
            .bind(&actor.request_id).execute(&mut *tx).await?;
        let value: serde_json::Value = sqlx::query_scalar(SETTINGS_VIEW)
            .fetch_one(&mut *tx)
            .await?;
        let result = serde_json::from_value(value).map_err(|_| Error::Unavailable)?;
        tx.commit().await?;
        Ok(result)
    }
}
