use crate::business::{Actor, Error, Result, hash, secret};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
impl Repository {
    pub async fn alias(&self, kind: &str, id: Uuid) -> Result<i64> {
        let old: Option<String> = sqlx::query_scalar(
            "SELECT legacy_id FROM legacy_id_map WHERE source_table=$1 AND target_id=$2",
        )
        .bind(kind)
        .bind(id)
        .fetch_optional(&self.0)
        .await?;
        if let Some(old) = old.and_then(|s| s.parse::<i64>().ok()) {
            return Ok(old);
        }
        Ok(sqlx::query_scalar("INSERT INTO legacy_aliases(kind,target_id) VALUES($1,$2) ON CONFLICT(kind,target_id) DO UPDATE SET kind=excluded.kind RETURNING public_id").bind(kind).bind(id).fetch_one(&self.0).await?)
    }
    pub async fn resolve(&self, kind: &str, value: &str) -> Result<Uuid> {
        if let Ok(id) = value.parse::<Uuid>() {
            return Ok(id);
        }
        let id=sqlx::query_scalar("SELECT target_id FROM legacy_id_map WHERE source_table=$1 AND legacy_id=$2 UNION ALL SELECT target_id FROM legacy_aliases WHERE kind=$1 AND public_id::text=$2 LIMIT 1").bind(kind).bind(value).fetch_optional(&self.0).await?;
        id.ok_or(Error::NotFound)
    }
    pub async fn counts(&self, a: &Actor, s: Uuid) -> Result<Value> {
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('today_count',(SELECT count(*) FROM posts WHERE school_id=$1 AND audit_status='approved' AND visibility='visible' AND created_at>=date_trunc('day',now() AT TIME ZONE 'Asia/Shanghai') AT TIME ZONE 'Asia/Shanghai'),'member_count',(SELECT count(*) FROM school_memberships WHERE school_id=$1),'msgCount',(SELECT count(*) FROM notifications WHERE school_id=$1 AND account_id=$2 AND read_at IS NULL),'attentionCount',(SELECT count(*) FROM follows WHERE school_id=$1 AND follower_id=$2),'beAttentionCount',(SELECT count(*) FROM follows WHERE school_id=$1 AND target_id=$2),'favorCount',(SELECT count(*) FROM post_reactions r JOIN posts p ON p.id=r.post_id WHERE r.school_id=$1 AND p.author_id=$2 AND r.kind='like'),'discoverCount',(SELECT count(*) FROM posts WHERE school_id=$1 AND author_id=$2 AND visibility<>'deleted'))").bind(s).bind(a.id).fetch_one(&self.0).await?)
    }
    pub async fn signed_media(&self, s: Uuid, path: &str, origin: &str) -> Result<String> {
        let prefix = format!("/api/v1/schools/{s}/media/");
        if !path.starts_with(&prefix) {
            return Ok(path.into());
        }
        let id = path
            .strip_prefix(&prefix)
            .and_then(|x| x.parse::<Uuid>().ok())
            .ok_or(Error::NotFound)?;
        let token = secret();
        let mut tx = crate::media::south::adapter::lifecycle::begin(&self.0).await?;
        // Business object first, then asset: match post writers and file cleanup.
        let post: Uuid = sqlx::query_scalar("SELECT p.id FROM posts p JOIN schools s ON s.id=p.school_id WHERE p.school_id=$1 AND s.active AND p.audit_status='approved' AND p.visibility='visible' AND p.images @> jsonb_build_array($2::text) ORDER BY p.id LIMIT 1 FOR SHARE OF p")
            .bind(s).bind(path).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        sqlx::query("SELECT id FROM file_assets WHERE id=$1 AND school_id=$2 AND state='ready' AND purpose='post_image' FOR UPDATE")
            .bind(id).bind(s).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        sqlx::query("INSERT INTO media_access_tokens(token_hash,school_id,asset_id,expires_at,post_id) VALUES($1,$2,$3,now()+interval '15 minutes',$4)")
            .bind(hash(&token)).bind(s).bind(id).bind(post).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(format!("{origin}/api/legacy/media/{token}"))
    }
    pub async fn media(
        &self,
        token: &str,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> Result<(String, Vec<u8>)> {
        let id:Uuid=sqlx::query_scalar("SELECT a.id FROM media_access_tokens t JOIN media_assets a ON a.id=t.asset_id AND a.school_id=t.school_id JOIN posts p ON p.id=t.post_id AND p.school_id=t.school_id JOIN schools s ON s.id=t.school_id WHERE t.token_hash=$1 AND t.expires_at>now() AND s.active AND p.audit_status='approved' AND p.visibility='visible' AND p.images @> jsonb_build_array('/api/v1/schools/'||a.school_id||'/media/'||a.id)").bind(hash(token)).fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        crate::media::south::adapter::postgres::Repository(self.0.clone(), files)
            .read(id)
            .await
    }
    pub async fn public(&self, s: Uuid) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT active AND coalesce((config->>'public_read')::boolean,false) FROM schools WHERE id=$1").bind(s).fetch_optional(&self.0).await?.unwrap_or(false))
    }
    pub async fn comment_like(&self, a: &Actor, s: Uuid, id: Uuid) -> Result<Value> {
        let mut tx = crate::media::south::adapter::lifecycle::begin(&self.0).await?;
        sqlx::query("SELECT c.id FROM comments c JOIN posts p ON p.id=c.post_id WHERE c.school_id=$1 AND c.id=$2 AND c.audit_status='approved' AND c.visibility='visible' AND NOT c.deleted AND p.audit_status='approved' AND p.visibility='visible' FOR UPDATE OF c").bind(s).bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        let removed = sqlx::query(
            "DELETE FROM comment_reactions WHERE school_id=$1 AND comment_id=$2 AND account_id=$3",
        )
        .bind(s)
        .bind(id)
        .bind(a.id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if removed == 0 {
            sqlx::query("INSERT INTO comment_reactions VALUES($1,$2,$3)")
                .bind(s)
                .bind(id)
                .bind(a.id)
                .execute(&mut *tx)
                .await?;
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM comment_reactions WHERE comment_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(json!({"favorNum":count}))
    }
    pub async fn fans(&self, s: Uuid, id: Uuid) -> Result<Vec<Value>> {
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',a.id,'display_name',a.display_name) FROM follows f JOIN accounts a ON a.id=f.follower_id WHERE f.school_id=$1 AND f.target_id=$2").bind(s).bind(id).fetch_all(&self.0).await?)
    }
    pub async fn received_likes(&self, s: Uuid, id: Uuid) -> Result<Vec<Value>> {
        Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',a.id,'display_name',a.display_name,'post_id',p.id,'title',p.title) FROM post_reactions r JOIN posts p ON p.id=r.post_id JOIN accounts a ON a.id=r.account_id WHERE r.school_id=$1 AND p.author_id=$2 AND r.kind='like' ORDER BY p.created_at DESC LIMIT 100").bind(s).bind(id).fetch_all(&self.0).await?)
    }
}
