use crate::business::{Result, require};
use serde_json::Value;

fn invisible(character: char) -> bool {
    matches!(character, '\u{200b}'..='\u{200d}' | '\u{2060}' | '\u{feff}' | '\u{fe00}'..='\u{fe0f}' | '\u{e0100}'..='\u{e01ef}')
}

pub fn has_meaningful_text(value: &str) -> bool {
    value.chars().any(|character| {
        !character.is_whitespace() && !character.is_control() && !invisible(character)
    })
}

pub fn normalize_title(value: &str) -> Result<String> {
    let value = value.trim();
    require(
        !value.chars().any(|character| {
            character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
        }),
        "标题必须为单行文本",
    )?;
    require(value.chars().count() <= 200, "标题最多 200 字")?;
    require(
        value.is_empty() || has_meaningful_text(value),
        "标题不能仅包含不可见字符",
    )?;
    Ok(value.into())
}

pub fn normalize_content(value: &str) -> Result<String> {
    let value = value.trim();
    require(has_meaningful_text(value), "请填写正文")?;
    require(value.chars().count() <= 20000, "正文最多 20000 字")?;
    Ok(value.into())
}

pub fn display_label(title: &str, content: &str, has_images: bool) -> String {
    let source = if has_meaningful_text(title) {
        title
    } else {
        content
    };
    let cleaned = source
        .chars()
        .filter(|character| {
            (!character.is_control() || character.is_whitespace())
                && !matches!(character, '\u{200b}' | '\u{2060}' | '\u{feff}')
        })
        .collect::<String>();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if !has_meaningful_text(&cleaned) {
        return if has_images {
            "图片动态"
        } else {
            "校园动态"
        }
        .into();
    }
    if cleaned.chars().count() > 80 {
        cleaned.chars().take(79).chain(['…']).collect()
    } else {
        cleaned
    }
}

pub fn project(mut value: Value) -> Value {
    value["display_label"] = display_label(
        value["title"].as_str().unwrap_or_default(),
        value["content"].as_str().unwrap_or_default(),
        value["images"]
            .as_array()
            .is_some_and(|images| !images.is_empty()),
    )
    .into();
    value
}

pub fn project_reference(mut value: Value) -> Value {
    let post = value
        .as_object_mut()
        .and_then(|fields| fields.remove("_post"));
    value["post_display_label"] = match post {
        Some(post) if post.is_object() => project(post)["display_label"].clone(),
        _ => Value::Null,
    };
    if value["kind"] == "report" {
        value["title"] = value["post_display_label"]
            .as_str()
            .unwrap_or("举报内容不可用")
            .into();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_title_and_required_body_use_scalar_limits() {
        assert_eq!(normalize_title(" \n ").unwrap(), "");
        assert!(normalize_title("多行\n标题").is_err());
        assert!(normalize_title(&"😀".repeat(200)).is_ok());
        assert!(normalize_title(&"😀".repeat(201)).is_err());
        assert!(normalize_content(&"字".repeat(20000)).is_ok());
        assert!(normalize_content(&"字".repeat(20001)).is_err());
        assert!(normalize_content(" \u{200b}\u{200d}\u{fe0f}").is_err());
        assert_eq!(normalize_content(" 👩‍💻\n正文 ").unwrap(), "👩‍💻\n正文");
    }

    #[test]
    fn labels_are_derived_without_mutating_content() {
        assert_eq!(display_label("标题", "正文", false), "标题");
        assert_eq!(display_label("", " 首行\n\t次行 ", false), "首行 次行");
        assert_eq!(display_label("", "\u{feff}正文\u{200b}", false), "正文");
        assert_eq!(display_label("", "", true), "图片动态");
        assert_eq!(display_label("", "", false), "校园动态");
        assert_eq!(
            display_label("", &"😀".repeat(80), false).chars().count(),
            80
        );
        assert_eq!(
            display_label("", &"😀".repeat(81), false),
            "😀".repeat(79) + "…"
        );
    }
}
