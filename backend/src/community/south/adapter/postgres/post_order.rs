use super::{Repository, post_management::guard};
use crate::{
    business::{Actor, Error, Page, Result},
    community::domain::post_management::ReorderPostsInput,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
impl Repository {
    pub async fn management_reorder(
        &self,
        a: &Actor,
        s: Uuid,
        v: ReorderPostsInput,
    ) -> Result<i64> {
        v.validate()?;
        let mut tx = crate::media::south::adapter::lifecycle::begin(&self.0).await?;
        crate::schools::south::adapter::postgres::permissions::authorize(
            &mut tx,
            a,
            s,
            "content.post.pin",
        )
        .await?;
        guard(&mut tx, a, s, false).await?;
        sqlx::query("INSERT INTO post_order_versions(school_id) VALUES($1) ON CONFLICT DO NOTHING")
            .bind(s)
            .execute(&mut *tx)
            .await?;
        let version: i64 = sqlx::query_scalar(
            "SELECT revision FROM post_order_versions WHERE school_id=$1 FOR UPDATE",
        )
        .bind(s)
        .fetch_one(&mut *tx)
        .await?;
        if version != v.order_revision {
            return Err(Error::Conflict);
        }
        let (limit, offset) = Page {
            page: Some(v.page),
            page_size: Some(v.page_size),
            ..Default::default()
        }
        .bounds()?;
        let current=sqlx::query("SELECT id,revision,sort_order FROM posts WHERE school_id=$1 AND visibility<>'deleted' AND is_top=$2 ORDER BY sort_order DESC,display_published_at DESC,id LIMIT $3 OFFSET $4 FOR UPDATE")
  .bind(s).bind(v.is_top).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
        if current.len() != v.original.len()
            || current.iter().zip(&v.original).any(|(r, o)| {
                r.get::<Uuid, _>("id") != o.id || r.get::<i64, _>("revision") != o.revision
            })
        {
            return Err(Error::Conflict);
        }
        let previous: Vec<Uuid> = current.iter().map(|r| r.get("id")).collect();
        if previous == v.ordered_ids {
            tx.commit().await?;
            return Ok(version);
        }
        sqlx::query("WITH ranked AS (SELECT id,row_number() OVER(ORDER BY sort_order DESC,display_published_at DESC,id) AS position,count(*) OVER() AS total FROM posts WHERE school_id=$1 AND visibility<>'deleted' AND is_top=$2), desired AS (SELECT id,CASE WHEN id=ANY($3::uuid[]) THEN total-$4-array_position($3::uuid[],id)+1 ELSE total-position+1 END AS weight FROM ranked) UPDATE posts p SET sort_order=d.weight,revision=p.revision+1,updated_at=now() FROM desired d WHERE p.id=d.id AND p.sort_order IS DISTINCT FROM d.weight")
            .bind(s).bind(v.is_top).bind(&v.ordered_ids).bind(offset).execute(&mut *tx).await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM post_order_versions WHERE school_id=$1")
                .bind(s)
                .fetch_one(&mut *tx)
                .await?;
        crate::audit::record(&mut tx,a,s,"post.reorder",s,json!({"previous_order_revision":version,"order_revision":revision,"previous_ids":previous,"ordered_ids":v.ordered_ids})).await?;
        tx.commit().await?;
        Ok(revision)
    }
}
