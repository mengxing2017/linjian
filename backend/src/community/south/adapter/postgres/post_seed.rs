use super::{decisions, post_content};
use crate::{
    business::{Actor, Error, Result, hash, require},
    community::{domain::post_seed::*, south::port::post_seed::Store},
    media::south::adapter::{
        filesystem::{Files, digest},
        lifecycle,
    },
};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

mod avatar_assets;
pub use avatar_assets::avatar_png;

#[derive(Clone)]
pub struct Repository(pub PgPool, pub Files);

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    school: Uuid,
) -> Result<Value> {
    crate::schools::south::adapter::postgres::permissions::authorize(
        tx,
        actor,
        school,
        "content.simulation.manage",
    )
    .await?;
    sqlx::query_scalar("SELECT jsonb_build_object('revision',revision,'config',config) FROM schools WHERE id=$1 AND active FOR UPDATE")
        .bind(school).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)
}
fn policy_digest(school: &Value) -> String {
    hash(&json!([school, AVATAR_VERSION]).to_string())
}
fn payload(value: Value) -> Result<PreviewPayload> {
    serde_json::from_value(value).map_err(|_| Error::Unavailable)
}
async fn batch_view(tx: &mut Transaction<'_, Postgres>, school: Uuid, id: Uuid) -> Result<Value> {
    let value:Value=sqlx::query_scalar("SELECT (to_jsonb(b)-'commit_idempotency_key'-'commit_request_digest'-'policy_digest')||jsonb_build_object('post_count',(SELECT count(*) FROM post_seed_items WHERE batch_id=b.id),'interaction_count',(SELECT count(*) FROM comments WHERE post_id IN (SELECT post_id FROM post_seed_items WHERE batch_id=b.id))+(SELECT count(*) FROM post_reactions WHERE post_id IN (SELECT post_id FROM post_seed_items WHERE batch_id=b.id))+(SELECT count(*) FROM cases WHERE post_id IN (SELECT post_id FROM post_seed_items WHERE batch_id=b.id))) FROM post_seed_batches b WHERE school_id=$1 AND id=$2")
        .bind(school).bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    let mut view: BatchView = serde_json::from_value(value).map_err(|_| Error::Unavailable)?;
    view.row_labels = view
        .preview_payload
        .rows
        .iter()
        .map(|row| {
            crate::community::domain::post_text::display_label(&row.title, &row.content, false)
        })
        .collect();
    serde_json::to_value(view).map_err(|_| Error::Unavailable)
}
async fn lock_batch(tx: &mut Transaction<'_, Postgres>, school: Uuid, id: Uuid) -> Result<Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(b) FROM post_seed_batches b WHERE school_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(school)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::NotFound)
}
async fn validate_rows(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    school: Uuid,
    rows: &[SeedRow],
) -> Result<Vec<RowError>> {
    let mut errors = vec![];
    for (index, row) in rows.iter().enumerate() {
        let result = async {
            row.validate(school, Utc::now())?;
            post_content::validate(tx, school, actor, None, &row.post()).await
        }
        .await;
        match result {
            Ok(()) => (),
            Err(Error::Invalid(message)) => errors.push(RowError {
                row: index + 1,
                message,
            }),
            Err(error) => return Err(error),
        }
    }
    Ok(errors)
}

