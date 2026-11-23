use crate::{
    business::{Actor, Error, Page, Result},
    message_advertisements::{
        domain::{self, Command, Filters},
        south::port::Store,
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct Repository(pub PgPool);

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    school: Option<Uuid>,
    manage: bool,
) -> Result<()> {
    if !actor.is_admin_browser() {
        return Err(Error::Forbidden);
    }
    if let Some(school) = school {
        crate::schools::south::adapter::postgres::permissions::fence(tx, school, false).await?;
    }
    let platform: bool = sqlx::query_scalar("SELECT a.platform_admin FROM accounts a JOIN sessions s ON s.account_id=a.id WHERE a.id=$1 AND a.active AND s.token_hash=$2 AND s.kind='admin_browser' AND s.expires_at>now() AND s.last_seen_at>now()-interval '30 minutes' FOR SHARE OF a,s")
        .bind(actor.id).bind(&actor.session_hash).fetch_optional(&mut **tx).await?.ok_or(Error::Unauthorized)?;
    match school {
        None => {
            actor.platform()?;
            if !platform {
                return Err(Error::Forbidden);
            }
        }
        Some(school) => {
            let permission = if manage {
                "school.message_advertisement.manage"
            } else {
                "school.message_advertisement.read"
            };
            crate::schools::south::adapter::postgres::permissions::authorize(
                tx, actor, school, permission,
            )
            .await?;
            if manage || !platform {
                sqlx::query("SELECT m.account_id FROM school_memberships m JOIN schools s ON s.id=m.school_id WHERE m.account_id=$1 AND m.school_id=$2 AND m.role='admin' AND s.active FOR SHARE OF m")
                    .bind(actor.id).bind(school).fetch_optional(&mut **tx).await?.ok_or(Error::Forbidden)?;
            }
        }
    }
    Ok(())
}

async fn revision_value(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Value> {
    sqlx::query_scalar("SELECT to_jsonb(r)||jsonb_build_object('version',r.version::text,'number',r.number::text,'config_version',c.version::text,'config_deleted_at',c.deleted_at,'current_revision_id',c.current_revision_id,'school_name',s.name,'decision',(SELECT to_jsonb(d) FROM school_message_advertisement_decisions d WHERE d.revision_id=r.id)) FROM school_message_advertisement_revisions r JOIN school_message_advertisements c ON c.id=r.advertisement_id JOIN schools s ON s.id=r.school_id WHERE r.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)
}

async fn config_value(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Value> {
    let row = sqlx::query("SELECT to_jsonb(c)||jsonb_build_object('version',c.version::text) AS value,current_revision_id FROM school_message_advertisements c WHERE c.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    let mut value: Value = row.get("value");
    value["current_revision"] = match row.get::<Option<Uuid>, _>("current_revision_id") {
        Some(revision) => revision_value(tx, revision).await?,
        None => Value::Null,
    };
    let candidate: Option<Uuid> = sqlx::query_scalar("SELECT id FROM school_message_advertisement_revisions WHERE advertisement_id=$1 AND status IN ('draft','pending')")
        .bind(id).fetch_optional(&mut **tx).await?;
    value["candidate"] = match candidate {
        Some(revision) => revision_value(tx, revision).await?,
        None => Value::Null,
    };
    let usage: Vec<Value> = sqlx::query_scalar("SELECT to_jsonb(u) FROM (SELECT event_id,result,revision_id,base_length,final_length,frozen_at FROM forum_message_advertisement_snapshots WHERE advertisement_id=$1 ORDER BY frozen_at DESC,event_id DESC LIMIT 10) u")
        .bind(id).fetch_all(&mut **tx).await?;
    value["recent_usage"] = json!(usage);
    Ok(value)
}

fn conflict(condition: bool) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Conflict)
    }
}

async fn new_draft(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    school: Uuid,
    advertisement: Uuid,
    content: &str,
) -> Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO school_message_advertisement_revisions(id,advertisement_id,school_id,number,content,created_by) SELECT $1,$2,$3,coalesce(max(number),0)+1,$4,$5 FROM school_message_advertisement_revisions WHERE advertisement_id=$2")
        .bind(id).bind(advertisement).bind(school).bind(content).bind(actor.id).execute(&mut **tx).await?;
    Ok(id)
}

impl Store for Repository {
    async fn current(&self, actor: &Actor, school: Uuid) -> Result<Option<Value>> {
        let mut tx = self.0.begin().await?;
        authorize(&mut tx, actor, Some(school), false).await?;
        sqlx::query("SELECT id FROM schools WHERE id=$1 FOR SHARE")
            .bind(school)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM school_message_advertisements WHERE school_id=$1 AND deleted_at IS NULL").bind(school).fetch_optional(&mut *tx).await?;
        let value = match id {
            Some(id) => Some(config_value(&mut tx, id).await?),
            None => None,
        };
        tx.commit().await?;
        Ok(value)
    }
    async fn history(
        &self,
        actor: &Actor,
        school: Uuid,
        advertisement: Uuid,
        page: &Page,
    ) -> Result<Vec<Value>> {
        let mut tx = self.0.begin().await?;
        authorize(&mut tx, actor, Some(school), false).await?;
        let (limit, offset) = page.fetch_bounds()?;
        sqlx::query("SELECT id FROM school_message_advertisements WHERE id=$1 AND school_id=$2")
            .bind(advertisement)
            .bind(school)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM school_message_advertisement_revisions WHERE advertisement_id=$1 AND school_id=$2 ORDER BY number DESC LIMIT $3 OFFSET $4")
            .bind(advertisement).bind(school).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
        let mut values = Vec::new();
        for id in ids {
            values.push(revision_value(&mut tx, id).await?);
        }
        tx.commit().await?;
        Ok(values)
    }
    async fn detail(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        advertisement: Option<Uuid>,
        revision: Uuid,
    ) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        authorize(&mut tx, actor, school, false).await?;
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_message_advertisement_revisions WHERE id=$1 AND ($2::uuid IS NULL OR school_id=$2) AND ($3::uuid IS NULL OR advertisement_id=$3))")
            .bind(revision).bind(school).bind(advertisement).fetch_one(&mut *tx).await?;
        if !valid {
            return Err(Error::NotFound);
        }
        let mut value = revision_value(&mut tx, revision).await?;
        let id: Uuid = serde_json::from_value(value["advertisement_id"].clone())
            .map_err(|_| Error::Unavailable)?;
        value["configuration"] = config_value(&mut tx, id).await?;
        tx.commit().await?;
        Ok(value)
    }
    async fn list(&self, actor: &Actor, filters: &Filters) -> Result<Vec<Value>> {
        let mut tx = self.0.begin().await?;
        authorize(&mut tx, actor, None, false).await?;
        let (limit, offset) = filters.page.fetch_bounds()?;
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT r.id FROM school_message_advertisement_revisions r WHERE ($1::uuid IS NULL OR r.school_id=$1) AND r.status=$2 AND ($3::timestamptz IS NULL OR coalesce(r.submitted_at,r.created_at) >= $3) AND ($4::timestamptz IS NULL OR coalesce(r.submitted_at,r.created_at) <= $4) ORDER BY coalesce(r.submitted_at,r.created_at) DESC,r.id DESC LIMIT $5 OFFSET $6")
            .bind(filters.school_id).bind(filters.page.status.as_deref().unwrap_or("pending")).bind(filters.created_from).bind(filters.created_to).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
        let mut values = Vec::new();
        for id in ids {
            values.push(revision_value(&mut tx, id).await?);
        }
        tx.commit().await?;
        Ok(values)
    }
    async fn write(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        advertisement: Option<Uuid>,
        revision: Option<Uuid>,
        command: Command,
    ) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        let (school, advertisement) = if command.platform() {
            authorize(&mut tx, actor, None, false).await?;
            let row = if let Some(revision) = revision {
                sqlx::query("SELECT school_id,advertisement_id AS id FROM school_message_advertisement_revisions WHERE id=$1").bind(revision).fetch_optional(&mut *tx).await?
            } else {
                sqlx::query("SELECT school_id,id FROM school_message_advertisements WHERE id=$1").bind(advertisement.ok_or(Error::NotFound)?).fetch_optional(&mut *tx).await?
            }.ok_or(Error::NotFound)?;
            (row.get("school_id"), Some(row.get::<Uuid, _>("id")))
        } else {
            let school = school.ok_or(Error::Forbidden)?;
            authorize(&mut tx, actor, Some(school), true).await?;
            (school, advertisement)
        };
        let school_active: bool =
            sqlx::query_scalar("SELECT active FROM schools WHERE id=$1 FOR UPDATE")
                .bind(school)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(Error::NotFound)?;
        if !school_active && !matches!(&command, Command::Remove(_)) {
            return Err(Error::SchoolUnavailable);
        }
        let before_config_version = command.expected_config_version().map(str::to_owned);
        let advertisement = if let Some(id) = advertisement {
            let row = sqlx::query("SELECT version,deleted_at FROM school_message_advertisements WHERE id=$1 AND school_id=$2 FOR UPDATE")
                .bind(id).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
            conflict(
                row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
                    .is_none(),
            )?;
            conflict(
                command
                    .expected_config_version()
                    .is_some_and(|value| domain::version(value).ok() == Some(row.get("version"))),
            )?;
            id
        } else {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_message_advertisements WHERE school_id=$1 AND deleted_at IS NULL)").bind(school).fetch_one(&mut *tx).await?;
            conflict(!exists && matches!(command, Command::Create(_)))?;
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO school_message_advertisements(id,school_id,created_by) VALUES($1,$2,$3)").bind(id).bind(school).bind(actor.id).execute(&mut *tx).await?;
            id
        };
        let submitting = matches!(&command, Command::Submit(_));
        let removal_reason = match &command {
            Command::Remove(input) => Some(input.reason.clone()),
            _ => None,
        };
        let (decision, reason) = match &command {
            Command::Decide(input) => (Some(input.decision.clone()), Some(input.reason.clone())),
            _ => (None, removal_reason.clone()),
        };
        let before_revision: Option<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',id,'version',version::text,'status',status) FROM school_message_advertisement_revisions WHERE advertisement_id=$1 AND (($2::uuid IS NOT NULL AND id=$2) OR ($2::uuid IS NULL AND status IN ('draft','pending')))")
            .bind(advertisement).bind(revision).fetch_optional(&mut *tx).await?;
        let mut target_revision = revision.or_else(|| {
            before_revision
                .as_ref()
                .and_then(|value| value["id"].as_str())
                .and_then(|value| Uuid::parse_str(value).ok())
        });
        let action = match command {
            Command::Create(input) => {
                target_revision =
                    Some(new_draft(&mut tx, actor, school, advertisement, &input.content).await?);
                "create"
            }
            Command::Draft(input) => {
                let candidate = sqlx::query("SELECT id,version,status,content FROM school_message_advertisement_revisions WHERE advertisement_id=$1 AND status IN ('draft','pending') FOR UPDATE")
                    .bind(advertisement).fetch_optional(&mut *tx).await?;
                if let Some(row) = candidate {
                    conflict(
                        row.get::<String, _>("status") == "draft"
                            && input
                                .expected_revision_version
                                .as_deref()
                                .and_then(|value| domain::version(value).ok())
                                == Some(row.get("version")),
                    )?;
                    let id: Uuid = row.get("id");
                    if row.get::<String, _>("content") == input.content {
                        return config_value(&mut tx, advertisement).await;
                    }
                    sqlx::query("UPDATE school_message_advertisement_revisions SET content=$2,version=version+1,updated_at=now() WHERE id=$1").bind(id).bind(input.content).execute(&mut *tx).await?;
                    target_revision = Some(id);
                } else {
                    conflict(input.expected_revision_version.is_none())?;
                    let same: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_message_advertisements c JOIN school_message_advertisement_revisions r ON r.id=c.current_revision_id WHERE c.id=$1 AND r.content=$2)").bind(advertisement).bind(&input.content).fetch_one(&mut *tx).await?;
                    if same {
                        return Err(Error::Invalid("内容未变化".into()));
                    }
                    target_revision = Some(
                        new_draft(&mut tx, actor, school, advertisement, &input.content).await?,
                    );
                }
                "draft.update"
            }
            Command::Submit(input) | Command::Withdraw(input) => {
                let id = revision.ok_or(Error::NotFound)?;
                let row = sqlx::query("SELECT * FROM school_message_advertisement_revisions WHERE id=$1 AND advertisement_id=$2 AND school_id=$3 FOR UPDATE")
                    .bind(id).bind(advertisement).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                conflict(
                    row.get::<i64, _>("version")
                        == domain::version(&input.expected_revision_version)?,
                )?;
                let status: String = row.get("status");
                if submitting {
                    conflict(status == "draft")?;
                    let digest = domain::digest(
                        school,
                        advertisement,
                        id,
                        row.get("number"),
                        &row.get::<String, _>("content"),
                    );
                    sqlx::query("UPDATE school_message_advertisement_revisions SET status='pending',review_digest=$2,submitted_by=$3,submitted_at=now(),version=version+1,updated_at=now() WHERE id=$1")
                        .bind(id).bind(digest).bind(actor.id).execute(&mut *tx).await?;
                    "submit"
                } else {
                    conflict(matches!(status.as_str(), "draft" | "pending"))?;
                    sqlx::query("UPDATE school_message_advertisement_revisions SET status='withdrawn',version=version+1,updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await?;
                    "withdraw"
                }
            }
            Command::Decide(input) => {
                let id = revision.ok_or(Error::NotFound)?;
                let row = sqlx::query("SELECT * FROM school_message_advertisement_revisions WHERE id=$1 AND advertisement_id=$2 AND school_id=$3 FOR UPDATE")
                    .bind(id).bind(advertisement).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                conflict(
                    row.get::<String, _>("status") == "pending"
                        && row.get::<i64, _>("version")
                            == domain::version(&input.expected_revision_version)?
                        && row.get::<Option<String>, _>("review_digest").as_deref()
                            == Some(input.review_digest.as_str()),
                )?;
                let approve = input.decision == "approve";
                sqlx::query("INSERT INTO school_message_advertisement_decisions(id,revision_id,advertisement_id,school_id,decision,reason,reviewer_id,review_digest) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
                    .bind(Uuid::now_v7()).bind(id).bind(advertisement).bind(school).bind(input.decision).bind(input.reason).bind(actor.id).bind(input.review_digest).execute(&mut *tx).await?;
                sqlx::query("UPDATE school_message_advertisement_revisions SET status=$2,version=version+1,updated_at=now() WHERE id=$1").bind(id).bind(if approve { "approved" } else { "rejected" }).execute(&mut *tx).await?;
                if approve {
                    sqlx::query("UPDATE school_message_advertisements SET current_revision_id=$2 WHERE id=$1").bind(advertisement).bind(id).execute(&mut *tx).await?;
                }
                if approve { "approve" } else { "reject" }
            }
            Command::Delete(_) | Command::Remove(_) => {
                sqlx::query("UPDATE school_message_advertisement_revisions SET status='withdrawn',version=version+1,updated_at=now() WHERE advertisement_id=$1 AND status IN ('draft','pending')")
                    .bind(advertisement).execute(&mut *tx).await?;
                sqlx::query("UPDATE school_message_advertisements SET current_revision_id=NULL,deleted_at=now(),deleted_by=$2,deletion_kind=$3,deletion_reason=$4,version=version+1,updated_at=now() WHERE id=$1")
                    .bind(advertisement).bind(actor.id).bind(if removal_reason.is_some() { "platform_remove" } else { "school_delete" }).bind(&removal_reason).execute(&mut *tx).await?;
                if removal_reason.is_some() {
                    "remove"
                } else {
                    "delete"
                }
            }
        };
        if !matches!(action, "create" | "delete" | "remove") {
            sqlx::query("UPDATE school_message_advertisements SET version=version+1,updated_at=now() WHERE id=$1").bind(advertisement).execute(&mut *tx).await?;
        }
        let value = config_value(&mut tx, advertisement).await?;
        let after_revision: Option<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',id,'version',version::text,'status',status) FROM school_message_advertisement_revisions WHERE id=$1")
            .bind(target_revision).fetch_optional(&mut *tx).await?;
        crate::audit::record(
            &mut tx,
            actor,
            school,
            &format!("message_advertisement.{action}"),
            advertisement,
            json!({"revision_id":target_revision,"before_config_version":before_config_version,"after_config_version":value["version"],"before_revision":before_revision,"after_revision":after_revision,"decision":decision,"reason":reason}),
        )
        .await?;
        tx.commit().await?;
        Ok(value)
    }
}

