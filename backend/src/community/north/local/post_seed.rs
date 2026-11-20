use crate::{
    business::{Actor, Result, require, text},
    community::{domain::post_seed::*, south::port::post_seed::Store},
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct Service<R: Store = crate::community::south::adapter::postgres::post_seed::Repository>(
    pub R,
);
impl<R: Store> Service<R> {
    pub async fn preview(&self, actor: &Actor, school: Uuid, input: PreviewInput) -> Result<Value> {
        input.validate()?;
        self.0.preview(actor, school, input).await
    }
    pub async fn commit(
        &self,
        actor: &Actor,
        school: Uuid,
        batch: Uuid,
        input: CommitInput,
    ) -> Result<Value> {
        require(
            input.revision > 0 && input.digest.len() == 64,
            "预览版本或摘要无效",
        )?;
        self.0.commit(actor, school, batch, input).await
    }
    pub async fn revoke(
        &self,
        actor: &Actor,
        school: Uuid,
        batch: Uuid,
        input: RevokeInput,
    ) -> Result<Value> {
        require(input.revision > 0, "版本号无效")?;
        text(&input.reason, 1, 500)?;
        self.0.revoke(actor, school, batch, input).await
    }
    pub async fn read(
        &self,
        actor: &Actor,
        school: Uuid,
        kind: &str,
        batch: Option<Uuid>,
        query: SeedQuery,
    ) -> Result<Value> {
        query.page().fetch_bounds()?;
        self.0.read(actor, school, kind, batch, query).await
    }
}
