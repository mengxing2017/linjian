//! File parsing is bounded before spreadsheet libraries allocate rectangular ranges.
use crate::{
    business::{Error, Result, require},
    community::domain::{PostInput, post_import::*},
};
use calamine::{Data, Reader};
use std::{
    collections::HashSet,
    io::{Cursor, Read},
};
use uuid::Uuid;
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
const MAX_EXPANDED: u64 = 24 * 1024 * 1024;
fn coordinate(value: &[u8]) -> Result<()> {
    let mut col = 0usize;
    let mut row = 0usize;
    for &b in value {
        if b.is_ascii_alphabetic() {
            col = col
                .saturating_mul(26)
                .saturating_add((b.to_ascii_uppercase() - b'A' + 1) as usize);
        } else if b.is_ascii_digit() {
            row = row.saturating_mul(10).saturating_add((b - b'0') as usize);
        } else {
            return Err(invalid("工作表坐标无效"));
        }
    }
    require(
        (1..=4).contains(&col) && (1..=MAX_ROWS + 1).contains(&row),
        "工作表最多 201 行、4 列",
    )
}
fn preflight_xlsx(bytes: &[u8]) -> Result<()> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid("XLSX 文件无效"))?;
    require(zip.len() <= 128, "工作簿文件数量超限")?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|_| invalid("工作簿读取失败"))?;
        total = total.saturating_add(entry.size());
        require(total <= MAX_EXPANDED, "工作簿解压后超过限制")?;
        let name = entry
            .name()
            .map_err(|_| invalid("工作簿文件名无效"))?
            .to_ascii_lowercase();
        require(
            !name.contains("vbaproject")
                && !name.contains("externallink")
                && !name.contains("macrosheets"),
            "不支持宏或外部链接",
        )?;
        if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
            let mut xml = String::new();
            entry
                .take(MAX_EXPANDED + 1)
                .read_to_string(&mut xml)
                .map_err(|_| invalid("工作表编码错误"))?;
            let mut reader = quick_xml::Reader::from_str(&xml);
            let mut cells = 0;
            loop {
                match reader
                    .read_event()
                    .map_err(|_| invalid("工作表 XML 无效"))?
                {
                    quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e) => {
                        if e.local_name().as_ref() == "dimension" {
                            for attr in e.attributes() {
                                let attr = attr.map_err(|_| invalid("工作表范围无效"))?;
                                if attr.key.as_ref() == "ref" {
                                    for coordinate_value in attr.value.split(':') {
                                        coordinate(coordinate_value.as_bytes())?;
                                    }
                                }
                            }
                        }
                        if e.local_name().as_ref() == "row" {
                            for attr in e.attributes() {
                                let attr = attr.map_err(|_| invalid("行坐标无效"))?;
                                if attr.key.as_ref() == "r" {
                                    require(
                                        attr.value
                                            .parse::<usize>()
                                            .is_ok_and(|r| r > 0 && r <= MAX_ROWS + 1),
                                        "工作表行数超限",
                                    )?;
                                }
                            }
                        }
                        if e.local_name().as_ref() == "c" {
                            cells += 1;
                            require(cells <= (MAX_ROWS + 1) * 4, "工作表单元格数量超限")?;
                            let mut has_coordinate = false;
                            for attr in e.attributes() {
                                let attr = attr.map_err(|_| invalid("工作表坐标无效"))?;
                                if attr.key.as_ref() == "r" {
                                    has_coordinate = true;
                                    coordinate(attr.value.as_bytes())?;
                                }
                            }
                            require(has_coordinate, "工作表单元格缺少坐标")?;
                        }
                        if e.local_name().as_ref() == "f" {
                            return Err(invalid("导入不接受公式，请先粘贴为纯文本"));
                        }
                    }
                    quick_xml::events::Event::Eof => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
fn preflight_xls(bytes: &[u8]) -> Result<()> {
    let mut cfb =
        cfb::CompoundFile::open(Cursor::new(bytes)).map_err(|_| invalid("XLS 文件无效"))?;
    require(
        !cfb.walk()
            .any(|e| e.name().to_ascii_lowercase().contains("vba")),
        "不支持宏",
    )?;
    let stream = if cfb.is_stream("/Workbook") {
        "/Workbook"
    } else {
        "/Book"
    };
    let mut workbook = Vec::new();
    cfb.open_stream(stream)
        .map_err(|_| invalid("XLS 工作簿缺失"))?
        .take(MAX_EXPANDED + 1)
        .read_to_end(&mut workbook)
        .map_err(|_| invalid("XLS 读取失败"))?;
    require(workbook.len() as u64 <= MAX_EXPANDED, "XLS 工作簿过大")?;
    let mut offset = 0;
    while offset + 4 <= workbook.len() {
        let code = u16::from_le_bytes([workbook[offset], workbook[offset + 1]]);
        let len = u16::from_le_bytes([workbook[offset + 2], workbook[offset + 3]]) as usize;
        offset += 4;
        let data = workbook
            .get(offset..offset + len)
            .ok_or_else(|| invalid("XLS 记录损坏"))?;
        offset += len;
        let u16at = |i| u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if code == 0x0085 && len >= 6 {
            require(data[5] == 0, "仅支持普通工作表，不支持宏或图表工作表")?;
        }
        if code == 0x0809 {
            require(
                len >= 4 && u16at(0) >= 0x0500,
                "仅支持 Excel 5 及以后的 XLS 文件",
            )?;
        }
        if code == 0x0200 && (10..14).contains(&len) {
            require(
                u16at(2) <= MAX_ROWS + 1 && u16at(6) <= 4,
                "工作表最多 201 行、4 列",
            )?;
        }
        if code == 0x0200 && len >= 14 {
            let rows = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
            require(
                rows <= MAX_ROWS + 1 && u16at(10) <= 4,
                "工作表最多 201 行、4 列",
            )?;
        }
        if matches!(
            code,
            0x0201 | 0x0203 | 0x0204 | 0x0205 | 0x027e | 0x00fd | 0x00bd | 0x00be | 0x0006
        ) && len >= 4
        {
            require(
                u16at(0) < MAX_ROWS + 1 && u16at(2) < 4,
                "工作表最多 201 行、4 列",
            )?;
            if matches!(code, 0x00bd | 0x00be) {
                require(len >= 6 && u16at(len - 2) < 4, "工作表列数超限")?;
            }
            require(code != 0x0006, "导入不接受公式，请先粘贴为纯文本")?;
        }
    }
    Ok(())
}
pub fn parse(
    name: &str,
    bytes: &[u8],
    school: Uuid,
) -> Result<(Vec<ImportRow>, Vec<ImportRowError>)> {
    require(
        !bytes.is_empty() && bytes.len() <= MAX_BYTES,
        "文件须为 5 MiB 以内的非空文件",
    )?;
    let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let mut line_numbers = Vec::new();
    let table: Vec<Vec<String>> = if extension == "csv" {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(bytes);
        let mut rows = Vec::new();
        for record in reader.records() {
            require(rows.len() <= MAX_ROWS, "最多导入 200 条帖子")?;
            let record = record.map_err(|_| invalid("CSV 必须使用 UTF-8 编码，且各行列数一致"))?;
            require(record.len() <= 4, "最多四列")?;
            line_numbers.push(
                record
                    .position()
                    .map(|p| p.line() as usize)
                    .unwrap_or(rows.len() + 1),
            );
            rows.push(record.iter().map(str::to_owned).collect());
        }
        rows
    } else {
        match extension.as_str() {
            "xlsx" => preflight_xlsx(bytes)?,
            "xls" => preflight_xls(bytes)?,
            _ => return Err(invalid("仅支持 CSV、XLS、XLSX")),
        }
        let mut workbook = calamine::open_workbook_auto_from_rs(Cursor::new(bytes))
            .map_err(|_| invalid("工作簿解析失败"))?;
        let names = workbook.sheet_names();
        require(names.len() == 1, "仅支持一个工作表")?;
        let range = workbook
            .worksheet_range(&names[0])
            .map_err(|_| invalid("工作表解析失败"))?;
        require(range.start() == Some((0, 0)), "表头须从 A1 单元格开始")?;
        require(
            range.height() <= MAX_ROWS + 1 && range.width() <= 4,
            "工作表最多 201 行、4 列",
        )?;
        range
            .rows()
            .map(|r| {
                r.iter()
                    .map(|c| match c {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        _ => c.to_string(),
                    })
                    .collect()
            })
            .collect()
    };
    require(
        table.len() > 1 && table.len() <= MAX_ROWS + 1,
        "需包含表头和 1 至 200 条帖子",
    )?;
    let headers = &table[0];
    let allowed = ["标题", "正文", "标签名称", "图集"];
    let mut seen = HashSet::new();
    for header in headers {
        require(
            allowed.contains(&header.trim()) && seen.insert(header.trim()),
            "表头只允许标题、正文、标签名称、图集，且不能重复",
        )?;
    }
    require(
        seen.contains("标题") && seen.contains("正文"),
        "缺少标题或正文表头",
    )?;
    let mut valid = Vec::new();
    let mut errors = Vec::new();
    for (index, row) in table.iter().enumerate().skip(1) {
        let parse_row = || -> Result<PostInput> {
            require(
                row.iter().all(|v| v.chars().count() <= 20000),
                "单元格文本超过限制",
            )?;
            let cell = |key: &str| {
                headers
                    .iter()
                    .position(|h| h.trim() == key)
                    .and_then(|i| row.get(i))
                    .map(|s| s.trim())
                    .unwrap_or("")
            };
            let array = |key: &str| -> Result<Vec<String>> {
                let v = cell(key);
                if v.is_empty() {
                    Ok(Vec::new())
                } else {
                    serde_json::from_str(v)
                        .map_err(|_| invalid("标签名称、图集须为 JSON 字符串数组"))
                }
            };
            let post = PostInput {
                title: cell("标题").into(),
                content: cell("正文").into(),
                tags: array("标签名称")?,
                images: array("图集")?,
                revision: None,
            };
            require(
                (1..=200).contains(&post.title.chars().count()),
                "标题须为 1 至 200 字",
            )?;
            require(
                (1..=20000).contains(&post.content.chars().count()),
                "正文须为 1 至 20000 字",
            )?;
            post.validate(school)?;
            Ok(post)
        };
        match parse_row() {
            Ok(post) => valid.push(ImportRow {
                row: line_numbers.get(index).copied().unwrap_or(index + 1),
                post,
            }),
            Err(e) => errors.push(ImportRowError {
                row: line_numbers.get(index).copied().unwrap_or(index + 1),
                message: match e {
                    Error::Invalid(message) => message,
                    _ => "文件内容校验失败".into(),
                },
            }),
        }
    }
    Ok((valid, errors))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_preview_reports_rows_and_rejects_extra_fields() {
        let (rows, errors) = parse(
            "posts.csv",
            "标题,正文,标签名称\n有效,正文,\n,缺少标题,\n".as_bytes(),
            Uuid::nil(),
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(errors[0].row, 3);
        assert!(parse("posts.csv", "标题,正文,学校\na,b,c".as_bytes(), Uuid::nil()).is_err());
    }
    #[test]
    fn spreadsheet_coordinates_are_bounded() {
        assert!(coordinate(b"D201").is_ok());
        assert!(coordinate(b"XFD1048576").is_err());
        assert!(parse("a.xls", b"broken", Uuid::nil()).is_err());
        assert!(parse("a.xlsm", b"broken", Uuid::nil()).is_err());
    }
}

#[cfg(test)]
mod spreadsheet_tests;
