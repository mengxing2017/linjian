use super::{
    Repository,
    post_management::{guard, locked},
};
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
        let mut tx = self.0.begin().await?;
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
        let current=sqlx::query("SELECT id,revision,sort_order FROM posts WHERE school_id=$1 AND visibility<>'deleted' AND is_top=$2 ORDER BY sort_order DESC,created_at DESC,id LIMIT $3 OFFSET $4 FOR UPDATE")
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
        for (i, id) in v.ordered_ids.iter().enumerate() {
            let old = current
                .iter()
                .find(|r| r.get::<Uuid, _>("id") == *id)
                .ok_or(Error::Conflict)?;
            let weight: i64 = current[i].get("sort_order");
            if old.get::<i64, _>("sort_order") == weight {
                continue;
            }
            locked(&mut tx, s, *id, old.get("revision")).await?;
            sqlx::query("UPDATE posts SET sort_order=$3,revision=revision+1,updated_at=now() WHERE school_id=$1 AND id=$2").bind(s).bind(id).bind(weight).execute(&mut *tx).await?;
        }
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
