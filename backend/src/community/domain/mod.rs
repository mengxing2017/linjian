pub mod interaction;
pub mod notification_summary;
pub mod post_search;
pub mod post_seed;
pub mod post_text;
pub mod public_post;
use crate::business::{Result, require, text};
use serde::Deserialize;
use uuid::Uuid;
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostInput {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_tag_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<i64>,
}
impl PostInput {
    pub fn normalized(mut self) -> Result<Self> {
        self.title = post_text::normalize_title(&self.title)?;
        self.content = post_text::normalize_content(&self.content)?;
        Ok(self)
    }

    pub fn validate(&self, school: Uuid) -> Result<()> {
        post_text::normalize_title(&self.title)?;
        post_text::normalize_content(&self.content)?;
        require(
            self.images.len() <= 9 && self.tags.len() <= 10 && self.candidate_tag_ids.len() <= 10,
            "图片或标签过多",
        )?;
        for image in &self.images {
            require(
                image.starts_with(&format!("/api/v1/schools/{school}/media/")),
                "图片必须属于当前学校",
            )?;
        }
        for tag in &self.tags {
            text(tag, 1, 50)?;
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePostInput {
    #[serde(default)]
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub candidate_tag_ids: Vec<Uuid>,
}
impl From<CreatePostInput> for PostInput {
    fn from(input: CreatePostInput) -> Self {
        Self {
            title: input.title,
            content: input.content,
            images: input.images,
            tags: input.tags,
            candidate_tag_ids: input.candidate_tag_ids,
            revision: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePostInput {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub candidate_tag_ids: Vec<Uuid>,
    pub revision: i64,
}
impl TryFrom<UpdatePostInput> for PostInput {
    type Error = crate::business::Error;
    fn try_from(input: UpdatePostInput) -> Result<Self> {
        require(input.revision > 0, "版本号无效")?;
        Ok(Self {
            title: input.title,
            content: input.content,
            images: input.images,
            tags: input.tags,
            candidate_tag_ids: input.candidate_tag_ids,
            revision: Some(input.revision),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommentInput {
    pub content: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetCommentVisibility {
    pub revision: i64,
    pub visibility: CommentVisibility,
}
#[derive(Debug, Clone, Copy, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentVisibility {
    Visible,
    Hidden,
}
impl CommentVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
        }
    }
}

pub mod moderation;
pub mod review;

pub mod case;

pub mod post_import;
pub mod post_management;

pub mod hot_posts;
pub mod post_recommendation;

#[cfg(test)]
mod post_input_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_defaults_title_but_complete_updates_require_title_and_revision() {
        let create: CreatePostInput = serde_json::from_value(json!({"content":"正文"})).unwrap();
        let input: PostInput = create.into();
        assert_eq!(input.normalized().unwrap().title, "");
        for value in [
            json!({"content":"正文","revision":1}),
            json!({"title":"","content":"正文"}),
            json!({"title":null,"content":"正文","revision":1}),
        ] {
            assert!(serde_json::from_value::<UpdatePostInput>(value).is_err());
        }
        assert!(
            serde_json::from_value::<CreatePostInput>(json!({"title":null,"content":"正文"}))
                .is_err()
        );
        assert!(
            serde_json::from_value::<CreatePostInput>(json!({"content":"正文","revision":1}))
                .is_err()
        );
        let update: UpdatePostInput =
            serde_json::from_value(json!({"title":"","content":"正文","revision":1})).unwrap();
        assert!(PostInput::try_from(update).is_ok());
        let update: UpdatePostInput =
            serde_json::from_value(json!({"title":"","content":"正文","revision":0})).unwrap();
        assert!(PostInput::try_from(update).is_err());
    }
}
