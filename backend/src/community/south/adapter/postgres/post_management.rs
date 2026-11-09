use super::{Repository, post_content};
use crate::{
    business::{Actor, Error, Result, require, text},
    community::domain::{PostInput, post_management::*, review},
};
use serde_json::{Value, json};
use sqlx::{PgConnection, Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) async fn live_operator(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Uuid,
) -> Result<()> {
    a.authorize(s, "operator")?;
    let account = sqlx::query("SELECT active,platform_admin FROM accounts WHERE id=$1 FOR SHARE")
        .bind(a.id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::Forbidden)?;
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM school_memberships WHERE account_id=$1 AND school_id=$2 FOR SHARE",
    )
    .bind(a.id)
    .bind(s)
    .fetch_optional(&mut **tx)
    .await?;
    if !account.get::<bool, _>("active")
        || !(account.get::<bool, _>("platform_admin")
            || role.is_some_and(|r| matches!(r.as_str(), "operator" | "admin")))
    {
        return Err(Error::Forbidden);
    }
    Ok(())
}
pub(super) async fn guard(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    s: Uuid,
    platform: bool,
) -> Result<()> {
    live_operator(tx, a, s).await?;
    if platform {
        a.platform()?;
        crate::schools::south::adapter::postgres::preview::platform(tx, a).await?;
    }
    crate::schools::south::adapter::postgres::profiles::lock_active_school(tx, s).await
}
const VIEW: &str = "jsonb_build_object('id',p.id,'school_id',p.school_id,'school_name',s.name,'author_id',p.author_id,'author_nickname',coalesce(nullif(btrim(u.nickname),''),nullif(btrim(a.display_name),''),'未知用户'),'title',p.title,'content',p.content,'images',p.images,'tags',p.tags,'created_at',p.created_at,'audit_status',p.audit_status,'visibility',p.visibility,'top_sort',p.top_sort,'revision',p.revision,'content_revision',p.content_revision,'is_top',p.is_top,'cover_image',p.images->>0)||forum_decision_details(p.school_id,p.id,'post',p.content_revision,p.audit_status)";
const JOIN: &str = " FROM posts p JOIN schools s ON s.id=p.school_id LEFT JOIN accounts a ON a.id=p.author_id LEFT JOIN miniapp_users u ON u.account_id=p.author_id ";
fn decode(mut row: Value) -> Result<PostManagementView> {
    row["review_digest"] = review::digest(&row)?.into();
    let is_top = row["is_top"].as_bool().ok_or(Error::Unavailable)?;
    let cover_image = row["cover_image"].as_str().map(str::to_owned);
    row.as_object_mut()
        .ok_or(Error::Unavailable)?
        .remove("is_top");
    row.as_object_mut().unwrap().remove("cover_image");
    Ok(PostManagementView {
        post: serde_json::from_value(row).map_err(|_| Error::Unavailable)?,
        is_top,
        cover_image,
    })
}
pub(super) async fn detail(
    conn: &mut PgConnection,
    s: Uuid,
    id: Uuid,
) -> Result<PostManagementView> {
    let row: Value = sqlx::query_scalar(&format!(
        "SELECT {VIEW}{JOIN} WHERE p.school_id=$1 AND p.id=$2 AND p.visibility<>'deleted'"
    ))
    .bind(s)
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(Error::NotFound)?;
    decode(row)
}
pub(super) async fn locked(
    tx: &mut Transaction<'_, Postgres>,
    s: Uuid,
    id: Uuid,
    revision: i64,
) -> Result<Value> {
    require(revision > 0, "版本号无效")?;
    let current:Value=sqlx::query_scalar("SELECT to_jsonb(p) FROM posts p WHERE school_id=$1 AND id=$2 AND visibility<>'deleted' FOR UPDATE")
 .bind(s).bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    if current["revision"].as_i64() != Some(revision) {
        return Err(Error::Conflict);
    }
    Ok(current)
}
pub(super) async fn change(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    target: (Uuid, Uuid),
    revision: i64,
    status: Option<&str>,
    pin: Option<bool>,
    reason: &str,
) -> Result<Value> {
    let (s, id) = target;
    let previous = locked(tx, s, id, revision).await?;
    let status = status.unwrap_or(previous["visibility"].as_str().ok_or(Error::Unavailable)?);
    let pin = if status == "deleted" {
        false
    } else {
        pin.unwrap_or(previous["is_top"].as_bool().unwrap_or(false))
    };
    if previous["visibility"] == status && previous["is_top"] == pin {
        return Ok(previous);
    }
    let current:Value=sqlx::query_scalar("UPDATE posts SET visibility=$3,is_top=$4,top_sort=CASE WHEN $4 THEN greatest(top_sort,1) ELSE 0 END,revision=revision+1,updated_at=now() WHERE school_id=$1 AND id=$2 RETURNING to_jsonb(posts)")
 .bind(s).bind(id).bind(status).bind(pin).fetch_one(&mut **tx).await?;
    let action = if status == "deleted" {
        "post.delete"
    } else if previous["visibility"] != status {
        "post.status"
    } else {
        "post.pin"
    };
    crate::audit::record(tx,a,s,action,id,json!({"previous_visibility":previous["visibility"],"visibility":status,"previous_is_top":previous["is_top"],"is_top":pin,"previous_revision":revision,"revision":current["revision"],"reason":reason})).await?;
    Ok(current)
}
impl Repository {
    pub async fn management_list(
        &self,
        a: &Actor,
        s: Uuid,
        f: PostManagementFilters,
    ) -> Result<PostManagementPage> {
        f.validate()?;
        let mut tx = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;
        // A single snapshot binds the ordering token to exactly the returned page.
        a.authorize(s, "operator")?;
        let allowed: bool = sqlx::query_scalar(super::reviews::AUTHORIZATION)
            .bind(a.id)
            .bind(s)
            .fetch_one(&mut *tx)
            .await?;
        if !allowed {
            return Err(Error::Forbidden);
        }
        let active: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM schools WHERE id=$1 AND active)")
                .bind(s)
                .fetch_one(&mut *tx)
                .await?;
        if !active {
            return Err(Error::SchoolUnavailable);
        }
        let order_revision = sqlx::query_scalar(
            "SELECT coalesce((SELECT revision FROM post_order_versions WHERE school_id=$1),1)",
        )
        .bind(s)
        .fetch_one(&mut *tx)
        .await?;
        let (limit, offset) = f.pagination().fetch_bounds()?;
        let rows:Vec<Value>=sqlx::query_scalar(&format!("SELECT {VIEW}{JOIN} WHERE p.school_id=$1 AND p.visibility<>'deleted' AND ($2::text IS NULL OR $2='' OR p.audit_status=$2) AND ($3::text IS NULL OR $3='' OR p.visibility=$3) AND ($4::boolean IS NULL OR p.is_top=$4) AND (p.title ILIKE '%'||$5||'%' OR p.content ILIKE '%'||$5||'%' OR coalesce(nullif(btrim(u.nickname),''),a.display_name,'未知用户') ILIKE '%'||$5||'%') AND ($6::text IS NULL OR $6='' OR p.tags ? $6) AND ($7::timestamptz IS NULL OR p.created_at>=$7) AND ($8::timestamptz IS NULL OR p.created_at<=$8) ORDER BY p.is_top DESC,p.sort_order DESC,p.created_at DESC,p.id LIMIT $9 OFFSET $10"))
  .bind(s).bind(f.audit_status).bind(f.visibility).bind(f.is_top).bind(f.search.unwrap_or_default()).bind(f.tag).bind(f.created_from).bind(f.created_to).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
        let items = rows.into_iter().map(decode).collect::<Result<_>>()?;
        tx.commit().await?;
        Ok(PostManagementPage {
            items,
            order_revision,
        })
    }
    pub async fn management_detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> Result<PostManagementView> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, false).await?;
        let item = detail(&mut tx, s, id).await?;
        tx.commit().await?;
        Ok(item)
    }
    pub async fn management_write(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        v: PostInput,
    ) -> Result<PostManagementView> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, id.is_none()).await?;
        let row = if let Some(id) = id {
            require(v.revision.is_some_and(|v| v > 0), "版本号无效")?;
            post_content::update(&mut tx, a, s, id, v).await?
        } else {
            require(v.revision.is_none(), "新增不接受版本号")?;
            post_content::create(&mut tx, a, s, v).await?
        };
        let item = detail(&mut tx, s, crate::business::uuid(&row, "id")?).await?;
        tx.commit().await?;
        Ok(item)
    }
    pub async fn management_pin(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: PinPostInput,
    ) -> Result<PostManagementView> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, false).await?;
        change(&mut tx, a, (s, id), v.revision, None, Some(v.is_top), "").await?;
        let item = detail(&mut tx, s, id).await?;
        tx.commit().await?;
        Ok(item)
    }
    pub async fn management_status(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: SetPostStatusInput,
    ) -> Result<PostManagementView> {
        let reason = text(
            &v.reason,
            if matches!(v.visibility, PostVisibility::Offline) {
                1
            } else {
                0
            },
            2000,
        )?;
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, false).await?;
        change(
            &mut tx,
            a,
            (s, id),
            v.revision,
            Some(v.visibility.as_str()),
            None,
            &reason,
        )
        .await?;
        let item = detail(&mut tx, s, id).await?;
        tx.commit().await?;
        Ok(item)
    }
    pub async fn management_delete(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> Result<()> {
        let mut tx = self.0.begin().await?;
        guard(&mut tx, a, s, false).await?;
        change(
            &mut tx,
            a,
            (s, id),
            revision,
            Some("deleted"),
            Some(false),
            "",
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
pub(super) async fn legacy_visibility(
    repo: &Repository,
    a: &Actor,
    s: Uuid,
    id: Uuid,
    v: Value,
) -> Result<Value> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Input {
        revision: i64,
        visibility: String,
        top_sort: Option<i32>,
        #[serde(default)]
        reason: String,
    }
    let input: Input =
        serde_json::from_value(v).map_err(|_| Error::Invalid("可见性参数无效".into()))?;
    require(
        matches!(
            input.visibility.as_str(),
            "visible" | "hidden" | "offline" | "deleted"
        ) && input.top_sort.is_none_or(|v| v >= 0),
        "可见状态或置顶参数无效",
    )?;
    let manager = a.can(s, "operator");
    let mut tx = repo.0.begin().await?;
    if manager {
        guard(&mut tx, a, s, false).await?;
    } else {
        crate::miniapp_users::south::adapter::postgres::guard_write(&mut tx, a, s).await?;
        if input.visibility != "deleted" || input.top_sort != Some(0) {
            return Err(Error::Forbidden);
        }
        let own: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM posts WHERE school_id=$1 AND id=$2 AND author_id=$3)",
        )
        .bind(s)
        .bind(id)
        .bind(a.id)
        .fetch_one(&mut *tx)
        .await?;
        if !own {
            return Err(Error::Forbidden);
        }
    }
    let reason = text(
        &input.reason,
        if input.visibility == "offline" { 1 } else { 0 },
        2000,
    )?;
    let item = change(
        &mut tx,
        a,
        (s, id),
        input.revision,
        Some(&input.visibility),
        input.top_sort.map(|v| v > 0),
        &reason,
    )
    .await?;
    tx.commit().await?;
    Ok(item)
}
