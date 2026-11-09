use super::*;
use crate::community::domain::post_management::*;

impl<R: Store> Service<R> {
    pub async fn management_list(
        &self,
        a: &Actor,
        s: Uuid,
        f: PostManagementFilters,
    ) -> Result<PostManagementPage> {
        a.authorize(s, "operator")?;
        f.validate()?;
        self.0.management_list(a, s, f).await
    }
    pub async fn management_detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> Result<PostManagementView> {
        a.authorize(s, "operator")?;
        self.0.management_detail(a, s, id).await
    }
    pub async fn management_write(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        v: PostInput,
    ) -> Result<PostManagementView> {
        a.authorize(s, "operator")?;
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
        a.authorize(s, "operator")?;
        self.0.management_pin(a, s, id, v).await
    }
    pub async fn management_status(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: SetPostStatusInput,
    ) -> Result<PostManagementView> {
        a.authorize(s, "operator")?;
        self.0.management_status(a, s, id, v).await
    }
    pub async fn management_delete(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> Result<()> {
        a.authorize(s, "operator")?;
        self.0.management_delete(a, s, id, revision).await
    }
    pub async fn management_reorder(
        &self,
        a: &Actor,
        s: Uuid,
        v: ReorderPostsInput,
    ) -> Result<i64> {
        a.authorize(s, "operator")?;
        v.validate()?;
        self.0.management_reorder(a, s, v).await
    }
}
