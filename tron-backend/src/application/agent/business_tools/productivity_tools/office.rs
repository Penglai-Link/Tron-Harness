use super::super::file_access::resolve_input_file;
use super::*;
use crate::application::resource::ResourceService;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};
use tron_i18n::t;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn package(bytes: &[u8], locale: &str) -> Result<BTreeMap<String, Vec<u8>>, ToolError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(error)?;
    if zip.len() > 4096 {
        return Err(error(
            t!(
                "agent_runtime.tool.office.package_too_many_entries",
                locale = locale
            )
            .into_owned(),
        ));
    }
    let mut total = 0u64;
    let mut entries = BTreeMap::new();
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(error)?;
        total = total.checked_add(file.size()).ok_or_else(|| {
            error(
                t!(
                    "agent_runtime.tool.office.package_too_large",
                    locale = locale
                )
                .into_owned(),
            )
        })?;
        if total > 64 * 1024 * 1024 {
            return Err(error(
                t!(
                    "agent_runtime.tool.office.package_expanded_too_large",
                    locale = locale
                )
                .into_owned(),
            ));
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
            return Err(error(
                t!("agent_runtime.tool.office.entry_too_large", locale = locale).into_owned(),
            ));
        }
        if entries.insert(name, data).is_some() {
            return Err(error(
                t!("agent_runtime.tool.office.duplicate_entry", locale = locale).into_owned(),
            ));
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
fn xml_document<'a>(
    entries: &'a BTreeMap<String, Vec<u8>>,
    locale: &str,
) -> Result<&'a str, ToolError> {
    std::str::from_utf8(entries.get("word/document.xml").ok_or_else(|| {
        error(t!("agent_runtime.tool.office.not_a_docx", locale = locale).into_owned())
    })?)
    .map_err(error)
}
fn word_create(input: &Value, locale: &str) -> Result<Vec<u8>, ToolError> {
    let paragraphs = input["paragraphs"].as_array().ok_or_else(|| {
        error(
            t!(
                "agent_runtime.tool.office.paragraphs_not_array",
                locale = locale
            )
            .into_owned(),
        )
    })?;
    if paragraphs.is_empty() || paragraphs.len() > 2000 {
        return Err(error(
            t!(
                "agent_runtime.tool.office.paragraphs_range",
                locale = locale
            )
            .into_owned(),
        ));
    }
    let mut body = String::new();
    for paragraph in paragraphs {
        let content = text(paragraph, "text", locale)?;
        if content.len() > 65536 {
            return Err(error(
                t!(
                    "agent_runtime.tool.office.paragraph_too_large",
                    locale = locale
                )
                .into_owned(),
            ));
        }
        let heading = paragraph["heading"].as_u64().unwrap_or(0);
        if heading > 6 {
            return Err(error(
                t!("agent_runtime.tool.office.heading_range", locale = locale).into_owned(),
            ));
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
            return Err(error(
                t!(
                    "agent_runtime.tool.office.table_rows_too_many",
                    locale = locale
                )
                .into_owned(),
            ));
        }
        body.push_str("<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr>");
        for row in rows {
            body.push_str("<w:tr>");
            for cell in row.as_array().ok_or_else(|| {
                error(
                    t!(
                        "agent_runtime.tool.office.table_rows_not_array",
                        locale = locale
                    )
                    .into_owned(),
                )
            })? {
                body.push_str(&format!(
                    "<w:tc><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:tc>",
                    escape(cell.as_str().ok_or_else(|| {
                        error(
                            t!(
                                "agent_runtime.tool.office.table_cells_not_strings",
                                locale = locale
                            )
                            .into_owned(),
                        )
                    })?)
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
    locale: &str,
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
                let close = original_xml.find('>').ok_or_else(|| {
                    error(
                        t!(
                            "agent_runtime.tool.office.invalid_text_element",
                            locale = locale
                        )
                        .into_owned(),
                    )
                })?;
                let opening = &original_xml[..close];
                let space = if opening.contains("xml:space=") {
                    ""
                } else {
                    " xml:space=\"preserve\""
                };
                let end_tag = original_xml.rfind("</").ok_or_else(|| {
                    error(
                        t!(
                            "agent_runtime.tool.office.invalid_text_element",
                            locale = locale
                        )
                        .into_owned(),
                    )
                })?;
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
        return Err(error(t!(
            "agent_runtime.tool.office.replacement_count_mismatch",
            locale = locale,
            expected = expected,
            count = count
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
    resources: &ResourceService,
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
    let locale = context.locale.as_str();
    if !valid_xml(input) {
        return Err(error(
            t!(
                "agent_runtime.tool.office.invalid_xml_characters",
                locale = locale
            )
            .into_owned(),
        ));
    }
    if name.starts_with("excel_") {
        return excel(name, input, context, resources, locale);
    }
    if name == "word_create" {
        let file = path(input, "path", context)?;
        if file.extension().and_then(|v| v.to_str()) != Some("docx") {
            return Err(error(
                t!(
                    "agent_runtime.tool.office.word_requires_docx",
                    locale = locale
                )
                .into_owned(),
            ));
        }
        let bytes = word_create(input, locale)?;
        write_new(&file, &bytes, locale)?;
        return Ok(json!({"path":file,"created":true}));
    }
    let source = resolve_input_file(input, context, resources)?;
    if Path::new(&source.file_name)
        .extension()
        .and_then(|value| value.to_str())
        != Some("docx")
    {
        return Err(error(
            t!(
                "agent_runtime.tool.office.word_requires_docx",
                locale = locale
            )
            .into_owned(),
        ));
    }
    let mut entries = package(&source.bytes, locale)?;
    let xml = xml_document(&entries, locale)?;
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
                    return Err(error(
                        t!(
                            "agent_runtime.tool.office.paragraph_output_limit",
                            locale = locale
                        )
                        .into_owned(),
                    ));
                }
                break;
            }
            bytes += content.len();
            result.push(json!({"index":index,"text":content,"inTable":paragraph.ancestors().any(|node|node.has_tag_name((WORD_NS,"tbl")))}));
        }
        return Ok(
            json!({"path":source.workspace_path,"fileName":source.file_name,"fileRef":source.reference,"paragraphs":result,"totalParagraphs":paragraphs.len(),"nextParagraph":if start+result.len()<paragraphs.len(){Some(start+result.len())}else{None}}),
        );
    }
    let output = path(input, "outputPath", context)?;
    if output.extension().and_then(|v| v.to_str()) != Some("docx") {
        return Err(error(
            t!(
                "agent_runtime.tool.office.word_output_extension",
                locale = locale
            )
            .into_owned(),
        ));
    }
    let find = text(input, "find", locale)?;
    let replacement = input["replace"].as_str().ok_or_else(|| {
        error(
            t!(
                "agent_runtime.tool.office.replace_not_string",
                locale = locale
            )
            .into_owned(),
        )
    })?;
    let (updated, count) = word_replace(
        xml,
        find,
        replacement,
        input["expectedCount"].as_u64().unwrap_or(1),
        locale,
    )?;
    entries.insert("word/document.xml".into(), updated.into_bytes());
    write_new(&output, &pack(entries)?, locale)?;
    Ok(
        json!({"path":output,"replacements":count,"sourcePath":source.workspace_path,"sourceFileName":source.file_name,"sourceFileRef":source.reference}),
    )
}

fn excel(
    name: &str,
    input: &Value,
    context: &AgentRunContext,
    resources: &ResourceService,
    locale: &str,
) -> Result<Value, ToolError> {
    let is_read = name == "excel_read";
    let has_source = is_read || input.get("fileRef").is_some() || input.get("outputPath").is_some();
    let source = has_source
        .then(|| resolve_input_file(input, context, resources))
        .transpose()?;
    if is_read && source.is_none() {
        return Err(error(
            t!("agent_runtime.tool.office.source_required", locale = locale).into_owned(),
        ));
    }
    if source.as_ref().is_some_and(|source| {
        Path::new(&source.file_name)
            .extension()
            .and_then(|value| value.to_str())
            != Some("xlsx")
    }) {
        return Err(error(
            t!(
                "agent_runtime.tool.office.excel_requires_xlsx",
                locale = locale
            )
            .into_owned(),
        ));
    }
    if name == "excel_write" && input.get("fileRef").is_some() && input.get("outputPath").is_none()
    {
        return Err(error(
            t!(
                "agent_runtime.tool.office.excel_output_required",
                locale = locale
            )
            .into_owned(),
        ));
    }
    let mut attachment_temp = None;
    let input_path = if let Some(source) = source.as_ref() {
        let _ = package(&source.bytes, locale)?; // Bound compressed and expanded package size before parsing.
        if let Some(path) = source.workspace_path.as_ref() {
            Some(path.clone())
        } else {
            let mut temporary = tempfile::Builder::new()
                .suffix(".xlsx")
                .tempfile()
                .map_err(error)?;
            std::io::Write::write_all(&mut temporary, &source.bytes).map_err(error)?;
            let path = temporary.path().to_path_buf();
            attachment_temp = Some(temporary);
            Some(path)
        }
    } else {
        None
    };
    let mut book = if let Some(path) = input_path.as_ref() {
        umya_spreadsheet::reader::xlsx::read(path).map_err(error)?
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
            let source = source.as_ref().expect("excel_read source was required");
            return Ok(
                json!({"path":source.workspace_path,"fileName":source.file_name,"fileRef":source.reference,"sheets":sheets}),
            );
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
            return Err(error(
                t!(
                    "agent_runtime.tool.office.range_out_of_bounds",
                    locale = locale
                )
                .into_owned(),
            ));
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
                        return Err(error(
                            t!(
                                "agent_runtime.tool.office.range_output_limit",
                                locale = locale
                            )
                            .into_owned(),
                        ));
                    }
                    cells.push(json!({"row":r,"column":c,"value":value,"formula":formula}));
                }
            }
        }
        let source = source.as_ref().expect("excel_read source was required");
        return Ok(
            json!({"path":source.workspace_path,"fileName":source.file_name,"fileRef":source.reference,"sheet":sheet_name,"cells":cells,"formulaValues":"cached; not recalculated by this tool"}),
        );
    }
    let sheets = input["sheets"].as_array().ok_or_else(|| {
        error(
            t!(
                "agent_runtime.tool.office.sheets_not_array",
                locale = locale
            )
            .into_owned(),
        )
    })?;
    if sheets.is_empty() || sheets.len() > 100 {
        return Err(error(
            t!("agent_runtime.tool.office.sheets_range", locale = locale).into_owned(),
        ));
    }
    let mut count = 0;
    for specification in sheets {
        let sheet_name = text(specification, "name", locale)?;
        if book.sheet_by_name(sheet_name).is_err() {
            book.new_sheet(sheet_name).map_err(error)?;
        }
        let sheet = book.sheet_by_name_mut(sheet_name).map_err(error)?;
        let cells = specification["cells"].as_array().ok_or_else(|| {
            error(t!("agent_runtime.tool.office.cells_not_array", locale = locale).into_owned())
        })?;
        count += cells.len();
        if count > 10000 {
            return Err(error(
                t!(
                    "agent_runtime.tool.office.write_cells_limit",
                    locale = locale
                )
                .into_owned(),
            ));
        }
        for specification in cells {
            let row = specification["row"].as_u64().ok_or_else(|| {
                error(t!("agent_runtime.tool.office.row_required", locale = locale).into_owned())
            })?;
            let column = specification["column"].as_u64().ok_or_else(|| {
                error(t!("agent_runtime.tool.office.column_required", locale = locale).into_owned())
            })?;
            if !(1..=1_048_576).contains(&row) || !(1..=16_384).contains(&column) {
                return Err(error(
                    t!(
                        "agent_runtime.tool.office.cell_out_of_bounds",
                        locale = locale
                    )
                    .into_owned(),
                ));
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
                        cell.set_value_number(value.as_f64().ok_or_else(|| {
                            error(
                                t!("agent_runtime.tool.office.invalid_number", locale = locale)
                                    .into_owned(),
                            )
                        })?);
                    }
                    Value::Bool(value) => {
                        cell.set_value_bool(*value);
                    }
                    _ => {
                        return Err(error(
                            t!(
                                "agent_runtime.tool.office.cell_value_invalid",
                                locale = locale
                            )
                            .into_owned(),
                        ));
                    }
                }
            }
        }
    }
    let output = if input.get("outputPath").is_some() {
        path(input, "outputPath", context)?
    } else {
        path(input, "path", context)?
    };
    if output.extension().and_then(|v| v.to_str()) != Some("xlsx") {
        return Err(error(
            t!(
                "agent_runtime.tool.office.excel_output_extension",
                locale = locale
            )
            .into_owned(),
        ));
    }
    let parent = output.parent().ok_or_else(|| {
        error(
            t!(
                "agent_runtime.tool.office.output_parent_required",
                locale = locale
            )
            .into_owned(),
        )
    })?;
    let temporary = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    umya_spreadsheet::writer::xlsx::write(&book, temporary.path()).map_err(error)?;
    // Formula values are not computed here; ask Excel to recalculate on opening.
    let output_bytes = read_file(temporary.path(), locale)?;
    let mut entries = package(&output_bytes, locale)?;
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
    write_new(&output, &pack(entries)?, locale)?;
    drop(attachment_temp);
    Ok(json!({"path":output,"cellsWritten":count,"formulasRecalculated":false}))
}
