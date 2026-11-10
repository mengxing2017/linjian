use crate::business::{Error, Result, require};
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn parse(name: &str, bytes: &[u8]) -> Result<(Vec<Value>, Vec<Value>)> {
    require(
        name.to_lowercase().ends_with(".csv") || name.to_lowercase().ends_with(".xlsx"),
        "仅支持 CSV 和 XLSX",
    )?;
    let (table, lines) = crate::community::south::adapter::post_import_file::read_table(
        name,
        bytes,
        2 * 1024 * 1024,
        5000,
        3,
    )?;
    require(
        table.len() > 1 && table.len() <= 5001,
        "需包含表头和 1 至 5000 条记录",
    )?;
    let headers: Vec<_> = table[0].iter().map(|s| s.trim()).collect();
    let unique: HashSet<_> = headers.iter().copied().collect();
    require(
        unique.len() == headers.len()
            && unique.contains("屏蔽词")
            && headers
                .iter()
                .all(|h| ["屏蔽词", "备注", "状态"].contains(h)),
        "表头仅支持屏蔽词、备注、状态，且屏蔽词必填",
    )?;
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for (i, row) in table.iter().enumerate().skip(1) {
        let line = lines.get(i).copied().unwrap_or(i + 1);
        let cell = |key: &str| {
            headers
                .iter()
                .position(|h| *h == key)
                .and_then(|n| row.get(n))
                .map(|s| s.trim())
                .unwrap_or("")
        };
        let word = cell("屏蔽词");
        let remark = cell("备注");
        let error = if word.is_empty() || word.chars().count() > 100 {
            Some("屏蔽词须为 1 至 100 个字符")
        } else if remark.chars().count() > 255 {
            Some("备注不能超过 255 个字符")
        } else if [word, remark].iter().any(|s| {
            s.chars()
                .next()
                .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@') || c.is_control())
        }) {
            Some("内容不能以公式字符开头")
        } else if !matches!(cell("状态"), "" | "启用" | "停用") {
            Some("状态须为启用或停用")
        } else if !seen.insert(word.to_lowercase()) {
            Some("文件内屏蔽词重复")
        } else {
            None
        };
        if let Some(message) = error {
            errors.push(json!({"row":line,"message":message}));
        } else {
            rows.push(
                json!({"row":line,"word":word,"remark":remark,"active":cell("状态")!="停用"}),
            );
        }
    }
    if rows.is_empty() && errors.is_empty() {
        return Err(Error::Invalid("文件没有词条".into()));
    }
    Ok((rows, errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_rows_and_duplicate_words() {
        let (rows, errors) = parse(
            "words.csv",
            "屏蔽词,备注,状态\n诈骗,说明,启用\n诈骗,,停用\n其他,,未知\n".as_bytes(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0]["row"], 3);
        assert_eq!(errors[1]["row"], 4);
    }
}
