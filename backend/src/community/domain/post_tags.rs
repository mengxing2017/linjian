use crate::business::{Result, require};
use std::collections::HashSet;

pub fn validate_selection(
    submitted: &[String],
    candidates: &[String],
    retained: &[String],
) -> Result<()> {
    let mut seen = HashSet::new();
    for tag in submitted {
        let name = tag.trim();
        require(seen.insert(name), "标签重复")?;
        require(
            candidates.iter().any(|candidate| candidate == name)
                || retained.iter().any(|previous| previous == name),
            "只能选择本校配置的标签，请重新选择",
        )?;
    }
    require(
        candidates
            .iter()
            .all(|candidate| seen.contains(candidate.as_str())),
        "候选标签与提交名称不一致",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn accepts_empty_and_complete_candidate_selection() {
        assert!(validate_selection(&[], &[], &[]).is_ok());
        assert!(validate_selection(&tags(&[" 校园 "]), &tags(&["校园"]), &[]).is_ok());
    }

    #[test]
    fn rejects_custom_names_even_when_mixed_with_valid_candidates() {
        assert!(validate_selection(&tags(&["自定义"]), &[], &[]).is_err());
        assert!(validate_selection(&tags(&["校园", "自定义"]), &tags(&["校园"]), &[]).is_err());
    }

    #[test]
    fn retains_only_names_from_the_locked_original_post() {
        assert!(
            validate_selection(
                &tags(&["校园", "旧标签"]),
                &tags(&["校园"]),
                &tags(&["旧标签"])
            )
            .is_ok()
        );
        assert!(validate_selection(&[], &[], &tags(&["旧标签"])).is_ok());
        assert!(validate_selection(&tags(&["其他帖子"]), &[], &tags(&["旧标签"])).is_err());
    }

    #[test]
    fn rejects_duplicates_and_unrepresented_candidates() {
        assert!(validate_selection(&tags(&["校园", " 校园 "]), &tags(&["校园"]), &[]).is_err());
        assert!(validate_selection(&[], &tags(&["校园"]), &[]).is_err());
    }
}
