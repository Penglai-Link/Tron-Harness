//! Attach original Office coordinates without inventing physical page numbers.
use super::{
    KnowledgeBaseError,
    local_documents::{MAX_PARSED_TEXT_BYTES, ParsedDocument, parse_xlsx, validate_office},
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
        if size > MAX_PARSED_TEXT_BYTES {
            return Err(invalid("解析文本超过16 MiB上限"));
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