impl Store for Repository {
    async fn preview(&self, actor: &Actor, school: Uuid, mut input: PreviewInput) -> Result<Value> {
        input.validate()?;
        for row in &mut input.rows {
            if let Ok(post) = row.post().normalized() {
                row.title = post.title;
                row.content = post.content;
            }
        }
        let mut tx = lifecycle::begin(&self.0).await?;
        let policy = authorize(&mut tx, actor, school).await?;
        let mut authors = vec![];
        for author in input.authors {
            let generated = Uuid::now_v7();
            let entropy = Uuid::new_v4();
            let mut name = author.nickname.unwrap_or_else(|| {
                format!(
                    "校园{}{}",
                    [
                        "晴天", "微风", "青柠", "星光", "小树", "晨曦", "暖阳", "云朵"
                    ][usize::from(entropy.as_bytes()[0]) % 8],
                    &entropy.simple().to_string()[..4]
                )
            });
            let mut revision = None;
            let mut avatar_url = None;
            if let Some(existing) = author.existing_id {
                let row=sqlx::query("SELECT u.nickname,u.revision,u.avatar_asset_id FROM accounts a JOIN miniapp_users u ON u.account_id=a.id WHERE a.id=$1 AND a.bound_school_id=$2 AND a.account_kind='simulated' AND a.active AND u.status='normal' FOR SHARE OF a,u")
                    .bind(existing).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                name = row.get("nickname");
                revision = Some(row.get("revision"));
                avatar_url = row
                    .get::<Option<Uuid>, _>("avatar_asset_id")
                    .map(|id| format!("/api/v1/miniapp/avatar-assets/{id}"));
            }
            crate::business::text(&name, 1, 100)?;
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM blocked_words WHERE (school_id=$1 OR school_id IS NULL) AND active AND position(lower(word) in lower($2))>0)")
                .bind(school).bind(&name).fetch_one(&mut *tx).await?;
            require(!blocked, "昵称包含禁止使用的词语")?;
            authors.push(PreviewAuthor {
                key: author.key,
                id: author.existing_id.unwrap_or(generated),
                existing: author.existing_id.is_some(),
                nickname: name,
                avatar_id: author
                    .avatar_id
                    .unwrap_or(entropy.as_bytes()[1] % AVATAR_COUNT),
                revision,
                avatar_url,
            });
        }
        let errors = validate_rows(&mut tx, actor, school, &input.rows).await?;
        let audit_mode = if policy["config"]["post_audit_mode"] == "auto" {
            "auto"
        } else {
            "manual"
        };
        let preview = PreviewPayload {
            authors,
            rows: input.rows,
            errors,
            audit_mode: audit_mode.into(),
            avatar_version: AVATAR_VERSION.into(),
        };
        let value = serde_json::to_value(preview).map_err(|_| Error::Unavailable)?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO post_seed_batches(id,school_id,created_by,name,preview_digest,policy_digest,preview_payload) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(school).bind(actor.id).bind(input.name.trim()).bind(hash(&value.to_string())).bind(policy_digest(&policy)).bind(value).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM post_seed_batches WHERE school_id=$1 AND created_by=$2 AND state='preview' AND expires_at<now()")
            .bind(school).bind(actor.id).execute(&mut *tx).await?;
        let result = batch_view(&mut tx, school, id).await?;
        tx.commit().await?;
        Ok(result)
    }
    async fn commit(
        &self,
        actor: &Actor,
        school: Uuid,
        batch: Uuid,
        input: CommitInput,
    ) -> Result<Value> {
        let mut lock = lifecycle::FileLock::shared(&self.0).await?;
        let mut tx = lifecycle::begin(&self.0).await?;
        let policy = authorize(&mut tx, actor, school).await?;
        let current = lock_batch(&mut tx, school, batch).await?;
        if current["created_by"] != json!(actor.id) {
            return Err(Error::Forbidden);
        }
        let request_digest = hash(&json!([batch, input.revision, input.digest]).to_string());
        if current["state"] != "preview" {
            if current["commit_idempotency_key"] != json!(input.idempotency_key)
                || current["commit_request_digest"] != request_digest
            {
                return Err(Error::Conflict);
            }
            let result = batch_view(&mut tx, school, batch).await?;
            tx.commit().await?;
            lock.release().await?;
            return Ok(result);
        }
        if current["revision"] != input.revision
            || current["preview_digest"] != input.digest
            || current["policy_digest"] != policy_digest(&policy)
        {
            return Err(Error::Conflict);
        }
        let expires: chrono::DateTime<Utc> = serde_json::from_value(current["expires_at"].clone())
            .map_err(|_| Error::Unavailable)?;
        require(expires > Utc::now(), "预览已过期，请重新预览")?;
        let preview = payload(current["preview_payload"].clone())?;
        require(preview.errors.is_empty(), "请先修正预览错误")?;
        require(
            validate_rows(&mut tx, actor, school, &preview.rows)
                .await?
                .is_empty(),
            "内容校验已变化，请重新预览",
        )?;
        for author in &preview.authors {
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM blocked_words WHERE (school_id=$1 OR school_id IS NULL) AND active AND position(lower(word) in lower($2))>0)")
                .bind(school).bind(&author.nickname).fetch_one(&mut *tx).await?;
            require(!blocked, "昵称校验已变化，请重新预览")?;
            if author.existing {
                let valid:bool=sqlx::query_scalar("SELECT true FROM accounts a JOIN miniapp_users u ON u.account_id=a.id WHERE a.id=$1 AND a.bound_school_id=$2 AND a.account_kind='simulated' AND a.active AND u.status='normal' AND u.revision=$3 AND u.nickname=$4 FOR SHARE OF a,u")
                    .bind(author.id).bind(school).bind(author.revision).bind(&author.nickname).fetch_optional(&mut *tx).await?.unwrap_or(false);
                if !valid {
                    return Err(Error::Conflict);
                }
            } else {
                sqlx::query("INSERT INTO accounts(id,username,display_name,bound_school_id,account_kind) VALUES($1,$2,$3,$4,'simulated')")
                    .bind(author.id).bind(format!("linjian-seed-{}",author.id)).bind(&author.nickname).bind(school).execute(&mut *tx).await?;
                let bytes = avatar_png(author.avatar_id)?;
                let asset = Uuid::now_v7();
                sqlx::query("INSERT INTO file_assets(id,owner_id,purpose,store_id,mime_type,size_bytes,width,height,sha256,state,idempotency_key,expires_at,gc_after) VALUES($1,$2,'avatar',$3,'image/png',$4,128,128,$5,'uploading',$1,now()+interval '1 day',now()+interval '1 day')")
                    .bind(asset).bind(actor.id).bind(self.1.store_id).bind(bytes.len() as i64).bind(digest(&bytes)).execute(&mut lock.connection).await?;
                self.1.write(asset, bytes).await?;
                sqlx::query("UPDATE file_assets SET owner_id=$2,state='ready' WHERE id=$1")
                    .bind(asset)
                    .bind(author.id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO miniapp_avatar_assets(id,owner_id,mime_type) VALUES($1,$2,'image/png')").bind(asset).bind(author.id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO miniapp_users(account_id,nickname,avatar_asset_id) VALUES($1,$2,$3)").bind(author.id).bind(&author.nickname).bind(asset).execute(&mut *tx).await?;
                crate::audit::record(
                    &mut tx,
                    actor,
                    school,
                    "post.seed_author.create",
                    author.id,
                    json!({"batch_id":batch}),
                )
                .await?;
            }
        }
        let policy = decisions::policy(&mut tx, school, false).await?;
        for (index, row) in preview.rows.iter().enumerate() {
            let author = preview
                .authors
                .iter()
                .find(|author| author.key == row.author_key)
                .ok_or(Error::Conflict)?;
            let id = Uuid::now_v7();
            let item:Value=sqlx::query_scalar("INSERT INTO posts(id,school_id,author_id,title,content,tags,created_via,source_projection,seed_batch_id,display_published_at) VALUES($1,$2,$3,$4,$5,$6,'seed','message_v1',$7,$8) RETURNING to_jsonb(posts)")
                .bind(id).bind(school).bind(author.id).bind(row.title.trim()).bind(row.content.trim()).bind(json!(row.tags)).bind(batch).bind(row.display_published_at).fetch_one(&mut *tx).await?;
            decisions::automatically_approve(
                &mut tx,
                item,
                decisions::PolicyDecision {
                    mode: policy.mode,
                    revision: policy.revision,
                },
            )
            .await?;
            sqlx::query("INSERT INTO post_seed_items(batch_id,school_id,row_number,post_id,author_id) VALUES($1,$2,$3,$4,$5)")
                .bind(batch).bind(school).bind((index+1) as i32).bind(id).bind(author.id).execute(&mut *tx).await?;
            crate::audit::record(&mut tx,actor,school,"post.seed.create",id,json!({"batch_id":batch,"author_id":author.id,"display_published_at":row.display_published_at})).await?;
        }
        sqlx::query("UPDATE post_seed_batches SET state='committed',revision=revision+1,committed_at=now(),commit_idempotency_key=$2,commit_request_digest=$3 WHERE id=$1")
            .bind(batch).bind(input.idempotency_key).bind(request_digest).execute(&mut *tx).await?;
        crate::audit::record(
            &mut tx,
            actor,
            school,
            "post.seed_batch.commit",
            batch,
            json!({"count":preview.rows.len()}),
        )
        .await?;
        let result = batch_view(&mut tx, school, batch).await?;
        tx.commit().await?;
        lock.release().await?;
        Ok(result)
    }
    async fn revoke(
        &self,
        actor: &Actor,
        school: Uuid,
        batch: Uuid,
        input: RevokeInput,
    ) -> Result<Value> {
        let mut tx = lifecycle::begin(&self.0).await?;
        authorize(&mut tx, actor, school).await?;
        let current = lock_batch(&mut tx, school, batch).await?;
        if current["state"] != "revoked" {
            if current["state"] != "committed" || current["revision"] != input.revision {
                return Err(Error::Conflict);
            }
            sqlx::query("UPDATE posts SET visibility='offline',revision=revision+1,updated_at=now() WHERE seed_batch_id=$1 AND school_id=$2 AND visibility NOT IN ('deleted','offline')")
                .bind(batch).bind(school).execute(&mut *tx).await?;
            sqlx::query("UPDATE post_seed_batches SET state='revoked',revision=revision+1,revoked_at=now(),revoked_by=$2,revoke_reason=$3 WHERE id=$1")
                .bind(batch).bind(actor.id).bind(input.reason.trim()).execute(&mut *tx).await?;
            crate::audit::record(
                &mut tx,
                actor,
                school,
                "post.seed_batch.revoke",
                batch,
                json!({"reason":input.reason}),
            )
            .await?;
        }
        let result = batch_view(&mut tx, school, batch).await?;
        tx.commit().await?;
        Ok(result)
    }
    async fn read(
        &self,
        actor: &Actor,
        school: Uuid,
        kind: &str,
        batch: Option<Uuid>,
        query: SeedQuery,
    ) -> Result<Value> {
        let mut tx = lifecycle::begin(&self.0).await?;
        authorize(&mut tx, actor, school).await?;
        let (limit, offset) = query.page().fetch_bounds()?;
        let result=match kind {
            "author-avatar" => {
                let author=batch.ok_or(Error::NotFound)?;
                let id:Uuid=sqlx::query_scalar("SELECT u.avatar_asset_id FROM accounts a JOIN miniapp_users u ON u.account_id=a.id WHERE a.id=$1 AND a.bound_school_id=$2 AND a.account_kind='simulated' AND u.avatar_asset_id IS NOT NULL")
                    .bind(author).bind(school).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
                json!({"id":id})
            },
            "avatars"=>json!((0..AVATAR_COUNT).map(|slot|json!({"id":slot.to_string(),"slot":slot,"version":AVATAR_VERSION,"url":format!("/api/v1/schools/{school}/post-seed-avatars/{slot}")})).collect::<Vec<_>>()),
            "authors"=>json!(sqlx::query_scalar::<_,Value>("SELECT jsonb_build_object('id',a.id,'nickname',u.nickname,'revision',u.revision,'avatar_url',CASE WHEN u.avatar_asset_id IS NOT NULL THEN '/api/v1/miniapp/avatar-assets/'||u.avatar_asset_id END) FROM accounts a JOIN miniapp_users u ON u.account_id=a.id WHERE a.account_kind='simulated' AND a.bound_school_id=$1 AND a.active AND u.status='normal' AND u.nickname ILIKE $2 ORDER BY a.created_at DESC,a.id LIMIT $3 OFFSET $4")
                .bind(school).bind(format!("%{}%",query.search.unwrap_or_default())).bind(limit).bind(offset).fetch_all(&mut *tx).await?),
            "items"=>{
                let id=batch.ok_or(Error::NotFound)?; lock_batch(&mut tx,school,id).await?;
                json!(sqlx::query_scalar::<_,Value>("SELECT jsonb_build_object('id',p.id,'row_number',i.row_number,'title',p.title,'content',p.content,'images',p.images,'author_id',p.author_id,'author_nickname',u.nickname,'audit_status',p.audit_status,'visibility',p.visibility,'created_at',p.created_at,'display_published_at',p.display_published_at) FROM post_seed_items i JOIN posts p ON p.id=i.post_id JOIN miniapp_users u ON u.account_id=p.author_id WHERE i.school_id=$1 AND i.batch_id=$2 ORDER BY i.row_number LIMIT $3 OFFSET $4")
                    .bind(school).bind(id).bind(limit).bind(offset).fetch_all(&mut *tx).await?.into_iter().map(|row| { let mut row = crate::community::domain::post_text::project(row); row.as_object_mut().unwrap().remove("content"); row.as_object_mut().unwrap().remove("images"); row }).collect::<Vec<_>>())
            },
            _=>if let Some(id)=batch { batch_view(&mut tx,school,id).await? } else {
                require(query.state.as_deref().is_none_or(|state|matches!(state,"preview"|"committed"|"revoked")),"批次状态无效")?;
                json!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(b)-'preview_payload'-'commit_idempotency_key'-'commit_request_digest'-'policy_digest' FROM post_seed_batches b WHERE school_id=$1 AND state=coalesce($2,'committed') AND (state<>'preview' OR created_by=$3) ORDER BY created_at DESC,id LIMIT $4 OFFSET $5")
                    .bind(school).bind(query.state).bind(actor.id).bind(limit).bind(offset).fetch_all(&mut *tx).await?)
            },
        };
        tx.commit().await?;
        Ok(result)
    }
}
