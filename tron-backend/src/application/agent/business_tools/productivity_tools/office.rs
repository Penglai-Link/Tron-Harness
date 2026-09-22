use super::*;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn package(path: &Path) -> Result<BTreeMap<String, Vec<u8>>, ToolError> {
    let bytes = read_file(path)?;
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(error)?;
    if zip.len() > 4096 {
        return Err(error("Office package contains too many entries"));
    }
    let mut total = 0u64;
    let mut entries = BTreeMap::new();
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(error)?;
        total = total
            .checked_add(file.size())
            .ok_or_else(|| error("Office package too large"))?;
        if total > 64 * 1024 * 1024 {
            return Err(error("expanded Office package exceeds 64 MiB"));
        }
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        let mut data = Vec::new();
        file.take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(error)?;
        if data.len() > 64 * 1024 * 1024 {
            return Err(error("Office entry too large"));
        }
        if entries.insert(name, data).is_some() {
            return Err(error("duplicate Office package entry"));
        }
    }
    Ok(entries)
}
fn pack(entries: BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, ToolError> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(
            name,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(error)?;
        zip.write_all(&data).map_err(error)?;
    }
    Ok(zip.finish().map_err(error)?.into_inner())
}
fn xml_document(entries: &BTreeMap<String, Vec<u8>>) -> Result<&str, ToolError> {
    std::str::from_utf8(
        entries
            .get("word/document.xml")
            .ok_or_else(|| error("not a DOCX document"))?,
    )
    .map_err(error)
}
fn word_create(input: &Value) -> Result<Vec<u8>, ToolError> {
    let paragraphs = input["paragraphs"]
        .as_array()
        .ok_or_else(|| error("paragraphs must be an array"))?;
    if paragraphs.is_empty() || paragraphs.len() > 2000 {
        return Err(error("provide 1-2000 paragraphs"));
    }
    let mut body = String::new();
    for paragraph in paragraphs {
        let content = text(paragraph, "text")?;
        if content.len() > 65536 {
            return Err(error("paragraph exceeds 64 KiB"));
        }
        let heading = paragraph["heading"].as_u64().unwrap_or(0);
        if heading > 6 {
            return Err(error("heading must be between 0 and 6"));
        }
        let properties = if heading > 0 {
            format!("<w:pPr><w:pStyle w:val=\"Heading{heading}\"/></w:pPr>")
        } else {
            String::new()
        };
        let content = escape(content).replace('\n', "</w:t><w:br/><w:t xml:space=\"preserve\">");
        body.push_str(&format!(
            "<w:p>{properties}<w:r><w:t xml:space=\"preserve\">{content}</w:t></w:r></w:p>"
        ));
    }
    if let Some(rows) = input.get("table").and_then(Value::as_array) {
        if rows.len() > 1000 {
            return Err(error("table exceeds 1000 rows"));
        }
        body.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>");
        for row in rows {
            body.push_str("<w:tr>");
            for cell in row
                .as_array()
                .ok_or_else(|| error("table rows must be arrays"))?
            {
                body.push_str(&format!(
                    "<w:tc><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:tc>",
                    escape(
                        cell.as_str()
                            .ok_or_else(|| error("table cells must be strings"))?
                    )
                ));
            }
            body.push_str("</w:tr>");
        }
        body.push_str("</w:tbl>");
    }
    let mut entries = BTreeMap::new();
    entries.insert("[Content_Types].xml".into(),br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#.to_vec());
    entries.insert("_rels/.rels".into(),br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_vec());
    entries.insert("word/_rels/document.xml.rels".into(),br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#.to_vec());
    let styles=(1..=6).map(|level|format!("<w:style w:type=\"paragraph\" w:styleId=\"Heading{level}\"><w:name w:val=\"heading {level}\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:outlineLvl w:val=\"{}\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"{}\"/></w:rPr></w:style>",level-1,36-level*2)).collect::<String>();
    entries.insert("word/styles.xml".into(),format!("<w:styles xmlns:w=\"{WORD_NS}\"><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>{styles}</w:styles>").into_bytes());
    entries.insert("word/document.xml".into(),format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document xmlns:w=\"{WORD_NS}\"><w:body>{body}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>").into_bytes());
    pack(entries)
}
fn word_replace(
    xml: &str,
    find: &str,
    replacement: &str,
    expected: u64,
) -> Result<(String, usize), ToolError> {
    let doc = roxmltree::Document::parse(xml).map_err(error)?;
    let mut edits = Vec::new();
    let mut count = 0usize;
    for paragraph in doc
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "p")))
    {
        let nodes = paragraph
            .descendants()
            .filter(|node| {
                node.has_tag_name((WORD_NS, "t"))
                    && node
                        .ancestors()
                        .find(|ancestor| ancestor.has_tag_name((WORD_NS, "p")))
                        == Some(paragraph)
            })
            .collect::<Vec<_>>();
        let joined = nodes
            .iter()
            .map(|node| node.text().unwrap_or_default())
            .collect::<String>();
        let matches = joined
            .match_indices(find)
            .map(|(start, _)| start..start + find.len())
            .collect::<Vec<_>>();
        if matches.is_empty() {
            continue;
        }
        count += matches.len();
        let mut offset = 0;
        for node in nodes {
            let original = node.text().unwrap_or_default();
            let end = offset + original.len();
            let mut changed = original.to_string();
            for range in matches.iter().rev() {
                let begin = range.start.max(offset);
                let finish = range.end.min(end);
                if begin < finish {
                    changed.replace_range(
                        begin - offset..finish - offset,
                        if range.start >= offset && range.start < end {
                            replacement
                        } else {
                            ""
                        },
                    );
                }
            }
            if changed != original {
                let original_xml = &xml[node.range()];
                let close = original_xml
                    .find('>')
                    .ok_or_else(|| error("invalid text element"))?;
                let opening = &original_xml[..close];
                let space = if opening.contains("xml:space=") {
                    ""
                } else {
                    " xml:space=\"preserve\""
                };
                let end_tag = original_xml
                    .rfind("</")
                    .ok_or_else(|| error("invalid text element"))?;
                edits.push((
                    node.range(),
                    format!(
                        "{opening}{space}>{}{}",
                        escape(&changed),
                        &original_xml[end_tag..]
                    ),
                ));
            }
            offset = end;
        }
    }
    if count as u64 != expected {
        return Err(error(format!(
            "expected {expected} matches, found {count}; no file written"
        )));
    }
    let mut result = xml.to_string();
    edits.sort_by_key(|(range, _)| range.start);
    for (range, replacement) in edits.into_iter().rev() {
        result.replace_range(range, &replacement);
    }
    Ok((result, count))
}
pub(super) fn execute(
    name: &str,
    input: &Value,
    context: &AgentRunContext,
) -> Result<Value, ToolError> {
    fn valid_xml(value: &Value) -> bool {
        match value {
            Value::String(text) => text.chars().all(|c| {
                matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
            }),
            Value::Array(items) => items.iter().all(valid_xml),
            Value::Object(items) => items.values().all(valid_xml),
            _ => true,
        }
    }
    if !valid_xml(input) {
        return Err(error("Office text contains invalid XML control characters"));
    }
    if name.starts_with("excel_") {
        return excel(name, input, context);
    }
    let file = path(input, "path", context)?;
    if file.extension().and_then(|v| v.to_str()) != Some("docx") {
        return Err(error("Word tools support .docx files"));
    }
    if name == "word_create" {
        let bytes = word_create(input)?;
        write_new(&file, &bytes)?;
        return Ok(json!({"path":file,"created":true}));
    }
    let mut entries = package(&file)?;
    let xml = xml_document(&entries)?;
    if name == "word_read" {
        let doc = roxmltree::Document::parse(xml).map_err(error)?;
        let start = input["startParagraph"].as_u64().unwrap_or(0) as usize;
        let limit = input["limit"].as_u64().unwrap_or(100).clamp(1, 500) as usize;
        let paragraphs = doc
            .descendants()
            .filter(|node| node.has_tag_name((WORD_NS, "p")))
            .collect::<Vec<_>>();
        let mut bytes = 0usize;
        let mut result = Vec::new();
        for (index, paragraph) in paragraphs.iter().enumerate().skip(start).take(limit) {
            let content = paragraph
                .descendants()
                .filter(|node| {
                    node.ancestors()
                        .find(|ancestor| ancestor.has_tag_name((WORD_NS, "p")))
                        == Some(*paragraph)
                })
                .filter_map(|node| {
                    if node.has_tag_name((WORD_NS, "t")) {
                        node.text()
                    } else if node.has_tag_name((WORD_NS, "br")) {
                        Some("\n")
                    } else if node.has_tag_name((WORD_NS, "tab")) {
                        Some("\t")
                    } else {
                        None
                    }
                })
                .collect::<String>();
            if bytes + content.len() > MAX_OUTPUT_BYTES {
                if result.is_empty() {
                    return Err(error("paragraph exceeds output limit"));
                }
                break;
            }
            bytes += content.len();
            result.push(json!({"index":index,"text":content,"inTable":paragraph.ancestors().any(|node|node.has_tag_name((WORD_NS,"tbl")))}));
        }
        return Ok(
            json!({"path":file,"paragraphs":result,"totalParagraphs":paragraphs.len(),"nextParagraph":if start+result.len()<paragraphs.len(){Some(start+result.len())}else{None}}),
        );
    }
    let output = path(input, "outputPath", context)?;
    if output.extension().and_then(|v| v.to_str()) != Some("docx") {
        return Err(error("outputPath must end in .docx"));
    }
    let find = text(input, "find")?;
    let replacement = input["replace"]
        .as_str()
        .ok_or_else(|| error("replace must be a string"))?;
    let (updated, count) = word_replace(
        xml,
        find,
        replacement,
        input["expectedCount"].as_u64().unwrap_or(1),
    )?;
    entries.insert("word/document.xml".into(), updated.into_bytes());
    write_new(&output, &pack(entries)?)?;
    Ok(json!({"path":output,"replacements":count,"sourcePath":file}))
}

