use super::*;
use std::io::Write;
fn xlsx(sheet: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Posts" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet),
    ] {
        zip.start_file(name, zip::write::FileOptions::<()>::default())
            .unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn record(out: &mut Vec<u8>, code: u16, data: &[u8]) {
    out.extend(code.to_le_bytes());
    out.extend((data.len() as u16).to_le_bytes());
    out.extend(data);
}
fn xls(formula: bool) -> Vec<u8> {
    let mut globals = Vec::new();
    record(&mut globals, 0x0809, &[0, 6, 5, 0]);
    // BOF (8) + BoundSheet (18) + EOF (4): worksheet begins at byte 30.
    let mut bound = 30u32.to_le_bytes().to_vec();
    bound.extend([0, 0, 6, 0]);
    bound.extend(b"Posts1");
    record(&mut globals, 0x0085, &bound);
    record(&mut globals, 0x000a, &[]);
    assert_eq!(globals.len(), 30);
    record(&mut globals, 0x0809, &[0, 6, 0x10, 0]);
    for (row, col, text) in [
        (0u16, 0u16, "标题"),
        (0, 1, "正文"),
        (1, 0, "导入标题"),
        (1, 1, "导入正文"),
    ] {
        let mut cell = Vec::new();
        cell.extend(row.to_le_bytes());
        cell.extend(col.to_le_bytes());
        cell.extend(0u16.to_le_bytes());
        cell.extend((text.encode_utf16().count() as u16).to_le_bytes());
        cell.push(1);
        for c in text.encode_utf16() {
            cell.extend(c.to_le_bytes());
        }
        record(&mut globals, 0x0204, &cell);
    }
    if formula {
        record(&mut globals, 0x0006, &[1, 0, 1, 0, 0, 0]);
    }
    record(&mut globals, 0x000a, &[]);
    let mut cfb = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    cfb.create_stream("/Workbook")
        .unwrap()
        .write_all(&globals)
        .unwrap();
    cfb.into_inner().into_inner()
}
#[test]
fn imports_xlsx_and_rejects_formula_or_unbounded_dimensions() {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:B2"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>标题</t></is></c><c r="B1" t="inlineStr"><is><t>正文</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>导入标题</t></is></c><c r="B2" t="inlineStr"><is><t>导入正文</t></is></c></row></sheetData></worksheet>"#;
    let (rows, errors) = parse("posts.xlsx", &xlsx(sheet), Uuid::nil()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(rows[0].post.title, "导入标题");
    assert!(
        parse(
            "posts.xlsx",
            &xlsx(&sheet.replace("A1:B2", "A1:XFD1048576")),
            Uuid::nil()
        )
        .is_err()
    );
    assert!(
        parse(
            "posts.xlsx",
            &xlsx(&sheet.replace(
                "<is><t>导入正文</t></is>",
                "<f>HYPERLINK(\"http://example.invalid\")</f><v>1</v>"
            )),
            Uuid::nil()
        )
        .is_err()
    );
}
#[test]
fn imports_biff8_xls_and_rejects_formula() {
    let (rows, errors) = parse("posts.xls", &xls(false), Uuid::nil()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(rows[0].post.content, "导入正文");
    assert!(parse("posts.xls", &xls(true), Uuid::nil()).is_err());
}
#[test]
fn csv_bom_quotes_multiline_limits_and_school_media() {
    let (rows,errors)=parse("posts.csv","\u{feff}标题,正文,标签名称,图集\n标题,\"第一行\n第二行\",\"[\"\"校园\"\",\"\"生活\"\"]\",[]\n".as_bytes(),Uuid::nil()).unwrap();
    assert!(errors.is_empty());
    assert_eq!(rows[0].post.tags, vec!["校园", "生活"]);
    assert!(rows[0].post.content.contains('\n'));
    assert!(parse("posts.csv", &vec![b'a'; MAX_BYTES + 1], Uuid::nil()).is_err());
    assert!(
        parse(
            "posts.csv",
            format!("标题,正文\n{}", "标题,正文\n".repeat(201)).as_bytes(),
            Uuid::nil()
        )
        .is_err()
    );
    let (_, errors) = parse(
        "posts.csv",
        "标题,正文,图集\n标题,正文,\"[\"\"https://example.invalid/image.png\"\"]\"\n".as_bytes(),
        Uuid::nil(),
    )
    .unwrap();
    assert_eq!(errors.len(), 1);
}
