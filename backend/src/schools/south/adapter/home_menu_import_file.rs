use crate::business::{Error, Result, require, text};
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn parse(name: &str, bytes: &[u8]) -> Result<(Vec<Value>, Vec<Value>)> {
    let (table, lines) = crate::community::south::adapter::post_import_file::read_table(
        name,
        bytes,
        5 * 1024 * 1024,
        500,
        5,
    )?;
    require(
        (2..=501).contains(&table.len()),
        "需包含表头和 1 至 500 条标签",
    )?;
    let headers: Vec<_> = table[0].iter().map(|v| v.trim()).collect();
    let allowed = ["菜单/标签名称", "菜单状态", "是否启用", "排序", "来源类型"];
    let unique: HashSet<_> = headers.iter().copied().collect();
    require(
        unique.len() == headers.len()
            && unique.contains("菜单/标签名称")
            && headers.iter().all(|h| allowed.contains(h)),
        "表头无效或重复；导出文件不能直接导入",
    )?;
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut names = HashSet::new();
    for (i, row) in table.iter().enumerate().skip(1) {
        let number = lines.get(i).copied().unwrap_or(i + 1);
        let cell = |key: &str| {
            headers
                .iter()
                .position(|h| *h == key)
                .and_then(|n| row.get(n))
                .map(|v| v.trim())
                .unwrap_or("")
        };
        let parse_row = || -> Result<Value> {
            let name = text(cell("菜单/标签名称"), 1, 50)?;
            require(
                !name
                    .chars()
                    .next()
                    .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@') || c.is_control()),
                "名称不能以公式字符开头",
            )?;
            let menu = match cell("菜单状态") {
                "" | "显示" | "shown" => "shown",
                "显示并作为热门标签" | "hot" => "hot",
                "隐藏" | "隐藏/禁用" | "hidden" => "hidden",
                _ => return Err(Error::Invalid("菜单状态无效".into())),
            };
            let enabled = match cell("是否启用") {
                "" | "启用" | "enabled" => "enabled",
                "待审核" | "pending" => "pending",
                "停用" | "disabled" => "disabled",
                _ => return Err(Error::Invalid("启用状态无效".into())),
            };
            require(
                matches!(cell("来源类型"), "" | "后台配置" | "admin"),
                "来源类型仅允许后台配置",
            )?;
            let sort = if cell("排序").is_empty() {
                0
            } else {
                cell("排序")
                    .parse::<i64>()
                    .map_err(|_| Error::Invalid("排序须为整数".into()))?
            };
            require(
                (-1_000_000_000..=1_000_000_000).contains(&sort),
                "排序超出范围",
            )?;
            Ok(
                json!({"row":number,"name":name,"menu_status":menu,"enable_status":enabled,"sort_order":sort}),
            )
        };
        match parse_row() {Ok(v)=>{if !names.insert(v["name"].as_str().unwrap().to_lowercase()) {errors.push(json!({"row":number,"message":"文件内名称重复"}));} else {rows.push(v)}}, Err(e)=>errors.push(json!({"row":number,"message":match e { Error::Invalid(s)=>s,_=>"内容无效".into() }}))}
    }
    Ok((rows, errors))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checks_rows_and_source() {
        let (rows,errors)=parse("a.csv", "菜单/标签名称,菜单状态,是否启用,排序,来源类型\n校园,hot,启用,2,后台配置\n校园,显示,启用,1,后台配置\n其他,显示,启用,0,用户创建\n".as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(errors.len(), 2);
    }
}
