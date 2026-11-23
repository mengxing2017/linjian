use super::{Repository, audit, guard_session, lock_user};
use crate::{
    business::{Actor, Error, Result, hash},
    miniapp_users::{
        domain::phone::{BindPhoneInput, PhoneBinding, VerifiedPhone},
        south::port::{PhoneBindingStore, PhoneReservation},
    },
};
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

async fn lock(tx: &mut Transaction<'_, Postgres>, actor: &Actor) -> Result<()> {
    if actor.is_admin_browser() {
        return Err(Error::Forbidden);
    }
    let (_, status, _) = lock_user(tx, actor.id).await?;
    if status != "normal" {
        return Err(Error::MiniappBanned);
    }
    guard_session(tx, actor).await
}

async fn read(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<PhoneBinding> {
    let row = sqlx::query("SELECT status,phone_last4,verified_at,revision FROM miniapp_phone_bindings WHERE account_id=$1")
        .bind(id).fetch_optional(&mut **tx).await?;
    Ok(match row {
        Some(row) => PhoneBinding {
            id,
            status: row.get("status"),
            masked_phone: row
                .get::<Option<String>, _>("phone_last4")
                .map(|last| format!("*******{last}")),
            verified_at: row.get("verified_at"),
            revision: row.get("revision"),
        },
        None => PhoneBinding {
            id,
            status: "unverified".into(),
            masked_phone: None,
            verified_at: None,
            revision: 0,
        },
    })
}

impl PhoneBindingStore for Repository {
    async fn phone_binding(&self, actor: &Actor) -> Result<PhoneBinding> {
        let mut tx = self.0.begin().await?;
        lock(&mut tx, actor).await?;
        let value = read(&mut tx, actor.id).await?;
        tx.commit().await?;
        Ok(value)
    }

    async fn reserve_phone(
        &self,
        actor: &Actor,
        app_id: &str,
        input: &BindPhoneInput,
    ) -> Result<PhoneReservation> {
        let fingerprint =
            hash(&json!([input.code, input.revision, input.notice_version, app_id]).to_string());
        let mut tx = self.0.begin().await?;
        lock(&mut tx, actor).await?;
        let existing = sqlx::query("SELECT fingerprint,status FROM miniapp_phone_binding_requests WHERE account_id=$1 AND request_id=$2")
            .bind(actor.id).bind(input.request_id).fetch_optional(&mut *tx).await?;
        if let Some(row) = existing {
            if row.get::<String, _>("fingerprint") != fingerprint {
                return Err(Error::Phone("phone_binding_conflict"));
            }
            if row.get::<String, _>("status") != "completed" {
                return Err(Error::Phone("phone_binding_result_uncertain"));
            }
            let value = read(&mut tx, actor.id).await?;
            tx.commit().await?;
            return Ok(PhoneReservation::Completed(value));
        }
        if read(&mut tx, actor.id).await?.revision != input.revision {
            return Err(Error::Phone("phone_binding_conflict"));
        }
        let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_phone_binding_requests WHERE account_id=$1 AND created_at>now()-interval '30 minutes'")
            .bind(actor.id).fetch_one(&mut *tx).await?;
        if recent >= 5 {
            return Err(Error::Phone("phone_binding_rate_limited"));
        }
        let openid: String = sqlx::query_scalar(
            "SELECT openid FROM miniapp_identities WHERE account_id=$1 AND app_id=$2",
        )
        .bind(actor.id)
        .bind(app_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Phone("phone_provider_unavailable"))?;
        sqlx::query("INSERT INTO miniapp_phone_binding_requests(account_id,request_id,fingerprint,status) VALUES($1,$2,$3,'processing')")
            .bind(actor.id).bind(input.request_id).bind(fingerprint).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(PhoneReservation::Reserved { openid })
    }

    async fn finish_phone(
        &self,
        actor: &Actor,
        input: &BindPhoneInput,
        phone: &VerifiedPhone,
        ciphertext: &[u8],
        key_version: &str,
    ) -> Result<PhoneBinding> {
        let mut tx = self.0.begin().await?;
        lock(&mut tx, actor).await?;
        if read(&mut tx, actor.id).await?.revision != input.revision {
            return Err(Error::Phone("phone_binding_conflict"));
        }
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM miniapp_phone_binding_requests WHERE account_id=$1 AND request_id=$2 AND status='processing' AND created_at>now()-interval '2 minutes')")
            .bind(actor.id).bind(input.request_id).fetch_one(&mut *tx).await?;
        if !active {
            return Err(Error::Phone("phone_binding_result_uncertain"));
        }
        sqlx::query("INSERT INTO miniapp_phone_bindings(account_id,status,phone_ciphertext,key_version,country_code,phone_last4,app_id,verified_at,notice_version,revision) VALUES($1,'verified',$2,$3,'86',$4,$5,$6,$7,1) ON CONFLICT(account_id) DO UPDATE SET status='verified',phone_ciphertext=$2,key_version=$3,country_code='86',phone_last4=$4,app_id=$5,verified_at=$6,notice_version=$7,revision=miniapp_phone_bindings.revision+1,updated_at=now()")
            .bind(actor.id).bind(ciphertext).bind(key_version).bind(&phone.last4).bind(&phone.app_id).bind(phone.verified_at).bind(&input.notice_version).execute(&mut *tx).await?;
        sqlx::query("UPDATE miniapp_phone_binding_requests SET status=CASE WHEN request_id=$2 THEN 'completed' ELSE 'uncertain' END WHERE account_id=$1 AND status='processing'")
            .bind(actor.id).bind(input.request_id).execute(&mut *tx).await?;
        let value = read(&mut tx, actor.id).await?;
        audit(&mut tx, actor, actor.bound_school_id, actor.id, "miniapp_user.phone_verified", json!({"revision":value.revision,"provider":"wechat","notice_version":input.notice_version})).await?;
        tx.commit().await?;
        Ok(value)
    }

    async fn fail_phone(&self, actor: &Actor, request: Uuid) -> Result<()> {
        sqlx::query("UPDATE miniapp_phone_binding_requests SET status='uncertain' WHERE account_id=$1 AND request_id=$2 AND status='processing'")
            .bind(actor.id).bind(request).execute(&self.0).await?;
        Ok(())
    }

    async fn revoke_phone(&self, actor: &Actor, revision: i64) -> Result<PhoneBinding> {
        let mut tx = self.0.begin().await?;
        lock(&mut tx, actor).await?;
        if read(&mut tx, actor.id).await?.revision != revision {
            return Err(Error::Phone("phone_binding_conflict"));
        }
        sqlx::query("INSERT INTO miniapp_phone_bindings(account_id,status,revision) VALUES($1,'revoked',1) ON CONFLICT(account_id) DO UPDATE SET status='revoked',phone_ciphertext=NULL,key_version=NULL,country_code=NULL,phone_last4=NULL,app_id=NULL,verified_at=NULL,notice_version=NULL,revision=miniapp_phone_bindings.revision+1,updated_at=now()")
            .bind(actor.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE miniapp_phone_binding_requests SET status='uncertain' WHERE account_id=$1 AND status='processing'")
            .bind(actor.id).execute(&mut *tx).await?;
        let value = read(&mut tx, actor.id).await?;
        audit(
            &mut tx,
            actor,
            actor.bound_school_id,
            actor.id,
            "miniapp_user.phone_revoked",
            json!({"revision":value.revision}),
        )
        .await?;
        tx.commit().await?;
        Ok(value)
    }
}

pub async fn require_verified(tx: &mut Transaction<'_, Postgres>, actor: &Actor) -> Result<()> {
    if actor.is_admin_browser() {
        return Ok(());
    }
    let verified: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM miniapp_phone_bindings WHERE account_id=$1 AND status='verified')")
        .bind(actor.id).fetch_one(&mut **tx).await?;
    crate::miniapp_users::domain::phone::require_phone(actor, verified)
}
