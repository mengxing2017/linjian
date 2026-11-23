use super::filesystem::{Files, digest};
use crate::business::{Actor, Error, Result, require};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool, pub Files);
impl Repository {
    pub async fn upload(
        &self,
        a: &Actor,
        school: Option<Uuid>,
        purpose: &str,
        bytes: Vec<u8>,
        key: Uuid,
    ) -> Result<Value> {
        self.upload_scoped(a, school, purpose, bytes, key, "resource.school.write")
            .await
    }
    pub async fn upload_scoped(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        purpose: &str,
        bytes: Vec<u8>,
        key: Uuid,
        permission: &str,
    ) -> Result<Value> {
        crate::schools::domain::permissions::artwork_context(permission, true)?;
        let mut tx = super::lifecycle::begin(&self.0).await?;
        if !actor.is_admin_browser()
            && let Some(s) = school
        {
            crate::miniapp_users::south::adapter::postgres::guard_write(&mut tx, actor, s).await?;
            if purpose == "post_image" {
                crate::miniapp_users::south::adapter::postgres::phone::require_verified(
                    &mut tx, actor,
                )
                .await?;
            }
        }
        match purpose {
            "avatar" => {
                require(
                    !actor.is_admin_browser() && school.is_none(),
                    "请使用小程序身份",
                )?;
                let (_, status, _) =
                    crate::miniapp_users::south::adapter::postgres::lock_user(&mut tx, actor.id)
                        .await?;
                if status != "normal" {
                    return Err(Error::MiniappBanned);
                }
                crate::miniapp_users::south::adapter::postgres::guard_session(&mut tx, actor)
                    .await?;
            }
            "operation_artwork" => {
                crate::schools::south::adapter::postgres::preview::authorize(
                    &mut tx, actor, school, permission,
                )
                .await?;
            }
            "post_image" | "feedback_evidence" => {
                let s = school.ok_or(Error::Forbidden)?;
                actor.authorize(s, "member")?;
                crate::schools::south::adapter::postgres::preview::authorize(
                    &mut tx, actor, school, "member",
                )
                .await?;
            }
            _ => return Err(Error::Invalid("图片用途无效".into())),
        }
        tx.commit().await?;
        let (mime, w, h, bytes) = self.1.validate(bytes).await?;
        let hash = digest(&bytes);
        let id = Uuid::now_v7();
        let hours = if purpose == "operation_artwork" {
            self.1.config.lifecycle.artwork_unbound_ttl_hours
        } else {
            self.1.config.lifecycle.unbound_ttl_hours
        };
        let row=sqlx::query("INSERT INTO file_assets(id,school_id,owner_id,purpose,store_id,mime_type,size_bytes,width,height,sha256,state,idempotency_key,expires_at,gc_after,artwork_permission) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'uploading',$11,now()+make_interval(hours=>$12),now()+make_interval(hours=>$12),$13) ON CONFLICT DO NOTHING RETURNING id")
            .bind(id).bind(school).bind(actor.id).bind(purpose).bind(self.1.store_id).bind(&mime).bind(bytes.len() as i64).bind(w as i32).bind(h as i32).bind(&hash).bind(key).bind(hours as i32).bind(if purpose == "operation_artwork" {Some(permission)} else {None}).fetch_optional(&self.0).await?;
        if row.is_none() {
            let r=sqlx::query("SELECT id,sha256,state,store_id,size_bytes FROM file_assets WHERE owner_id=$1 AND school_id IS NOT DISTINCT FROM $2 AND purpose=$3 AND idempotency_key=$4 AND artwork_permission IS NOT DISTINCT FROM $5").bind(actor.id).bind(school).bind(purpose).bind(key).bind(if purpose == "operation_artwork" {Some(permission)} else {None}).fetch_one(&self.0).await?;
            if r.get::<String, _>("sha256") != hash {
                return Err(Error::Conflict);
            }
            let existing: Uuid = r.get("id");
            match r.get::<String, _>("state").as_str() {
                "ready" => return self.view(existing).await,
                "uploading" => {
                    self.1
                        .read(r.get("store_id"), existing, r.get("size_bytes"), hash)
                        .await
                        .map_err(|_| Error::Conflict)?;
                    return self
                        .finish(actor, school, purpose, existing, permission)
                        .await;
                }
                _ => return Err(Error::Conflict),
            }
        }
        self.1.write(id, bytes).await?;
        self.finish(actor, school, purpose, id, permission).await
    }
    async fn finish(
        &self,
        a: &Actor,
        school: Option<Uuid>,
        purpose: &str,
        id: Uuid,
        permission: &str,
    ) -> Result<Value> {
        let mut tx = super::lifecycle::begin(&self.0).await?;
        // Recheck authorization after I/O: a revoked session cannot finalize.
        if !a.is_admin_browser()
            && let Some(s) = school
        {
            crate::miniapp_users::south::adapter::postgres::guard_write(&mut tx, a, s).await?;
            if purpose == "post_image" {
                crate::miniapp_users::south::adapter::postgres::phone::require_verified(&mut tx, a)
                    .await?;
            }
        }
        if purpose == "avatar" {
            crate::miniapp_users::south::adapter::postgres::guard_session(&mut tx, a).await?;
        } else {
            crate::schools::south::adapter::postgres::preview::authorize(
                &mut tx,
                a,
                school,
                if purpose == "operation_artwork" {
                    permission
                } else {
                    "member"
                },
            )
            .await?;
        }
        let state: String =
            sqlx::query_scalar("SELECT state FROM file_assets WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if state == "ready" {
            tx.commit().await?;
            return self.view(id).await;
        }
        if state != "uploading" {
            return Err(Error::Conflict);
        }
        sqlx::query("UPDATE file_assets SET state='ready' WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        match purpose {
            "operation_artwork" => {
                sqlx::query("INSERT INTO operational_media(id,school_id,owner_id,mime_type,size_bytes,width,height) SELECT id,school_id,owner_id,mime_type,size_bytes,width,height FROM file_assets WHERE id=$1").bind(id).execute(&mut *tx).await?;
            }
            "avatar" => {
                sqlx::query("INSERT INTO miniapp_avatar_assets(id,owner_id,mime_type) SELECT id,owner_id,mime_type FROM file_assets WHERE id=$1").bind(id).execute(&mut *tx).await?;
            }
            _ => {
                sqlx::query("INSERT INTO media_assets(id,school_id,owner_id,mime_type,size_bytes) SELECT id,school_id,owner_id,mime_type,size_bytes FROM file_assets WHERE id=$1").bind(id).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        self.view(id).await
    }
    pub async fn view(&self, id: Uuid) -> Result<Value> {
        let mut v:Value=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'purpose',purpose,'mime_type',mime_type,'size_bytes',size_bytes,'width',width,'height',height,'expires_at',expires_at) FROM file_assets WHERE id=$1 AND state='ready'").bind(id).fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        let (s, p): (Option<Uuid>, String) =
            sqlx::query_as("SELECT school_id,purpose FROM file_assets WHERE id=$1")
                .bind(id)
                .fetch_one(&self.0)
                .await?;
        v["url"] = json!(match p.as_str() {
            "avatar" => format!("/api/v1/miniapp/avatar-assets/{id}"),
            "operation_artwork" => format!(
                "{}/resource-artwork/{id}",
                s.map(|s| format!("/api/v1/schools/{s}"))
                    .unwrap_or("/api/v1/platform".into())
            ),
            _ => format!("/api/v1/schools/{}/media/{id}", s.ok_or(Error::Forbidden)?),
        });
        v["image_ref"] = json!({"kind":"asset","id":id});
        Ok(v)
    }
    /// Only call after the business object's authorization has been checked.
    pub async fn read(&self, id: Uuid) -> Result<(String, Vec<u8>)> {
        let r=sqlx::query("SELECT mime_type,store_id,size_bytes,sha256 FROM file_assets WHERE id=$1 AND state='ready'").bind(id).fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        Ok((
            r.get("mime_type"),
            self.1
                .read(r.get("store_id"), id, r.get("size_bytes"), r.get("sha256"))
                .await?,
        ))
    }
    pub async fn get(&self, a: &Actor, s: Uuid, id: Uuid) -> Result<(String, Vec<u8>)> {
        a.authorize(s, "member")?;
        let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM file_assets f WHERE f.id=$1 AND f.school_id=$2 AND f.state='ready' AND ((f.owner_id=$3 AND ((NOT f.ever_bound AND f.expires_at>now() AND NOT EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=f.id)) OR EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=f.id AND l.owner_id=$3 AND l.expires_at>now()))) OR (f.purpose='post_image' AND EXISTS(SELECT 1 FROM posts p WHERE p.school_id=$2 AND p.images @> $4 AND p.visibility<>'deleted' AND (p.author_id=$3 OR $5 OR (p.audit_status='approved' AND p.visibility='visible')))) OR (f.purpose='feedback_evidence' AND EXISTS(SELECT 1 FROM cases c WHERE c.school_id=$2 AND c.kind='feedback' AND c.deleted_at IS NULL AND c.images @> $4 AND (c.account_id=$3 OR $6)))))")
            .bind(id).bind(s).bind(a.id).bind(json!([format!("/api/v1/schools/{s}/media/{id}")])).bind(a.has_permission(s,"content.post.read")).bind(a.has_permission(s,"content.case.read")).fetch_one(&self.0).await?;
        if !allowed {
            return Err(Error::NotFound);
        }
        self.read(id).await
    }
    pub async fn lease(&self, a: &Actor, s: Uuid, draft: Uuid, ids: Vec<Uuid>) -> Result<Value> {
        a.authorize(s, "member")?;
        require(ids.len() <= 9, "最多9张图片")?;
        let mut ids = ids;
        ids.sort();
        ids.dedup();
        let mut tx = super::lifecycle::begin(&self.0).await?;
        crate::miniapp_users::south::adapter::postgres::guard_write(&mut tx, a, s).await?;
        crate::schools::south::adapter::postgres::preview::authorize(&mut tx, a, Some(s), "member")
            .await?;
        for id in ids {
            sqlx::query("SELECT id FROM file_assets WHERE id=$1 AND school_id=$2 AND owner_id=$3 AND purpose='post_image' AND state='ready' FOR UPDATE")
                .bind(id).bind(s).bind(a.id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
            // A lock wait may have crossed the last reference's deletion. Check eligibility
            // in a new command so an old statement snapshot cannot resurrect the asset.
            let eligible: bool = sqlx::query_scalar("SELECT (NOT ever_bound AND expires_at>now()) OR EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=file_assets.id) OR EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=file_assets.id AND l.owner_id=$2 AND l.expires_at>now()) FROM file_assets WHERE id=$1")
                .bind(id).bind(a.id).fetch_one(&mut *tx).await?;
            if !eligible {
                return Err(Error::NotFound);
            }
            sqlx::query("INSERT INTO file_asset_leases VALUES($1,$2,$3,now()+make_interval(days=>$4)) ON CONFLICT(asset_id,owner_id,draft_id) DO UPDATE SET expires_at=excluded.expires_at").bind(id).bind(a.id).bind(draft).bind(self.1.config.lifecycle.draft_lease_days as i32).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(json!({"id":draft,"lease_days":self.1.config.lifecycle.draft_lease_days}))
    }
}
