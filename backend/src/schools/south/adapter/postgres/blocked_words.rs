use super::{Repository, preview};
use crate::{
    business::{Actor, Error, Result, require},
    schools::domain::blocked_words::{OrderInput, WordInput, WordQuery},
};
use serde_json::{Value, json};
use uuid::Uuid;

fn scope(s: Option<Uuid>) -> String {
    s.map(|s| s.to_string()).unwrap_or_else(|| "global".into())
}
impl crate::schools::south::port::blocked_words::Store for Repository {
    async fn list(&self, a: &Actor, s: Option<Uuid>, q: &WordQuery) -> Result<Value> {
        self.blocked_list(a, s, q).await
    }
    async fn save(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Option<Uuid>,
        v: WordInput,
    ) -> Result<Value> {
        self.blocked_save(a, s, id, v).await
    }
    async fn delete(&self, a: &Actor, s: Option<Uuid>, id: Uuid, r: i64) -> Result<()> {
        self.blocked_delete(a, s, id, r).await
    }
    async fn order(&self, a: &Actor, s: Option<Uuid>, v: OrderInput) -> Result<()> {
        self.blocked_order(a, s, v).await
    }
    async fn import_preview(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        name: &str,
        bytes: &[u8],
    ) -> Result<Value> {
        self.blocked_import_preview(a, s, name, bytes).await
    }
    async fn import_commit(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        digest: &str,
    ) -> Result<Value> {
        self.blocked_import_commit(a, s, id, digest).await
    }
}
async fn lock(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, s: Option<Uuid>) -> Result<i64> {
    let key = scope(s);
    sqlx::query("INSERT INTO blocked_word_order_versions(scope) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(&key)
        .execute(&mut **tx)
        .await?;
    Ok(sqlx::query_scalar(
        "SELECT revision FROM blocked_word_order_versions WHERE scope=$1 FOR UPDATE",
    )
    .bind(&key)
    .fetch_one(&mut **tx)
    .await?)
}
async fn bump(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, s: Option<Uuid>) -> Result<()> {
    sqlx::query("UPDATE blocked_word_order_versions SET revision=revision+1 WHERE scope=$1")
        .bind(scope(s))
        .execute(&mut **tx)
        .await?;
    Ok(())
}
impl Repository {
    pub async fn blocked_import_preview(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        name: &str,
        bytes: &[u8],
    ) -> Result<Value> {
        if let Some(s) = s {
            a.authorize(s, "admin")?
        } else {
            a.platform()?
        }
        let (rows, mut errors) =
            crate::schools::south::adapter::blocked_word_import_file::parse(name, bytes)?;
        let mut tx = self.0.begin().await?;
        preview::authorize(&mut tx, a, s, "admin").await?;
        let revision = lock(&mut tx, s).await?;
        for row in &rows {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM blocked_words WHERE school_id IS NOT DISTINCT FROM $1 AND lower(word)=lower($2))").bind(s).bind(row["word"].as_str().unwrap_or("")).fetch_one(&mut *tx).await?;
            if exists {
                errors.push(json!({"row":row["row"],"message":"同范围屏蔽词已存在"}));
            }
        }
        let id = Uuid::now_v7();
        let digest=crate::business::hash(&json!({"id":id,"actor":a.id,"scope":s,"revision":revision,"rows":rows,"errors":errors}).to_string());
        let value:Value=sqlx::query_scalar("INSERT INTO blocked_word_imports(id,school_id,actor_id,digest,order_revision,rows,errors) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING jsonb_build_object('id',id,'digest',digest,'rows',rows,'errors',errors,'expires_at',expires_at)").bind(id).bind(s).bind(a.id).bind(&digest).bind(revision).bind(json!(rows)).bind(json!(errors)).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(value)
    }
    pub async fn blocked_import_commit(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        digest: &str,
    ) -> Result<Value> {
        if let Some(s) = s {
            a.authorize(s, "admin")?
        } else {
            a.platform()?
        }
        let mut tx = self.0.begin().await?;
        preview::authorize(&mut tx, a, s, "admin").await?;
        let revision = lock(&mut tx, s).await?;
        let job:Value=sqlx::query_scalar("SELECT jsonb_build_object('digest',digest,'revision',order_revision,'rows',rows,'errors',errors,'valid',expires_at>now(),'result',result) FROM blocked_word_imports WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 AND actor_id=$3 FOR UPDATE").bind(id).bind(s).bind(a.id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        require(job["digest"].as_str() == Some(digest), "导入摘要不匹配")?;
        if !job["result"].is_null() {
            return Ok(job["result"].clone());
        }
        require(
            job["valid"] == true && job["revision"].as_i64() == Some(revision),
            "导入预检已过期或词条已变化，请重新预检",
        )?;
        require(
            job["errors"].as_array().is_some_and(|v| v.is_empty()),
            "导入存在错误，请修正后重试",
        )?;
        let rows = job["rows"].as_array().ok_or(Error::Conflict)?;
        for row in rows {
            sqlx::query("INSERT INTO blocked_words(id,school_id,word,remark,active,sort_order) VALUES($1,$2,$3,$4,$5,coalesce((SELECT max(sort_order)+1 FROM blocked_words WHERE school_id IS NOT DISTINCT FROM $2),0))").bind(Uuid::now_v7()).bind(s).bind(row["word"].as_str().unwrap_or("")).bind(row["remark"].as_str().unwrap_or("")).bind(row["active"].as_bool().unwrap_or(false)).execute(&mut *tx).await?;
        }
        bump(&mut tx, s).await?;
        let result = json!({"id":id,"count":rows.len()});
        sqlx::query("UPDATE blocked_word_imports SET result=$2,rows='[]' WHERE id=$1")
            .bind(id)
            .bind(&result)
            .execute(&mut *tx)
            .await?;
        crate::audit::record(
            &mut tx,
            a,
            s.unwrap_or_default(),
            "blocked_word.import",
            id,
            json!({"count":rows.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn blocked_list(&self, a: &Actor, s: Option<Uuid>, q: &WordQuery) -> Result<Value> {
        if let Some(s) = s {
            a.authorize(s, "operator")?;
        } else {
            a.platform()?;
        }
        let mut tx = self.0.begin().await?;
        preview::authorize(&mut tx, a, s, "operator").await?;
        let actual = if q.global.unwrap_or(false) { None } else { s };
        let rev = lock(&mut tx, actual).await?;
        let page = q.page.unwrap_or(1);
        require((1..=100000).contains(&page), "分页参数无效")?;
        let active = match q.status.as_deref() {
            None | Some("") => None,
            Some("active") => Some(true),
            Some("inactive") => Some(false),
            _ => return Err(Error::Invalid("状态无效".into())),
        };
        let search = q.search.as_deref().unwrap_or("");
        require(search.chars().count() <= 255, "查询词过长")?;
        let ordering = q.ordering.unwrap_or(false);
        require(
            !ordering || (search.is_empty() && active.is_none()),
            "排序时不能筛选",
        )?;
        let limit = if ordering { 5001 } else { 11 };
        let offset = if ordering { 0 } else { (page - 1) * 10 };
        let mut rows:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(w) FROM blocked_words w WHERE school_id IS NOT DISTINCT FROM $1 AND ($2='' OR strpos(lower(word),lower($2))>0 OR strpos(lower(remark),lower($2))>0) AND ($3::boolean IS NULL OR active=$3) ORDER BY sort_order DESC,id LIMIT $4 OFFSET $5").bind(actual).bind(search).bind(active).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
        if ordering {
            require(rows.len() <= 5000, "词条超过 5000 条，无法排序")?;
        }
        let more = !ordering && rows.len() > 10;
        if more {
            rows.truncate(10);
        }
        tx.commit().await?;
        Ok(json!({"id":scope(actual),"items":rows,"order_revision":rev,"has_more":more}))
    }
    pub async fn blocked_save(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Option<Uuid>,
        mut v: WordInput,
    ) -> Result<Value> {
        if let Some(s) = s {
            a.authorize(s, "admin")?
        } else {
            a.platform()?
        }
        v.validate()?;
        require(
            (id.is_none() && v.revision == 0) || (id.is_some() && v.revision > 0),
            "修订号无效",
        )?;
        let mut tx = self.0.begin().await?;
        preview::authorize(&mut tx, a, s, "admin").await?;
        lock(&mut tx, s).await?;
        let id = id.unwrap_or_else(Uuid::now_v7);
        let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM blocked_words WHERE school_id IS NOT DISTINCT FROM $1 AND lower(word)=lower($2) AND id<>$3)")
            .bind(s).bind(&v.word).bind(id).fetch_one(&mut *tx).await?;
        require(!duplicate, "该屏蔽词已存在，请勿重复添加")?;
        let row: Value = if v.revision == 0 {
            sqlx::query_scalar("INSERT INTO blocked_words(id,school_id,word,remark,active,sort_order) VALUES($1,$2,$3,$4,$5,coalesce((SELECT max(sort_order)+1 FROM blocked_words WHERE school_id IS NOT DISTINCT FROM $2),0)) RETURNING to_jsonb(blocked_words)").bind(id).bind(s).bind(&v.word).bind(&v.remark).bind(v.active).fetch_one(&mut *tx).await?
        } else {
            sqlx::query_scalar("UPDATE blocked_words SET word=$3,remark=$4,active=$5,revision=revision+1,updated_at=now() WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 AND revision=$6 RETURNING to_jsonb(blocked_words)").bind(id).bind(s).bind(&v.word).bind(&v.remark).bind(v.active).bind(v.revision).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?
        };
        bump(&mut tx, s).await?;
        crate::audit::record(
            &mut tx,
            a,
            s.unwrap_or_default(),
            "blocked_word.save",
            id,
            json!({"word":v.word,"active":v.active}),
        )
        .await?;
        tx.commit().await?;
        Ok(row)
    }
    pub async fn blocked_delete(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        revision: i64,
    ) -> Result<()> {
        if let Some(s) = s {
            a.authorize(s, "admin")?
        } else {
            a.platform()?
        }
        let mut tx = self.0.begin().await?;
        preview::authorize(&mut tx, a, s, "admin").await?;
        lock(&mut tx, s).await?;
        let n=sqlx::query("DELETE FROM blocked_words WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 AND revision=$3").bind(id).bind(s).bind(revision).execute(&mut *tx).await?.rows_affected();
        if n != 1 {
            return Err(Error::Conflict);
        }
        bump(&mut tx, s).await?;
        crate::audit::record(
            &mut tx,
            a,
            s.unwrap_or_default(),
            "blocked_word.delete",
            id,
            json!({}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn blocked_order(&self, a: &Actor, s: Option<Uuid>, v: OrderInput) -> Result<()> {
        if let Some(s) = s {
            a.authorize(s, "admin")?
        } else {
            a.platform()?
        }
        let mut tx = self.0.begin().await?;
        preview::authorize(&mut tx, a, s, "admin").await?;
        if lock(&mut tx, s).await? != v.order_revision {
            return Err(Error::Conflict);
        }
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM blocked_words WHERE school_id IS NOT DISTINCT FROM $1 ORDER BY id",
        )
        .bind(s)
        .fetch_all(&mut *tx)
        .await?;
        let mut requested = v.ids.clone();
        requested.sort();
        require(
            ids == requested && ids.len() <= 5000,
            "排序列表已变化或超过上限，请刷新",
        )?;
        for (i, id) in v.ids.iter().enumerate() {
            sqlx::query("UPDATE blocked_words SET sort_order=$2 WHERE id=$1")
                .bind(id)
                .bind((v.ids.len() - i) as i32)
                .execute(&mut *tx)
                .await?;
        }
        bump(&mut tx, s).await?;
        crate::audit::record(
            &mut tx,
            a,
            s.unwrap_or_default(),
            "blocked_word.order",
            Uuid::nil(),
            json!({"count":ids.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
