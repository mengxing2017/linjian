use super::{Repository, preview};
use crate::{
    business::{Actor, Error, Result, require},
    schools::domain::home_menu::{OrderInput, TagInput},
};
use serde_json::{Value, json};
use uuid::Uuid;

async fn guard(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    a: &Actor,
    s: Uuid,
    role: &str,
) -> Result<()> {
    if role != "member" && !a.browser {
        return Err(Error::Forbidden);
    }
    preview::authorize(tx, a, Some(s), role).await?;
    sqlx::query("SELECT id FROM schools WHERE id=$1 AND active FOR SHARE")
        .bind(s)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::SchoolUnavailable)?;
    Ok(())
}
async fn lock_catalog(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, s: Uuid) -> Result<i64> {
    sqlx::query("INSERT INTO home_menu_settings(school_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(s)
        .execute(&mut **tx)
        .await?;
    Ok(sqlx::query_scalar(
        "SELECT catalog_revision FROM home_menu_settings WHERE school_id=$1 FOR UPDATE",
    )
    .bind(s)
    .fetch_one(&mut **tx)
    .await?)
}
async fn bump(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, s: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE home_menu_settings SET catalog_revision=catalog_revision+1 WHERE school_id=$1",
    )
    .bind(s)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
impl Repository {
    pub async fn home_menu_list(&self, a: &Actor, s: Uuid, public: bool) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, if public { "member" } else { "operator" }).await?;
        let settings: Value=sqlx::query_scalar("SELECT jsonb_build_object('latest_label',latest_label,'latest_visible',latest_visible,'revision',revision,'catalog_revision',catalog_revision) FROM home_menu_settings WHERE school_id=$1").bind(s).fetch_optional(&mut *tx).await?.unwrap_or(json!({"latest_label":"最新","latest_visible":true,"revision":1,"catalog_revision":1}));
        let tags: Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name,'menu_status',menu_status,'source_type',source_type,'enable_status',enable_status,'sort_order',sort_order,'revision',revision,'created_at',created_at) FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL AND (NOT $2 OR (source_type='admin' AND enable_status='enabled' AND menu_status IN ('shown','hot'))) ORDER BY sort_order,id").bind(s).bind(public).fetch_all(&mut *tx).await?;
        Ok(json!({"id":s,"settings":settings,"tags":tags}))
    }
    pub async fn home_menu_save(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        v: TagInput,
    ) -> Result<Value> {
        v.validate()?;
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "admin").await?;
        lock_catalog(&mut tx, s).await?;
        let menu = serde_json::to_value(v.menu_status)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let enabled = serde_json::to_value(v.enable_status)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let tag = id.unwrap_or_else(Uuid::now_v7);
        if id.is_none() {
            require(v.revision.is_none(), "新标签不能指定修订")?;
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL",
            )
            .bind(s)
            .fetch_one(&mut *tx)
            .await?;
            require(count < 500, "标签数量已达上限")?;
            sqlx::query("INSERT INTO home_menu_tags(id,school_id,name,menu_status,enable_status,sort_order) VALUES($1,$2,$3,$4,$5,coalesce((SELECT max(sort_order)+1 FROM home_menu_tags WHERE school_id=$2 AND deleted_at IS NULL),0))").bind(tag).bind(s).bind(v.name.trim()).bind(menu).bind(enabled).execute(&mut *tx).await?;
        } else {
            let n=sqlx::query("UPDATE home_menu_tags SET name=$3,menu_status=$4,enable_status=$5,revision=revision+1,updated_at=now() WHERE school_id=$1 AND id=$2 AND revision=$6 AND deleted_at IS NULL").bind(s).bind(tag).bind(v.name.trim()).bind(menu).bind(enabled).bind(v.revision).execute(&mut *tx).await?.rows_affected();
            if n != 1 {
                return Err(Error::Conflict);
            }
        }
        bump(&mut tx, s).await?;
        crate::audit::record(
            &mut tx,
            a,
            s,
            if id.is_some() {
                "home_menu_tag.update"
            } else {
                "home_menu_tag.create"
            },
            tag,
            json!({}),
        )
        .await?;
        let row:Value=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name,'menu_status',menu_status,'source_type',source_type,'enable_status',enable_status,'sort_order',sort_order,'revision',revision,'created_at',created_at) FROM home_menu_tags WHERE id=$1 AND school_id=$2").bind(tag).bind(s).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(row)
    }
    pub async fn home_menu_delete(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> Result<()> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "admin").await?;
        lock_catalog(&mut tx, s).await?;
        let n=sqlx::query("UPDATE home_menu_tags SET deleted_at=now(),updated_at=now(),revision=revision+1 WHERE school_id=$1 AND id=$2 AND revision=$3 AND deleted_at IS NULL").bind(s).bind(id).bind(revision).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(Error::Conflict);
        }
        bump(&mut tx, s).await?;
        crate::audit::record(&mut tx, a, s, "home_menu_tag.delete", id, json!({})).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn home_menu_order(&self, a: &Actor, s: Uuid, v: OrderInput) -> Result<()> {
        v.validate()?;
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "admin").await?;
        if lock_catalog(&mut tx, s).await? != v.catalog_revision {
            return Err(Error::Conflict);
        }
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL ORDER BY id",
        )
        .bind(s)
        .fetch_all(&mut *tx)
        .await?;
        let mut supplied = v.ids.clone();
        supplied.sort();
        if ids != supplied {
            return Err(Error::Conflict);
        }
        for (i, id) in v.ids.iter().enumerate() {
            sqlx::query("UPDATE home_menu_tags SET sort_order=$3,revision=revision+1,updated_at=now() WHERE school_id=$1 AND id=$2").bind(s).bind(id).bind(i as i64).execute(&mut *tx).await?;
        }
        bump(&mut tx, s).await?;
        crate::audit::record(
            &mut tx,
            a,
            s,
            "home_menu_tag.order",
            s,
            json!({"count":v.ids.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

impl Repository {
    pub async fn home_menu_settings_save(
        &self,
        a: &Actor,
        s: Uuid,
        v: crate::schools::domain::home_menu::SettingsInput,
    ) -> Result<Value> {
        v.validate()?;
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "admin").await?;
        lock_catalog(&mut tx, s).await?;
        let row: Option<Value> = sqlx::query_scalar("UPDATE home_menu_settings SET latest_label=$2,latest_visible=$3,revision=revision+1 WHERE school_id=$1 AND revision=$4 RETURNING jsonb_build_object('id',school_id,'latest_label',latest_label,'latest_visible',latest_visible,'revision',revision,'catalog_revision',catalog_revision)").bind(s).bind(v.latest_label.trim()).bind(v.latest_visible).bind(v.revision).fetch_optional(&mut *tx).await?;
        let row = row.ok_or(Error::Conflict)?;
        crate::audit::record(&mut tx, a, s, "home_menu.settings", s, json!({})).await?;
        tx.commit().await?;
        Ok(row)
    }
}

impl Repository {
    pub async fn home_menu_filtered(
        &self,
        a: &Actor,
        s: Uuid,
        q: &crate::schools::domain::home_menu::CatalogQuery,
        export: bool,
    ) -> Result<Value> {
        let (size, offset) = q.bounds()?;
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "operator").await?;
        let menu = q.menu_status.map(|x| {
            serde_json::to_value(x)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        });
        let source = q.source_type.map(|x| {
            serde_json::to_value(x)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        });
        let enabled = q.enable_status.map(|x| {
            serde_json::to_value(x)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        });
        let rows: Vec<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name,'menu_status',menu_status,'source_type',source_type,'enable_status',enable_status,'sort_order',sort_order,'revision',revision,'created_at',created_at) FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL AND ($2='' OR position(lower($2) in lower(name))>0) AND ($3::text IS NULL OR menu_status=$3) AND ($4::text IS NULL OR source_type=$4) AND ($5::text IS NULL OR enable_status=$5) ORDER BY sort_order,id LIMIT $6 OFFSET $7")
            .bind(s).bind(q.search.as_deref().unwrap_or("").trim()).bind(menu).bind(source).bind(enabled).bind(if export {501} else {size+1}).bind(if export {0} else {offset}).fetch_all(&mut *tx).await?;
        if export {
            require(rows.len() <= 500, "导出数量超过 500 条")?;
        }
        let more = !export && rows.len() > size as usize;
        Ok(
            json!({"id":s,"tags":rows.into_iter().take(if export {500} else {size as usize}).collect::<Vec<_>>(),"page":{"number":q.page.unwrap_or(1),"size":size,"has_more":more}}),
        )
    }
}

