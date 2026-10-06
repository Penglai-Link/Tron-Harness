//! Attach original Office coordinates without inventing physical page numbers.
use super::{
    KnowledgeBaseError,
    local_documents::{MAX_TEXT_BYTES, ParsedDocument, parse_xlsx, validate_office},
};
use serde_json::Value;

fn invalid(message: impl std::fmt::Display) -> KnowledgeBaseError {
    KnowledgeBaseError::Invalid(message.to_string())
}
fn canonical(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
}

pub(super) fn apply(
    parsed: &mut ParsedDocument,
    bytes: &[u8],
    middle: &Value,
) -> Result<(), KnowledgeBaseError> {
    // The parser port also supports test/local implementations without this vendor schema.
    if middle["schema"] != "docvortex.middle" {
        return Ok(());
    }
    match parsed.format.as_str() {
        "docx" => word(parsed, bytes)?,
        "xlsx" => excel(parsed, bytes, middle)?,
        _ => return Ok(()),
    }
    let mut offset = 0usize;
    let mut size = 0usize;
    for section in &mut parsed.sections {
        size = size.saturating_add(section.text.len());
        if size > MAX_TEXT_BYTES {
            return Err(invalid("解析文本超过2 MiB上限"));
        }
        section.location.char_start = offset;
        offset += section.text.chars().count();
        section.location.char_end = offset;
        offset += 1;
    }
    Ok(())
}
fn word(parsed: &mut ParsedDocument, bytes: &[u8]) -> Result<(), KnowledgeBaseError> {
    let xml = validate_office(bytes, Some("word/document.xml"))
        .map_err(invalid)?
        .ok_or_else(|| invalid("Word缺少正文结构"))?;
    let xml = std::str::from_utf8(&xml).map_err(invalid)?;
    let doc = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            nodes_limit: 300_000,
            ..Default::default()
        },
    )
    .map_err(invalid)?;
    let ns = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let paragraphs: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name((ns, "p")))
        .collect();
    let text: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            canonical(
                &p.descendants()
                    .filter(|n| n.has_tag_name((ns, "t")))
                    .filter_map(|n| n.text())
                    .collect::<String>(),
            )
        })
        .collect();
    let picture_paragraphs: Vec<u32> = paragraphs
        .iter()
        .enumerate()
        .flat_map(|(i, p)| {
            p.descendants()
                .filter(|n| {
                    n.has_tag_name((
                        "http://schemas.openxmlformats.org/drawingml/2006/main",
                        "blip",
                    ))
                })
                .map(move |_| i as u32 + 1)
        })
        .collect();
    let image_count = parsed
        .sections
        .iter()
        .filter(|s| s.location.block_kind.as_deref() == Some("image"))
        .count();
    let mut image_ordinal = 0;
    for section in &mut parsed.sections {
        section.location.page = None;
        if section.location.block_kind.as_deref() == Some("image") {
            // Only use sequence mapping when the native parser and original drawing counts agree.
            if image_count == picture_paragraphs.len() {
                section.location.paragraph = Some(picture_paragraphs[image_ordinal]);
                section.location.paragraph_end = section.location.paragraph;
            }
            image_ordinal += 1;
        } else if section.location.block_kind.as_deref() != Some("table") {
            let key = canonical(section.text.split("[图片资源:").next().unwrap_or(""));
            if !key.is_empty() {
                let matches: Vec<_> = text
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| *s == &key)
                    .map(|(i, _)| i as u32 + 1)
                    .collect();
                if matches.len() == 1 {
                    section.location.paragraph = Some(matches[0]);
                    section.location.paragraph_end = section.location.paragraph;
                }
            }
        }
    }
    Ok(())
}
fn excel(
    parsed: &mut ParsedDocument,
    bytes: &[u8],
    middle: &Value,
) -> Result<(), KnowledgeBaseError> {
    let xml = validate_office(bytes, Some("xl/workbook.xml"))
        .map_err(invalid)?
        .ok_or_else(|| invalid("Excel缺少工作表结构"))?;
    let xml = std::str::from_utf8(&xml).map_err(invalid)?;
    let workbook = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            nodes_limit: 300_000,
            ..Default::default()
        },
    )
    .map_err(invalid)?;
    let ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let sheets: Vec<_> = workbook
        .descendants()
        .filter(|n| n.has_tag_name((ns, "sheet")))
        .collect();
    if sheets
        .iter()
        .any(|n| n.attribute("state").is_some_and(|s| s != "visible"))
    {
        return Err(invalid(
            "当前解析策略不完整处理隐藏工作表，请取消隐藏后重新上传",
        ));
    }
    let names: Vec<_> = sheets
        .iter()
        .map(|n| {
            n.attribute("name")
                .map(str::to_owned)
                .ok_or_else(|| invalid("Excel工作表名称缺失"))
        })
        .collect::<Result<_, _>>()?;
    if middle["pages"].as_array().map_or(0, Vec::len) != names.len() {
        return Err(invalid("Excel工作表定位不匹配，索引未发布"));
    }
    for section in &mut parsed.sections {
        section.location.page = None;
        let index = section
            .location
            .logical_page
            .ok_or_else(|| invalid("Excel工作表序号缺失"))? as usize;
        section.location.sheet = Some(
            names
                .get(index)
                .ok_or_else(|| invalid("Excel工作表序号无效"))?
                .clone(),
        );
    }
    // Cell-level companions retain exact original addresses independently of vendor table clustering.
    let cells = parse_xlsx(bytes).map_err(invalid)?;
    parsed.sections.extend(cells.sections);
    parsed.warnings.extend(cells.warnings);
    parsed.warnings.sort();
    parsed.warnings.dedup();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{local_documents::split_document, parsing::normalize_middle};
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;
    fn middle(format: &str, count: usize) -> Value {
        json!({"schema":"docvortex.middle","schema_version":"2.0","metadata":{"file_suffix":format},"extensions":{"mineru":{"tier":"flash"}},"pages":(0..count).map(|i|json!({"page_idx":i,"blocks":[{"type":"text","index":0,"content":[{"type":"text","content":"SYNTHETIC ONLY — ORBIT REPORT"}]}]})).collect::<Vec<_>>()})
    }
    #[test]
    fn word_uses_original_unique_paragraphs_without_fabricated_pages() {
        let m = middle("docx", 1);
        let mut parsed = normalize_middle(&m, &BTreeSet::new()).unwrap();
        apply(
            &mut parsed,
            include_bytes!("../../../tests/fixtures/mineru-v1/synthetic.docx"),
            &m,
        )
        .unwrap();
        assert_eq!(parsed.sections[0].location.page, None);
        assert_eq!(parsed.sections[0].location.paragraph, Some(1));
        let chunks = split_document(&parsed, &Default::default()).unwrap();
        assert_eq!(chunks[0].location.page, None);
        assert_eq!(chunks[0].location.paragraph, Some(1));
    }
    #[test]
    fn excel_preserves_multiple_sheet_names_and_exact_sparse_cell_addresses() {
        let mut book = umya_spreadsheet::new_file();
        book.sheet_by_name_mut("Sheet1")
            .unwrap()
            .cell_mut("B7")
            .set_value("cobalt-orbit");
        book.new_sheet("Revenue Q4").unwrap();
        book.sheet_by_name_mut("Revenue Q4")
            .unwrap()
            .cell_mut("D12")
            .set_value("240");
        let mut bytes = Vec::new();
        umya_spreadsheet::writer::xlsx::write_writer(&book, &mut bytes).unwrap();
        let m = middle("xlsx", 2);
        let mut parsed = normalize_middle(&m, &BTreeSet::new()).unwrap();
        apply(&mut parsed, &bytes, &m).unwrap();
        assert!(parsed.sections.iter().all(|s| s.location.page.is_none()));
        assert_eq!(parsed.sections[0].location.sheet.as_deref(), Some("Sheet1"));
        assert_eq!(
            parsed.sections[1].location.sheet.as_deref(),
            Some("Revenue Q4")
        );
        let exact = parsed
            .sections
            .iter()
            .find(|s| s.text.contains("Revenue Q4!D12: 240"))
            .unwrap();
        assert_eq!(exact.location.cell_ranges, vec!["D12"]);
        assert_eq!(exact.location.row_start, Some(12));
        assert!(
            parsed
                .sections
                .iter()
                .any(|s| s.text.contains("Sheet1!B7: cobalt-orbit")
                    && s.location.cell_ranges == vec!["B7"])
        );
        let mut bad = middle("xlsx", 1);
        let mut parsed = normalize_middle(&bad, &BTreeSet::new()).unwrap();
        assert!(apply(&mut parsed, &bytes, &bad).is_err());
        bad["pages"] = json!([]);
    }
}
