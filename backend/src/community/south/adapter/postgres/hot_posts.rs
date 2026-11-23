use super::Repository;
use crate::{
    business::{Error, Result},
    community::domain::hot_posts::{
        HotPostPolicy, HotPostSummary, HotPostsMeta, HotPostsQuery, HotPostsResult,
    },
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

const RANKING_SQL: &str = include_str!("hot_posts.sql");

impl Repository {
    pub async fn hot_posts(
        &self,
        school: Uuid,
        query: HotPostsQuery,
        policy: HotPostPolicy,
    ) -> Result<HotPostsResult> {
        query.validate()?;
        let mut transaction = self.0.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("SET LOCAL statement_timeout = '1500ms'")
            .execute(&mut *transaction)
            .await?;
        let active: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM schools WHERE id=$1 AND active)")
                .bind(school)
                .fetch_one(&mut *transaction)
                .await?;
        if !active {
            return Err(Error::SchoolUnavailable);
        }
        let time: DateTime<Utc> = sqlx::query_scalar("SELECT transaction_timestamp()")
            .fetch_one(&mut *transaction)
            .await?;
        let rows =
            sqlx::query_as::<_, (Uuid, Uuid, i64, String, String, bool, i64, i64)>(RANKING_SQL)
                .bind(school)
                .bind(time - policy.window)
                .bind(time)
                .bind(policy.comment_weight)
                .bind(policy.offset_hours)
                .bind(policy.exponent)
                .bind(policy.minimum_engagement)
                .bind(i64::from(query.limit))
                .fetch_all(&mut *transaction)
                .await?;
        transaction.commit().await?;
        let items = rows
            .into_iter()
            .map(
                |(id, school_id, rank, title, content, has_images, like_count, comment_count)| {
                    HotPostSummary {
                        id,
                        school_id,
                        rank,
                        display_label: crate::community::domain::post_text::display_label(
                            &title, &content, has_images,
                        ),
                        like_count,
                        comment_count,
                    }
                },
            )
            .collect();
        Ok(HotPostsResult {
            items,
            meta: HotPostsMeta {
                window_start: time - policy.window,
                window_end: time,
                ranked_at: time,
                validated_at: time,
                limit: query.limit,
            },
        })
    }
}