impl Repository {
    pub async fn home_menu_import_preview(
        &self,
        a: &Actor,
        s: Uuid,
        name: &str,
        bytes: &[u8],
    ) -> Result<Value> {
        // Authorize before parsing untrusted input, then recheck under the transaction lock.
        {
            let mut tx = self.0.begin().await?;
            guard(&mut tx, a, s, "admin").await?;
        }
        let (rows, mut errors) =
            crate::schools::south::adapter::home_menu_import_file::parse(name, bytes)?;
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "admin").await?;
        let revision = lock_catalog(&mut tx, s).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL",
        )
        .bind(s)
        .fetch_one(&mut *tx)
        .await?;
        if count + rows.len() as i64 > 500 {
            errors.push(json!({"row":0,"message":"导入后超过 500 条上限"}));
        }
        for row in &rows {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL AND lower(btrim(name))=lower(btrim($2)))").bind(s).bind(row["name"].as_str().unwrap()).fetch_one(&mut *tx).await?;
            if exists {
                errors.push(json!({"row":row["row"],"message":"本校已有同名标签"}));
            }
        }
        let id = Uuid::now_v7();
        let file_digest = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(bytes))
        };
        let digest = crate::business::hash(
            &json!([id, a.id, s, revision, file_digest, rows, errors]).to_string(),
        );
        let result:Value=sqlx::query_scalar("INSERT INTO home_menu_imports(id,school_id,actor_id,digest,catalog_revision,rows,errors,file_digest) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING jsonb_build_object('id',id,'digest',digest,'rows',rows,'errors',errors,'expires_at',expires_at,'result',result)").bind(id).bind(s).bind(a.id).bind(&digest).bind(revision).bind(json!(rows)).bind(json!(errors)).bind(&file_digest).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn home_menu_import_commit(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        digest: &str,
    ) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "admin").await?;
        let revision = lock_catalog(&mut tx, s).await?;
        let job:Value=sqlx::query_scalar("SELECT jsonb_build_object('digest',digest,'file_digest',file_digest,'catalog_revision',catalog_revision,'rows',rows,'errors',errors,'result',result,'valid',expires_at>now()) FROM home_menu_imports WHERE id=$1 AND school_id=$2 AND actor_id=$3 FOR UPDATE").bind(id).bind(s).bind(a.id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if job["digest"] != digest {
            return Err(Error::Conflict);
        }
        if !job["result"].is_null() {
            return Ok(json!({"id":id,"result":job["result"]}));
        }
        if job["catalog_revision"].as_i64() != Some(revision) {
            return Err(Error::Conflict);
        }
        require(job["valid"] == true, "导入预览已过期")?;
        let mut rows = job["rows"].as_array().cloned().ok_or(Error::Conflict)?;
        require(
            job["errors"].as_array().is_some_and(|v| v.is_empty())
                && !rows.is_empty()
                && rows.len() <= 500,
            "请修正导入错误",
        )?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM home_menu_tags WHERE school_id=$1 AND deleted_at IS NULL",
        )
        .bind(s)
        .fetch_one(&mut *tx)
        .await?;
        require(count + rows.len() as i64 <= 500, "导入后超过 500 条上限")?;
        rows.sort_by_key(|v| {
            (
                v["sort_order"].as_i64().unwrap_or(0),
                v["row"].as_u64().unwrap_or(0),
            )
        });
        let mut generated: Vec<_> = rows.iter().map(|_| Uuid::now_v7()).collect();
        generated.sort();
        let mut ids = Vec::new();
        for (row, tag) in rows.into_iter().zip(generated) {
            sqlx::query("INSERT INTO home_menu_tags(id,school_id,name,menu_status,enable_status,sort_order) VALUES($1,$2,$3,$4,$5,$6)").bind(tag).bind(s).bind(row["name"].as_str()).bind(row["menu_status"].as_str()).bind(row["enable_status"].as_str()).bind(row["sort_order"].as_i64()).execute(&mut *tx).await?;
            ids.push(tag);
        }
        bump(&mut tx, s).await?;
        sqlx::query("UPDATE home_menu_imports SET result=$2,committed_at=now(),rows='[]',errors='[]' WHERE id=$1").bind(id).bind(json!(ids)).execute(&mut *tx).await?;
        crate::audit::record(
            &mut tx,
            a,
            s,
            "home_menu_tag.import",
            id,
            json!({"count":ids.len(),"file_digest":job["file_digest"]}),
        )
        .await?;
        tx.commit().await?;
        Ok(json!({"id":id,"result":ids}))
    }
}

