use super::*;
use crate::community::domain::post_management::*;

impl<R: Store> Service<R> {
    pub async fn management_list(
        &self,
        a: &Actor,
        s: Uuid,
        f: PostManagementFilters,
    ) -> Result<PostManagementPage> {
        a.permission(s, "content.post.read")?;
        f.validate()?;
        self.0.management_list(a, s, f).await
    }
    pub async fn management_detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> Result<PostManagementView> {
        a.permission(s, "content.post.read")?;
        self.0.management_detail(a, s, id).await
    }
    pub async fn management_write(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        v: PostInput,
    ) -> Result<PostManagementView> {
        a.permission(s, "content.post.edit")?;
        if id.is_none() {
            a.platform()?;
        }
        v.validate(s)?;
        self.0.management_write(a, s, id, v).await
    }
    pub async fn management_pin(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: PinPostInput,
    ) -> Result<PostManagementView> {
        a.permission(s, "content.post.pin")?;
        self.0.management_pin(a, s, id, v).await
    }
    pub async fn management_status(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: SetPostStatusInput,
    ) -> Result<PostManagementView> {
        a.permission(s, "content.post.visibility")?;
        self.0.management_status(a, s, id, v).await
    }
    pub async fn management_delete(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> Result<()> {
        a.permission(s, "content.post.delete")?;
        self.0.management_delete(a, s, id, revision).await
    }
    pub async fn management_reorder(
        &self,
        a: &Actor,
        s: Uuid,
        v: ReorderPostsInput,
    ) -> Result<i64> {
        a.permission(s, "content.post.pin")?;
        v.validate()?;
        self.0.management_reorder(a, s, v).await
    }
}

impl<R: Store> Service<R> {
    pub async fn recommendation_preview(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> Result<serde_json::Value> {
        a.permission(s, "content.post.read")?;
        self.0.recommendation_preview(a, s, id).await
    }
    pub async fn recommend_post(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        mut input: crate::community::domain::post_recommendation::RecommendPost,
    ) -> Result<(serde_json::Value, bool)> {
        input.validate()?;
        self.0.recommend_post(a, s, id, input).await
    }
    pub async fn recommendations(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        record: Option<Uuid>,
        page: Page,
    ) -> Result<Vec<serde_json::Value>> {
        a.permission(s, "content.post.read")?;
        page.bounds()?;
        self.0.recommendations(a, s, id, record, page).await
    }
}
