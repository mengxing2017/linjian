use super::Service;
use crate::{
    business::{Actor, Result},
    community::{
        domain::hot_posts::{HotPostPolicy, HotPostsQuery, HotPostsResult},
        south::port::Store,
    },
};
use uuid::Uuid;

impl<R: Store> Service<R> {
    pub async fn hot_posts(
        &self,
        actor: &Actor,
        school: Uuid,
        query: HotPostsQuery,
    ) -> Result<HotPostsResult> {
        actor.authorize(school, "member")?;
        query.validate()?;
        self.0
            .hot_posts(school, query, HotPostPolicy::default())
            .await
    }
}