pub struct Candidate {
    pub advertisement: Uuid,
    pub revision: Uuid,
    pub content: String,
}

pub async fn candidate(
    tx: &mut Transaction<'_, Postgres>,
    school: Uuid,
) -> Result<Option<Candidate>> {
    Ok(sqlx::query("SELECT c.id,r.id AS revision_id,r.content FROM school_message_advertisements c JOIN school_message_advertisement_revisions r ON r.id=c.current_revision_id AND r.advertisement_id=c.id AND r.school_id=c.school_id JOIN school_message_advertisement_decisions d ON d.revision_id=r.id AND d.decision='approve' AND d.review_digest=r.review_digest WHERE c.school_id=$1 AND c.deleted_at IS NULL AND r.status='approved'")
        .bind(school).fetch_optional(&mut **tx).await?.map(|row| Candidate { advertisement: row.get("id"), revision: row.get("revision_id"), content: row.get("content") }))
}

pub async fn save_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    event: Uuid,
    school: Uuid,
    candidate: Option<&Candidate>,
    composition: &domain::Composition,
) -> Result<()> {
    sqlx::query("INSERT INTO forum_message_advertisement_snapshots(event_id,school_id,advertisement_id,revision_id,content,result,base_length,final_length,composition_version) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(event).bind(school).bind(candidate.map(|value| value.advertisement)).bind(candidate.map(|value| value.revision)).bind(candidate.map(|value| value.content.as_str())).bind(composition.result).bind(composition.base_length as i32).bind(composition.final_length as i32).bind(domain::COMPOSITION_VERSION).execute(&mut **tx).await?;
    Ok(())
}
