use crate::business::{Actor, Decision, Page, Result};
use crate::community::domain::case::{CaseManagementView, ResolveCaseInput};
use crate::community::domain::{CommentInput, PostInput};
use crate::community::domain::{post_import::*, post_management::*};
use serde_json::Value;
use uuid::Uuid;
pub trait Store: Clone + Send + Sync + 'static {
    fn management_list(
        &self,
        a: &Actor,
        s: Uuid,
        f: PostManagementFilters,
    ) -> impl std::future::Future<Output = Result<PostManagementPage>> + Send;

    fn management_detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<PostManagementView>> + Send;

    fn management_write(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        v: PostInput,
    ) -> impl std::future::Future<Output = Result<PostManagementView>> + Send;

    fn management_pin(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: PinPostInput,
    ) -> impl std::future::Future<Output = Result<PostManagementView>> + Send;

    fn management_status(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: SetPostStatusInput,
    ) -> impl std::future::Future<Output = Result<PostManagementView>> + Send;

    fn management_delete(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn management_reorder(
        &self,
        a: &Actor,
        s: Uuid,
        v: ReorderPostsInput,
    ) -> impl std::future::Future<Output = Result<i64>> + Send;

    fn check_post_import_access(
        &self,
        a: &Actor,
        s: Uuid,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn import_preview(
        &self,
        a: &Actor,
        s: Uuid,
        file_digest: String,
        rows: Vec<ImportRow>,
        errors: Vec<ImportRowError>,
    ) -> impl std::future::Future<Output = Result<PostImportPreview>> + Send;

    fn import_detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<PostImportPreview>> + Send;

    fn import_commit(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: CommitPostImport,
    ) -> impl std::future::Future<Output = Result<PostImportPreview>> + Send;

    fn reviews(
        &self,
        a: &Actor,
        s: Uuid,
        page: Page,
        id: Option<Uuid>,
    ) -> impl std::future::Future<Output = Result<Vec<crate::community::domain::review::PostReview>>>
    + Send;
    fn validate_media(
        &self,
        s: Uuid,
        images: &[String],
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn check_words(
        &self,
        s: Uuid,
        content: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn posts(
        &self,
        a: &Actor,
        s: Uuid,
        p: Page,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn detail(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn create(
        &self,
        a: &Actor,
        s: Uuid,
        v: PostInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn update(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: PostInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn visibility(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: Value,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn moderate(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: Decision,
        comment: bool,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn react(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        kind: &str,
        add: bool,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn reactions(
        &self,
        a: &Actor,
        s: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn comments(
        &self,
        a: &Actor,
        s: Uuid,
        id: Option<Uuid>,
        p: Page,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn comment(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: CommentInput,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn comment_visibility(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: crate::community::domain::SetCommentVisibility,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn delete_comment(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        revision: i64,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn follow(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        add: bool,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn follows(
        &self,
        a: &Actor,
        s: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn notifications(
        &self,
        a: &Actor,
        s: Uuid,
        page: Page,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn read(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn cases(
        &self,
        a: &Actor,
        s: Uuid,
        page: Page,
    ) -> impl std::future::Future<Output = Result<Vec<Value>>> + Send;
    fn case(
        &self,
        a: &Actor,
        s: Uuid,
        v: Value,
    ) -> impl std::future::Future<Output = Result<Value>> + Send;
    fn resolve(
        &self,
        a: &Actor,
        s: Uuid,
        id: Uuid,
        v: ResolveCaseInput,
    ) -> impl std::future::Future<Output = Result<CaseManagementView>> + Send;
}
