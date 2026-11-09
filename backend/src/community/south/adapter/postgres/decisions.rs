use crate::{
    business::{Error, Result, uuid},
    community::domain::{
        moderation::{AuditMode, DecisionSource},
        review::digest,
    },
    content_publication::domain::{self, Policy},
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) struct PolicyDecision {
    pub mode: AuditMode,
    pub revision: i64,
}
pub(super) async fn policy(
    tx: &mut Transaction<'_, Postgres>,
    school: Uuid,
    comment: bool,
) -> Result<PolicyDecision> {
    let row = sqlx::query("SELECT config,revision FROM schools WHERE id=$1 AND active")
        .bind(school)
        .fetch_one(&mut **tx)
        .await?;
    Ok(PolicyDecision {
        mode: AuditMode::from_config(
            &row.get("config"),
            if comment {
                "comment_audit_mode"
            } else {
                "post_audit_mode"
            },
        ),
        revision: row.get("revision"),
    })
}

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    current: &Value,
    comment: bool,
    actor: Option<Uuid>,
    decision: &str,
    reason: &str,
    policy_revision: Option<i64>,
) -> Result<Uuid> {
    let id = Uuid::now_v7();
    let source = if actor.is_some() {
        DecisionSource::Human
    } else {
        DecisionSource::Automatic
    };
    let version = current[if comment {
        "revision"
    } else {
        "content_revision"
    }]
    .as_i64()
    .ok_or(Error::Conflict)?;
    let fingerprint = if comment {
        crate::business::hash(&current["content"].to_string())
    } else {
        digest(current)?
    };
    sqlx::query("INSERT INTO moderation_events(id,school_id,actor_id,object_id,object_kind,decision,reason,decision_source,content_revision,policy_revision,policy_mode,review_digest) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
        .bind(id).bind(uuid(current,"school_id")?).bind(actor).bind(uuid(current,"id")?)
        .bind(if comment {"comment"} else {"post"}).bind(decision).bind(reason).bind(source.as_str())
        .bind(version).bind(policy_revision).bind(policy_revision.map(|_| "auto")).bind(fingerprint)
        .execute(&mut **tx).await?;
    Ok(id)
}

pub(super) async fn snapshot(
    tx: &mut Transaction<'_, Postgres>,
    current: &Value,
    event: Uuid,
    actor: Option<Uuid>,
    policy_revision: Option<i64>,
) -> Result<Option<Uuid>> {
    if current["source_projection"] != "miniapp_summary_v1" {
        return Ok(None);
    }
    let preview = domain::preview(current)?;
    let link_policy = Policy::from_env()?;
    let id = Uuid::now_v7();
    let revision = current["revision"].as_i64().ok_or(Error::Conflict)?;
    let content_digest = crate::business::hash(
        &json!([
            current["title"],
            current["content"],
            current["images"],
            current["tags"]
        ])
        .to_string(),
    );
    sqlx::query("INSERT INTO forum_publication_snapshots(id,school_id,post_id,author_id,content_revision,approved_post_revision,moderation_event_id,moderator_id,title,content,images,tags,content_digest,summary,summary_algorithm,preview_digest,link_policy,decision_source,policy_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)")
        .bind(id).bind(uuid(current,"school_id")?).bind(uuid(current,"id")?).bind(uuid(current,"author_id")?)
        .bind(preview.content_revision).bind(revision+1).bind(event).bind(actor)
        .bind(current["title"].as_str().unwrap_or("")).bind(current["content"].as_str().unwrap_or(""))
        .bind(&current["images"]).bind(&current["tags"]).bind(content_digest).bind(preview.summary)
        .bind(domain::ALGORITHM).bind(preview.preview_digest).bind(serde_json::to_value(link_policy).map_err(|_|Error::Unavailable)?)
        .bind(if actor.is_some() {"human"} else {"automatic"}).bind(policy_revision)
        .execute(&mut **tx).await?;
    Ok(Some(id))
}

pub(super) async fn automatically_approve(
    tx: &mut Transaction<'_, Postgres>,
    current: Value,
    policy: PolicyDecision,
) -> Result<Value> {
    if policy.mode != AuditMode::Auto {
        return Ok(current);
    }
    let event = record(
        tx,
        &current,
        false,
        None,
        "approved",
        "学校自动审核策略",
        Some(policy.revision),
    )
    .await?;
    let snapshot = snapshot(tx, &current, event, None, Some(policy.revision)).await?;
    let item = sqlx::query_scalar("UPDATE posts SET audit_status='approved',approved_snapshot_id=$2,revision=revision+1 WHERE id=$1 RETURNING to_jsonb(posts)")
        .bind(uuid(&current,"id")?).bind(snapshot).fetch_one(&mut **tx).await?;
    // The decision actor is the system policy; the posting user remains the author.
    sqlx::query("INSERT INTO audit_events(id,school_id,actor_id,action,object_id,detail,request_id) VALUES($1,$2,NULL,'moderation.automatic',$3,$4,'system-policy')")
        .bind(Uuid::now_v7()).bind(uuid(&current,"school_id")?).bind(uuid(&current,"id")?)
        .bind(json!({"decision_source":"automatic","policy_revision":policy.revision,"content_revision":current["content_revision"]}))
        .execute(&mut **tx).await?;
    Ok(item)
}