impl Repository {
    pub async fn home_menu_settings(&self, a: &Actor, s: Uuid) -> Result<Value> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, "operator").await?;
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',school_id,'latest_label',latest_label,'latest_visible',latest_visible,'revision',revision,'catalog_revision',catalog_revision) FROM home_menu_settings WHERE school_id=$1")
            .bind(s).fetch_optional(&mut *tx).await?.unwrap_or(json!({"id":s,"latest_label":"最新","latest_visible":true,"revision":1,"catalog_revision":1})))
    }
}

impl crate::schools::south::port::home_menu::Store for Repository {
    async fn list(&self, a: &Actor, s: Uuid, public: bool) -> Result<Value> {
        self.home_menu_list(a, s, public).await
    }
    async fn filtered(
        &self,
        a: &Actor,
        s: Uuid,
        q: &crate::schools::domain::home_menu::CatalogQuery,
        export: bool,
    ) -> Result<Value> {
        self.home_menu_filtered(a, s, q, export).await
    }
    async fn save(&self, a: &Actor, s: Uuid, id: Option<Uuid>, v: TagInput) -> Result<Value> {
        self.home_menu_save(a, s, id, v).await
    }
    async fn delete(&self, a: &Actor, s: Uuid, id: Uuid, revision: i64) -> Result<()> {
        self.home_menu_delete(a, s, id, revision).await
    }
    async fn order(&self, a: &Actor, s: Uuid, v: OrderInput) -> Result<()> {
        self.home_menu_order(a, s, v).await
    }
    async fn settings(&self, a: &Actor, s: Uuid) -> Result<Value> {
        self.home_menu_settings(a, s).await
    }
    async fn save_settings(
        &self,
        a: &Actor,
        s: Uuid,
        v: crate::schools::domain::home_menu::SettingsInput,
    ) -> Result<Value> {
        self.home_menu_settings_save(a, s, v).await
    }
    async fn import_preview(&self, a: &Actor, s: Uuid, name: &str, bytes: &[u8]) -> Result<Value> {
        self.home_menu_import_preview(a, s, name, bytes).await
    }
    async fn import_commit(&self, a: &Actor, s: Uuid, id: Uuid, digest: &str) -> Result<Value> {
        self.home_menu_import_commit(a, s, id, digest).await
    }
}
