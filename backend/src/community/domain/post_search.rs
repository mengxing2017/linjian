use crate::business::{Result, require};

pub fn normalize(input: Option<&str>) -> Result<Option<String>> {
    let keyword = input.unwrap_or_default().trim_matches(|character| {
        matches!(character, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
    });
    require(keyword.chars().count() <= 100, "搜索关键词最多 100 字")?;
    Ok((!keyword.is_empty()).then(|| keyword.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_search_normalizes_the_ecmascript_boundary_whitespace() {
        let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/post-search-normalization.json"
        ))
        .unwrap();
        for fixture in fixtures {
            assert_eq!(
                normalize(fixture["input"].as_str())
                    .unwrap()
                    .unwrap_or_default(),
                fixture["output"].as_str().unwrap()
            );
        }
        assert_eq!(normalize(None).unwrap(), None);
        assert_eq!(normalize(Some(" \t\r\n　\u{feff}")).unwrap(), None);
        assert_eq!(
            normalize(Some("\u{feff} 课 本　")).unwrap().as_deref(),
            Some("课 本")
        );
        assert_eq!(
            normalize(Some("\u{0085}词\u{0085}")).unwrap().as_deref(),
            Some("\u{0085}词\u{0085}")
        );
        assert!(normalize(Some(&"😀".repeat(100))).is_ok());
        assert!(normalize(Some(&"😀".repeat(101))).is_err());
    }
}
