use super::*;
use crate::business::require;
use crate::school_changes::domain::Submit;
use ring::hmac;
use sqlx::Row;

impl Repository {
    pub(super) async fn submit_request(&self, actor: &Actor, mut input: Submit) -> Result<Value> {
        let settings = &self.settings;
        let policy = &settings.policy;
        input.validate(policy)?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, policy.digest_key.as_bytes());
        let digest = hmac::sign(
            &key,
            &serde_json::to_vec(&input).map_err(|_| Error::Unavailable)?,
        );
        let digest: Vec<u8> = digest.as_ref().to_vec();
        let digest = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mut tx = self.pool.begin().await?;
        let from = own_session(&mut tx, actor)
            .await?
            .ok_or(Error::Policy("school_change_unbound"))?;
        expire_pending(&mut tx, actor.id).await?;
        let existing = sqlx::query("SELECT id,submission_digest FROM miniapp_school_change_requests WHERE account_id=$1 AND submission_key=$2").bind(actor.id).bind(input.submission_key).fetch_optional(&mut *tx).await?;
        if let Some(existing) = existing {
            if existing.get::<String, _>("submission_digest") != digest {
                return Err(Error::Policy("school_change_idempotency_conflict"));
            }
            let result = view(&mut tx, existing.get("id")).await?;
            tx.commit().await?;
            return Ok(result);
        }
        require(from != input.to_school_id, "目标学校不能与当前学校相同")?;
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM miniapp_school_change_requests WHERE account_id=$1 AND status='pending')").bind(actor.id).fetch_one(&mut *tx).await?;
        if pending {
            return Err(Error::Policy("school_change_pending"));
        }
        let recent: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM miniapp_school_change_requests WHERE account_id=$1 AND submitted_at>now()-$2::bigint*interval '1 second')").bind(actor.id).bind(policy.min_submission_seconds).fetch_one(&mut *tx).await?;
        if recent {
            return Err(Error::RateLimited);
        }
        school_binding::validate_target(&mut tx, actor.id, Some(from), input.to_school_id).await?;
        let mut total = 0_i64;
        for document in &input.document_ids {
            let row = sqlx::query("SELECT size_bytes FROM miniapp_school_change_documents WHERE id=$1 AND owner_id=$2 AND request_id IS NULL AND state='ready' AND expires_at>now() AND store_id=$3 FOR UPDATE")
                .bind(document).bind(actor.id).bind(settings.files.store_id).fetch_optional(&mut *tx).await?.ok_or(Error::Policy("school_change_document_unavailable"))?;
            total += row.get::<i64, _>("size_bytes");
        }
        require(total <= policy.max_total_bytes, "材料总大小超过限制")?;
        let id = Uuid::now_v7();
        let submitted = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let deadline = crate::school_changes::domain::calendar::review_deadline(submitted, policy)?;
        sqlx::query("INSERT INTO miniapp_school_change_requests(id,account_id,from_school_id,to_school_id,declared_student_name,declared_student_number,reason,submission_key,submission_digest,consent_version,review_due_at,document_retention_seconds,personal_retention_seconds,audit_retention_seconds,consent_notice,review_guidance,student_relationship,submitted_at,consented_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$18)")
            .bind(id).bind(actor.id).bind(from).bind(input.to_school_id).bind(input.declared_student_name).bind(input.declared_student_number).bind(input.reason).bind(input.submission_key).bind(digest).bind(input.consent_version).bind(deadline).bind(policy.document_retention_seconds).bind(policy.personal_retention_seconds).bind(policy.audit_retention_seconds).bind(&policy.notice).bind(&policy.review_guidance).bind(input.student_relationship.as_str()).bind(submitted).execute(&mut *tx).await?;
        sqlx::query("UPDATE miniapp_school_change_documents SET request_id=$1,expires_at=NULL WHERE id=ANY($2) AND owner_id=$3").bind(id).bind(input.document_ids).bind(actor.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE miniapp_school_change_events e SET request_id=$1 FROM miniapp_school_change_documents d WHERE e.document_id=d.id AND d.request_id=$1 AND e.request_id IS NULL").bind(id).execute(&mut *tx).await?;
        event(
            &mut tx,
            Some(id),
            None,
            Some(actor.id),
            "submitted",
            "user",
            policy.audit_retention_seconds,
        )
        .await?;
        let result = view(&mut tx, id).await?;
        tx.commit().await?;
        Ok(result)
    }
}
