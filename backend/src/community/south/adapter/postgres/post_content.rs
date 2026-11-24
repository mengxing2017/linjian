//! Shared content writes for the forum, management and atomic imports.
use super::decisions;
use crate::{
    business::{Actor, Error, Result, require},
    community::domain::{PostInput, post_tags},
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
pub(super) async fn validate(
    tx: &mut Transaction<'_, Postgres>,
    s: Uuid,
    a: &Actor,
    current_post: Option<Uuid>,
    v: &PostInput,
) -> Result<()> {
    validate_content(tx, s, a, current_post, v)
        .await
        .map(|_| ())
}
async fn validate_content(
    tx: &mut Transaction<'_, Postgres>,
    school: Uuid,
    actor: &Actor,
    current_post: Option<Uuid>,
    input: &PostInput,
) -> Result<Vec<String>> {
    input.validate(school)?;
    let mut selected = std::collections::HashSet::new();
    let mut candidate_names = Vec::new();
    for id in &input.candidate_tag_ids {
        require(selected.insert(id), "候选标签重复")?;
        let name: String = sqlx::query_scalar("SELECT name FROM home_menu_tags WHERE school_id=$1 AND id=$2 AND deleted_at IS NULL AND source_type='admin' AND enable_status='enabled' AND menu_status IN ('shown','hot') FOR SHARE")
            .bind(school).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(|| Error::Invalid("候选标签已失效，请重新选择".into()))?;
        require(
            input.tags.iter().any(|tag| tag.trim() == name),
            "候选标签与提交名称不一致",
        )?;
        candidate_names.push(name);
    }
    for path in &input.images {
        let id = path
            .rsplit('/')
            .next()
            .and_then(|v| v.parse::<Uuid>().ok())
            .ok_or_else(|| Error::Invalid("图片地址无效".into()))?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM file_assets f WHERE f.id=$2 AND f.school_id=$1 AND f.purpose='post_image' AND f.state='ready' AND (f.owner_id=$3 OR EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=f.id AND r.object_type='posts' AND r.object_id=$4)) AND ((NOT f.ever_bound AND f.expires_at>now()) OR EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=f.id) OR EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=f.id AND l.owner_id=$3 AND l.expires_at>now())))"
        ).bind(school).bind(id).bind(actor.id).bind(current_post).fetch_one(&mut **tx).await?;
        require(exists, "图片不存在或属于其他学校")?;
    }
    let words: Vec<String> = sqlx::query_scalar(
        "SELECT word FROM blocked_words WHERE (school_id=$1 OR school_id IS NULL) AND active",
    )
    .bind(school)
    .fetch_all(&mut **tx)
    .await?;
    let content = format!("{} {}", input.title, input.content).to_lowercase();
    require(
        !words
            .iter()
            .any(|word| content.contains(&word.to_lowercase())),
        "内容包含禁止发布的词语",
    )?;
    Ok(candidate_names)
}
pub(super) async fn create(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Uuid,
    v: PostInput,
) -> Result<Value> {
    let v = v.normalized()?;
    let candidates = validate_content(tx, s, a, None, &v).await?;
    if !a.is_admin_browser() {
        post_tags::validate_selection(&v.tags, &candidates, &[])?;
    }
    let policy = decisions::policy(tx, s, false).await?;
    let id = Uuid::now_v7();
    let item=sqlx::query_scalar("INSERT INTO posts(id,school_id,author_id,title,content,images,tags,audit_status,created_via,source_projection) VALUES($1,$2,$3,$4,$5,$6,$7,'pending',$8,$9) RETURNING to_jsonb(posts)")
 .bind(id).bind(s).bind(a.id).bind(v.title.trim()).bind(v.content.trim()).bind(json!(v.images)).bind(json!(v.tags))
 .bind(if a.is_admin_browser() {"web"} else {"miniapp"}).bind("message_v1").fetch_one(&mut **tx).await?;
    let item = decisions::automatically_approve(tx, item, policy).await?;
    crate::audit::record(
        tx,
        a,
        s,
        "post.create",
        id,
        json!({"audit_status":item["audit_status"]}),
    )
    .await?;
    Ok(item)
}
pub(super) async fn update(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Uuid,
    id: Uuid,
    v: PostInput,
) -> Result<Value> {
    let v = v.normalized()?;
    let current:Value=sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE school_id=$1 AND id=$2 AND revision=$3 AND visibility<>'deleted' AND (author_id=$4 OR $5) FOR UPDATE")
 .bind(s).bind(id).bind(v.revision.unwrap_or(0)).bind(a.id).bind(a.has_permission(s,"content.post.edit")).fetch_optional(&mut **tx).await?.ok_or(Error::Conflict)?;
    let candidates = validate_content(tx, s, a, Some(id), &v).await?;
    if !a.is_admin_browser() {
        let retained: Vec<String> = serde_json::from_value(current["tags"].clone())
            .map_err(|error| Error::Database(error.to_string()))?;
        post_tags::validate_selection(&v.tags, &candidates, &retained)?;
    }
    if current["title"] == v.title.trim()
        && current["content"] == v.content.trim()
        && current["images"] == json!(v.images)
        && current["tags"] == json!(v.tags)
    {
        return Ok(current);
    }
    let policy = decisions::policy(tx, s, false).await?;
    let item=sqlx::query_scalar("UPDATE posts SET title=$3,content=$4,images=$5,tags=$6,audit_status='pending',revision=revision+1,updated_at=now() WHERE school_id=$1 AND id=$2 RETURNING to_jsonb(posts)")
 .bind(s).bind(id).bind(v.title.trim()).bind(v.content.trim()).bind(json!(v.images)).bind(json!(v.tags)).fetch_one(&mut **tx).await?;
    let item = decisions::automatically_approve(tx, item, policy).await?;
    crate::audit::record(
        tx,
        a,
        s,
        "post.edit",
        id,
        json!({"previous_revision":current["revision"],"revision":item["revision"]}),
    )
    .await?;
    Ok(item)
}