fn excel(name: &str, input: &Value, context: &AgentRunContext) -> Result<Value, ToolError> {
    let file = path(input, "path", context)?;
    if file.extension().and_then(|v| v.to_str()) != Some("xlsx") {
        return Err(error("Excel tools support .xlsx files"));
    }
    let mut book = if name == "excel_read" || input.get("outputPath").is_some() {
        let _ = package(&file)?; // Bound compressed and expanded package size before parsing.
        umya_spreadsheet::reader::xlsx::read(&file).map_err(error)?
    } else {
        umya_spreadsheet::new_file()
    };
    if name == "excel_read" {
        let sheets = book
            .sheet_collection()
            .iter()
            .map(|sheet| sheet.name().to_string())
            .collect::<Vec<_>>();
        let Some(sheet_name) = input.get("sheet").and_then(Value::as_str) else {
            return Ok(json!({"path":file,"sheets":sheets}));
        };
        let sheet = book.sheet_by_name(sheet_name).map_err(error)?;
        let row = input["startRow"].as_u64().unwrap_or(1);
        let rows = input["rows"].as_u64().unwrap_or(50).clamp(1, 200);
        let column = input["startColumn"].as_u64().unwrap_or(1);
        let columns = input["columns"].as_u64().unwrap_or(20).clamp(1, 100);
        if row < 1
            || row > 1_048_576
            || row + rows - 1 > 1_048_576
            || column < 1
            || column > 16_384
            || column + columns - 1 > 16_384
        {
            return Err(error("range exceeds worksheet bounds"));
        }
        let mut cells = Vec::new();
        let mut bytes = 0;
        for r in row..row + rows {
            for c in column..column + columns {
                if let Some(cell) = sheet.cell((c as u32, r as u32)) {
                    let value = cell.value().to_string();
                    let formula = cell.formula().to_string();
                    bytes += value.len() + formula.len();
                    if bytes > MAX_OUTPUT_BYTES {
                        return Err(error("range exceeds output limit; read a smaller range"));
                    }
                    cells.push(json!({"row":r,"column":c,"value":value,"formula":formula}));
                }
            }
        }
        return Ok(
            json!({"path":file,"sheet":sheet_name,"cells":cells,"formulaValues":"cached; not recalculated by this tool"}),
        );
    }
    let sheets = input["sheets"]
        .as_array()
        .ok_or_else(|| error("sheets must be an array"))?;
    if sheets.is_empty() || sheets.len() > 100 {
        return Err(error("provide 1-100 sheets"));
    }
    let mut count = 0;
    for specification in sheets {
        let sheet_name = text(specification, "name")?;
        if book.sheet_by_name(sheet_name).is_err() {
            book.new_sheet(sheet_name).map_err(error)?;
        }
        let sheet = book.sheet_by_name_mut(sheet_name).map_err(error)?;
        let cells = specification["cells"]
            .as_array()
            .ok_or_else(|| error("cells must be an array"))?;
        count += cells.len();
        if count > 10000 {
            return Err(error("write exceeds 10000 cells"));
        }
        for specification in cells {
            let row = specification["row"]
                .as_u64()
                .ok_or_else(|| error("row is required"))?;
            let column = specification["column"]
                .as_u64()
                .ok_or_else(|| error("column is required"))?;
            if !(1..=1_048_576).contains(&row) || !(1..=16_384).contains(&column) {
                return Err(error("cell exceeds worksheet bounds"));
            }
            let cell = sheet.cell_mut((column as u32, row as u32));
            if let Some(formula) = specification.get("formula").and_then(Value::as_str) {
                cell.set_formula(formula.trim_start_matches('='));
            } else {
                match &specification["value"] {
                    Value::String(value) => {
                        cell.set_value_string(value);
                    }
                    Value::Number(value) => {
                        cell.set_value_number(
                            value.as_f64().ok_or_else(|| error("invalid number"))?,
                        );
                    }
                    Value::Bool(value) => {
                        cell.set_value_bool(*value);
                    }
                    _ => return Err(error("cell requires a string, number, boolean, or formula")),
                }
            }
        }
    }
    let output = if input.get("outputPath").is_some() {
        path(input, "outputPath", context)?
    } else {
        file.clone()
    };
    if output.extension().and_then(|v| v.to_str()) != Some("xlsx") {
        return Err(error("outputPath must end in .xlsx"));
    }
    let parent = output
        .parent()
        .ok_or_else(|| error("output parent is required"))?;
    let temporary = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    umya_spreadsheet::writer::xlsx::write(&book, temporary.path()).map_err(error)?;
    // Formula values are not computed here; ask Excel to recalculate on opening.
    let mut entries = package(temporary.path())?;
    if let Some(data) = entries.get_mut("xl/workbook.xml") {
        let mut xml = String::from_utf8(data.clone()).map_err(error)?;
        let document = roxmltree::Document::parse(&xml).map_err(error)?;
        let range = document
            .descendants()
            .find(|node| node.tag_name().name() == "calcPr")
            .map(|node| node.range());
        let properties = "<calcPr calcMode=\"auto\" fullCalcOnLoad=\"1\" forceFullCalc=\"1\"/>";
        if let Some(range) = range {
            xml.replace_range(range, properties);
        } else if let Some(end) = xml.rfind("</workbook>") {
            xml.insert_str(end, properties);
        }
        *data = xml.into_bytes();
    }
    write_new(&output, &pack(entries)?)?;
    Ok(json!({"path":output,"cellsWritten":count,"formulasRecalculated":false}))
}
