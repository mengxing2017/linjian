use crate::business::{Actor, Error, Result};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn lock_accounts(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    target: Uuid,
) -> Result<()> {
    let mut ids = vec![actor.id, target];
    ids.sort();
    ids.dedup();
    for id in ids {
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::NotFound)?;
    }
    Ok(())
}

pub async fn platform_session(tx: &mut Transaction<'_, Postgres>, actor: &Actor) -> Result<()> {
    actor.platform()?;
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts a JOIN sessions se ON se.account_id=a.id WHERE a.id=$1 AND a.active AND a.platform_admin AND se.token_hash=$2 AND se.kind='admin_browser' AND se.expires_at>now() AND se.last_seen_at>now()-interval '30 minutes')")
        .bind(actor.id).bind(&actor.session_hash).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

pub async fn validate_target(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
    old: Option<Uuid>,
    target: Uuid,
) -> Result<()> {
    let active: bool = sqlx::query_scalar("SELECT active FROM schools WHERE id=$1 FOR SHARE")
        .bind(target)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::Policy("school_change_target_unavailable"))?;
    if !active {
        return Err(Error::Policy("school_change_target_unavailable"));
    }
    let conflict: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_memberships WHERE account_id=$1 AND school_id=ANY($2) AND role IN ('admin','operator'))")
        .bind(account).bind(old.into_iter().chain([target]).collect::<Vec<_>>()).fetch_one(&mut **tx).await?;
    if conflict {
        return Err(Error::Policy("school_change_role_conflict"));
    }
    Ok(())
}

pub async fn change(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    account: Uuid,
    old: Option<Uuid>,
    target: Uuid,
) -> Result<()> {
    validate_target(tx, account, old, target).await?;
    if old == Some(target) {
        return Ok(());
    }
    if let Some(old) = old {
        sqlx::query("UPDATE school_memberships SET role='revoked' WHERE school_id=$1 AND account_id=$2 AND role='member'").bind(old).bind(account).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'member') ON CONFLICT(school_id,account_id) DO UPDATE SET role='member'").bind(target).bind(account).execute(&mut **tx).await?;
    sqlx::query("UPDATE accounts SET bound_school_id=$2 WHERE id=$1")
        .bind(account)
        .bind(target)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "UPDATE miniapp_users SET revision=revision+1,updated_at=now() WHERE account_id=$1",
    )
    .bind(account)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM sessions WHERE account_id=$1 AND kind='miniapp'")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    crate::school_changes::south::adapter::postgres::expire_pending(tx, account).await?;
    let pending: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM miniapp_school_change_requests WHERE account_id=$1 AND status='pending' ORDER BY id FOR UPDATE").bind(account).fetch_all(&mut **tx).await?;
    for request in pending {
        crate::school_changes::south::adapter::postgres::finish(
            tx,
            request,
            "invalidated",
            None,
            Some(actor.id),
            "school_changed",
            None,
        )
        .await?;
    }
    crate::audit::record(
        tx,
        actor,
        old.unwrap_or(Uuid::nil()),
        "miniapp_user.school_change",
        account,
        json!({"from":old,"to":target,"miniapp_sessions_revoked":true}),
    )
    .await?;
    Ok(())
}
