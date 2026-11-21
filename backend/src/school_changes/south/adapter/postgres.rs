use crate::business::{Actor, Error, Result};
use crate::media::south::adapter::filesystem::Files;
use crate::miniapp_users::south::adapter::postgres::school_binding;
use crate::school_changes::{domain::policy::Policy, south::port::Store};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

mod cleanup;
mod documents;
mod requests;
mod review;

#[derive(Clone, Debug)]
pub struct Settings {
    pub policy: Arc<Policy>,
    pub files: Files,
}
impl Settings {
    pub fn new(policy: Policy, public: &Files) -> Result<Self> {
        policy.validate()?;
        let files = super::private_files::open(&policy, public)?;
        Ok(Self {
            policy: Arc::new(policy),
            files,
        })
    }
    pub fn defaults(public: &Files) -> Result<Self> {
        Self::new(super::private_files::defaults(public)?, public)
    }
    pub fn from_env(public: &Files) -> Result<Self> {
        match Policy::from_env()? {
            Some(policy) => Self::new(policy, public),
            None => Self::defaults(public),
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self::defaults(&Files::default()).expect("test school change storage")
    }
}
#[derive(Clone)]
pub struct Repository {
    pub pool: PgPool,
    pub settings: Settings,
    decode: Arc<tokio::sync::Semaphore>,
}
impl Repository {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self {
            pool,
            settings,
            decode: Arc::new(tokio::sync::Semaphore::new(2)),
        }
    }
}
pub async fn own_session(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
) -> Result<Option<Uuid>> {
    if actor.is_admin_browser() {
        return Err(Error::Forbidden);
    }
    let (school, status, _) =
        crate::miniapp_users::south::adapter::postgres::lock_user(tx, actor.id).await?;
    if status != "normal" {
        return Err(Error::MiniappBanned);
    }
    crate::miniapp_users::south::adapter::postgres::guard_session(tx, actor).await?;
    let ordinary: bool =
        sqlx::query_scalar("SELECT account_kind='ordinary' FROM accounts WHERE id=$1")
            .bind(actor.id)
            .fetch_one(&mut **tx)
            .await?;
    if !ordinary {
        return Err(Error::Forbidden);
    }
    Ok(school)
}
pub async fn event(
    tx: &mut Transaction<'_, Postgres>,
    request: Option<Uuid>,
    document: Option<Uuid>,
    actor: Option<Uuid>,
    action: &str,
    code: &str,
    seconds: i64,
) -> Result<()> {
    sqlx::query("INSERT INTO miniapp_school_change_events(id,request_id,document_id,actor_id,action,reason_code,purge_after) VALUES($1,$2,$3,$4,$5,$6,now()+$7::bigint*interval '1 second')")
        .bind(Uuid::now_v7()).bind(request).bind(document).bind(actor).bind(action).bind(code).bind(seconds).execute(&mut **tx).await?;
    Ok(())
}
pub async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    request: Uuid,
    status: &str,
    reviewer: Option<Uuid>,
    actor: Option<Uuid>,
    code: &str,
    reason: Option<&str>,
) -> Result<()> {
    let seconds: i64 = sqlx::query_scalar("UPDATE miniapp_school_change_requests SET status=$2,revision=revision+1,closed_at=CASE WHEN $4='review_timeout' THEN review_due_at ELSE clock_timestamp() END,reviewed_by=$3,reviewed_at=CASE WHEN $3::uuid IS NULL THEN NULL ELSE clock_timestamp() END,result_reason_code=$4,result_reason=$5 WHERE id=$1 AND status='pending' RETURNING audit_retention_seconds")
        .bind(request).bind(status).bind(reviewer).bind(code).bind(reason).fetch_optional(&mut **tx).await?.ok_or(Error::Policy("school_change_conflict"))?;
    sqlx::query("UPDATE miniapp_school_change_documents d SET purge_after=CASE WHEN r.status='approved' THEN r.closed_at ELSE r.closed_at+r.document_retention_seconds*interval '1 second' END FROM miniapp_school_change_requests r WHERE d.request_id=r.id AND r.id=$1").bind(request).execute(&mut **tx).await?;
    sqlx::query("UPDATE miniapp_school_change_events SET purge_after=now()+$2::bigint*interval '1 second' WHERE request_id=$1").bind(request).bind(seconds).execute(&mut **tx).await?;
    event(tx, Some(request), None, actor, status, code, seconds).await
}

pub(crate) async fn expire_pending(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
) -> Result<bool> {
    let requests: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM miniapp_school_change_requests WHERE account_id=$1 AND status='pending' AND review_due_at<=clock_timestamp() ORDER BY id FOR UPDATE")
        .bind(account).fetch_all(&mut **tx).await?;
    for request in &requests {
        finish(
            tx,
            *request,
            "rejected",
            None,
            None,
            "review_timeout",
            Some("申请超过审核期限未完成审核，已由系统自动驳回，可重新申请。"),
        )
        .await?;
    }
    Ok(!requests.is_empty())
}

