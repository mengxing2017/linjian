use super::Repository;
use crate::{
    business::{Actor, Error, Page, Result, require, text},
    community::domain::review::{PostReview, digest},
};
use serde_json::Value;
use uuid::Uuid;

pub(super) const AUTHORIZATION: &str = "SELECT EXISTS(SELECT 1 FROM accounts a WHERE a.id=$1 AND a.active AND (a.platform_admin OR EXISTS(SELECT 1 FROM school_memberships m WHERE m.account_id=a.id AND m.school_id=$2 AND m.role IN('operator','admin'))))";

impl Repository {
    pub async fn reviews(
        &self,
        a: &Actor,
        school: Uuid,
        page: Page,
        id: Option<Uuid>,
    ) -> Result<Vec<PostReview>> {
        if !a.browser {
            return Err(crate::business::Error::Forbidden);
        }
        a.authorize(school, "operator")?;
        let allowed: bool = sqlx::query_scalar(AUTHORIZATION)
            .bind(a.id)
            .bind(school)
            .fetch_one(&self.0)
            .await?;
        if !allowed {
            return Err(Error::Forbidden);
        }
        crate::schools::south::adapter::postgres::Repository(self.0.clone())
            .require_active(school)
            .await?;
        let (limit, offset) = page.fetch_bounds()?;
        require(
            page.status
                .as_deref()
                .is_none_or(|s| matches!(s, "pending" | "approved" | "rejected" | "")),
            "审核状态无效",
        )?;
        let search = page.search.unwrap_or_default();
        text(&search, 0, 200)?;
        let rows: Vec<Value> = sqlx::query_scalar("SELECT jsonb_build_object('id',p.id,'school_id',p.school_id,'school_name',s.name,'author_id',p.author_id,'author_nickname',CASE WHEN u.account_id IS NOT NULL THEN coalesce(nullif(btrim(u.nickname),''),'未设置昵称') ELSE coalesce(nullif(btrim(a.display_name),''),'未设置昵称') END,'title',p.title,'content',p.content,'images',p.images,'tags',p.tags,'created_at',p.created_at,'audit_status',p.audit_status,'visibility',p.visibility,'top_sort',p.top_sort,'revision',p.revision,'content_revision',p.content_revision) FROM posts p JOIN schools s ON s.id=p.school_id LEFT JOIN accounts a ON a.id=p.author_id LEFT JOIN miniapp_users u ON u.account_id=p.author_id WHERE p.school_id=$1 AND s.active AND p.visibility<>'deleted' AND ($2::uuid IS NULL OR p.id=$2) AND ($3::text IS NULL OR $3='' OR p.audit_status=$3) AND (p.title ILIKE '%'||$4||'%' OR p.content ILIKE '%'||$4||'%') ORDER BY p.created_at DESC,p.id DESC LIMIT $5 OFFSET $6")
            .bind(school).bind(id).bind(page.status).bind(search).bind(limit).bind(offset).fetch_all(&self.0).await?;
        rows.into_iter()
            .map(|mut row| {
                row["review_digest"] = digest(&row)?.into();
                serde_json::from_value(row).map_err(|_| Error::Unavailable)
            })
            .collect()
    }
}