const VIEW: &str = "SELECT jsonb_build_object('id',r.id,'account_id',r.account_id,'from_school_id',r.from_school_id,'from_school_name',fs.name,'to_school_id',r.to_school_id,'to_school_name',ts.name,'status',r.status,'revision',r.revision,'submitted_at',r.submitted_at,'review_due_at',r.review_due_at,'closed_at',r.closed_at,'reviewed_at',r.reviewed_at,'reviewed_by',r.reviewed_by,'declared_student_name',r.declared_student_name,'declared_student_number',r.declared_student_number,'reason',r.reason,'consent_version',r.consent_version,'result_reason_code',r.result_reason_code,'result_reason',r.result_reason,'personal_data_purged',r.personal_data_purged_at IS NOT NULL,'documents',coalesce((SELECT jsonb_agg(jsonb_build_object('id',d.id,'mime',d.mime,'available',d.state='ready' AND (d.purge_after IS NULL OR d.purge_after>now()))) FROM miniapp_school_change_documents d WHERE d.request_id=r.id),'[]'::jsonb),'blocked_reasons',to_jsonb(array_remove(ARRAY[CASE WHEN NOT a.active OR a.account_kind<>'ordinary' OR u.status<>'normal' THEN 'account_unavailable' END,CASE WHEN NOT ts.active THEN 'school_unavailable' END,CASE WHEN a.bound_school_id IS DISTINCT FROM r.from_school_id THEN 'source_changed' END,CASE WHEN EXISTS(SELECT 1 FROM school_memberships m WHERE m.account_id=r.account_id AND m.school_id IN(r.from_school_id,r.to_school_id) AND m.role IN('admin','operator')) THEN 'role_conflict' END,CASE WHEN NOT EXISTS(SELECT 1 FROM miniapp_school_change_documents d WHERE d.request_id=r.id) OR EXISTS(SELECT 1 FROM miniapp_school_change_documents d WHERE d.request_id=r.id AND (d.state<>'ready' OR d.purge_after<=now())) THEN 'document_unavailable' END],NULL))) FROM miniapp_school_change_requests r JOIN schools fs ON fs.id=r.from_school_id JOIN schools ts ON ts.id=r.to_school_id JOIN accounts a ON a.id=r.account_id JOIN miniapp_users u ON u.account_id=r.account_id";
async fn view(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Value> {
    let mut value: Value = sqlx::query_scalar(&format!("{VIEW} WHERE r.id=$1"))
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
    let relationship: Option<String> = sqlx::query_scalar(
        "SELECT student_relationship FROM miniapp_school_change_requests WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    value["student_relationship"] = json!(relationship);
    if value["status"] == "approved" {
        for document in value["documents"]
            .as_array_mut()
            .ok_or(Error::Unavailable)?
        {
            document["available"] = json!(false);
        }
    }
    if value["status"] == "pending" && relationship.is_none() {
        value["blocked_reasons"]
            .as_array_mut()
            .ok_or(Error::Unavailable)?
            .push(json!("relationship_required"));
    }
    let expired: bool = sqlx::query_scalar("SELECT closed_at IS NOT NULL AND closed_at+personal_retention_seconds*interval '1 second'<=now() FROM miniapp_school_change_requests WHERE id=$1").bind(id).fetch_one(&mut **tx).await?;
    if expired {
        for field in [
            "declared_student_name",
            "declared_student_number",
            "reason",
            "result_reason",
        ] {
            value[field] = Value::Null;
        }
        value["personal_data_purged"] = json!(true);
    }
    if value["status"] != "pending" {
        value["blocked_reasons"] = json!([]);
    }
    Ok(value)
}
impl Store for Repository {
    fn policy(&self) -> Value {
        let settings = &self.settings;
        json!({"id":"current","enabled":true,"version":settings.policy.version,"notice":settings.policy.notice,"student_number_required":settings.policy.student_number_required,"max_file_bytes":settings.policy.max_file_bytes,"max_total_bytes":settings.policy.max_total_bytes,"max_documents":settings.policy.max_documents,"review_working_days":settings.policy.review_working_days})
    }
    async fn upload(&self, actor: &Actor, bytes: Vec<u8>) -> Result<Value> {
        self.upload_document(actor, bytes).await
    }
    async fn submit(
        &self,
        actor: &Actor,
        input: crate::school_changes::domain::Submit,
    ) -> Result<Value> {
        self.submit_request(actor, input).await
    }
    async fn current(&self, actor: &Actor) -> Result<Option<Value>> {
        let mut tx = self.pool.begin().await?;
        own_session(&mut tx, actor).await?;
        expire_pending(&mut tx, actor.id).await?;
        let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM miniapp_school_change_requests WHERE account_id=$1 ORDER BY (status='pending') DESC,submitted_at DESC,id DESC LIMIT 1").bind(actor.id).fetch_optional(&mut *tx).await?;
        let result = if let Some(id) = id {
            Some(view(&mut tx, id).await?)
        } else {
            None
        };
        tx.commit().await?;
        Ok(result)
    }
    async fn detail(&self, actor: &Actor, id: Uuid, platform: bool) -> Result<Value> {
        let owner: Uuid = sqlx::query_scalar("SELECT account_id FROM miniapp_school_change_requests WHERE id=$1 AND ($2 OR account_id=$3)").bind(id).bind(platform).bind(actor.id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let mut tx = self.pool.begin().await?;
        school_binding::lock_accounts(&mut tx, actor, owner).await?;
        if platform {
            school_binding::platform_session(&mut tx, actor).await?;
        } else {
            own_session(&mut tx, actor).await?;
        }
        sqlx::query("SELECT account_id FROM miniapp_school_change_requests WHERE id=$1 AND ($2 OR account_id=$3)").bind(id).bind(platform).bind(actor.id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        expire_pending(&mut tx, owner).await?;
        let mut result = view(&mut tx, id).await?;
        if platform {
            let guidance: String = sqlx::query_scalar(
                "SELECT review_guidance FROM miniapp_school_change_requests WHERE id=$1",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            result["review_guidance"] = json!(guidance);
            if result["status"] == "pending" {
                let mut blockers = result["blocked_reasons"]
                    .as_array()
                    .ok_or(Error::Unavailable)?
                    .clone();
                if !self.settings.policy.restore_revoked_member {
                    let revoked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_memberships m JOIN miniapp_school_change_requests r ON r.account_id=m.account_id AND r.to_school_id=m.school_id WHERE r.id=$1 AND m.role='revoked')").bind(id).fetch_one(&mut *tx).await?;
                    if revoked {
                        blockers.push(json!("target_member_revoked"));
                    }
                }
                result["blocked_reasons"] = json!(blockers);
            }
            let history: Vec<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',id,'action',action,'actor_id',actor_id,'reason_code',reason_code,'created_at',created_at) FROM miniapp_school_change_events WHERE request_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100").bind(id).fetch_all(&mut *tx).await?;
            result["history"] = json!(history);
            let retention: i64 = sqlx::query_scalar(
                "SELECT audit_retention_seconds FROM miniapp_school_change_requests WHERE id=$1",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            event(
                &mut tx,
                Some(id),
                None,
                Some(actor.id),
                "detail_read",
                "platform",
                retention,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(result)
    }
    async fn list(
        &self,
        actor: &Actor,
        filters: &crate::school_changes::domain::Filters,
    ) -> Result<Vec<Value>> {
        let mut tx = self.pool.begin().await?;
        school_binding::platform_session(&mut tx, actor).await?;
        let (limit, offset) = filters.page().fetch_bounds()?;
        let mut rows: Vec<Value> = sqlx::query_scalar(&format!("{VIEW} WHERE ($1::text IS NULL OR r.status=$1) AND ($2::timestamptz IS NULL OR r.submitted_at>=$2) AND ($3::timestamptz IS NULL OR r.submitted_at<=$3) AND ($6::uuid IS NULL OR r.account_id=$6) ORDER BY r.submitted_at,r.id LIMIT $4 OFFSET $5"))
            .bind(&filters.status).bind(filters.created_from).bind(filters.created_to).bind(limit).bind(offset).bind(filters.account_id).fetch_all(&mut *tx).await?;
        for row in &mut rows {
            if row["status"] != "pending" {
                row["blocked_reasons"] = json!([]);
            }
            for field in [
                "declared_student_name",
                "declared_student_number",
                "reason",
                "result_reason",
                "documents",
            ] {
                row.as_object_mut().ok_or(Error::Unavailable)?.remove(field);
            }
        }
        tx.commit().await?;
        Ok(rows)
    }
    async fn close(
        &self,
        actor: &Actor,
        id: Uuid,
        revision: i64,
        decision: Option<crate::school_changes::domain::Decision>,
        approve: bool,
    ) -> Result<Value> {
        self.close_request(actor, id, revision, decision, approve)
            .await
    }
    async fn document(&self, actor: &Actor, id: Uuid, request: Option<Uuid>) -> Result<Vec<u8>> {
        self.read_document(actor, id, request).await
    }
    async fn cleanup(&self) -> Result<()> {
        self.cleanup_documents().await
    }
    async fn purge_account_data(&self, account: Uuid) -> Result<()> {
        self.purge_account(account).await
    }
}
