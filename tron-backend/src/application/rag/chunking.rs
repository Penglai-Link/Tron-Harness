//! One source-located splitter for normalized documents and structured records.
use crate::application::knowledge_base::local_documents::{
    ChunkConfig, ChunkMode, LocalDocumentError, LocatedChunk, LocatedSection, MAX_CHUNKS,
    MAX_TEXT_BYTES, ParsedDocument,
};
fn failed(message: impl std::fmt::Display) -> LocalDocumentError {
    LocalDocumentError(message.to_string())
}
/// Split canonical text on heading/page/sheet boundaries and enforce a hard Unicode character bound.
/// Fixed mode ignores headings but still preserves page/sheet boundaries for reliable citations.
pub fn split_document(
    document: &ParsedDocument,
    config: &ChunkConfig,
) -> Result<Vec<LocatedChunk>, LocalDocumentError> {
    config.validate()?;
    let mut chunks = Vec::new();
    let mut group: Vec<&LocatedSection> = Vec::new();
    let mut group_path = Vec::new();
    let mut heading_stack: Vec<(u8, String)> = Vec::new();
    for current in &document.sections {
        let source_boundary = group.last().is_some_and(|previous| {
            previous.location.page != current.location.page
                || previous.location.sheet != current.location.sheet
                || previous.location.rendered_page != current.location.rendered_page
        });
        let heading_boundary = config.mode != ChunkMode::Fixed
            && current
                .heading_level
                .is_some_and(|level| level <= config.heading_depth);
        if (source_boundary || heading_boundary) && !group.is_empty() {
            emit_group(&group, &group_path, config, &mut chunks)?;
            group.clear();
        }
        if let Some(level) = current.heading_level {
            heading_stack.retain(|(previous, _)| *previous < level);
            heading_stack.push((level, current.location.heading.clone().unwrap_or_default()));
        }
        if group.is_empty() {
            group_path = heading_stack.iter().map(|(_, text)| text.clone()).collect();
        }
        group.push(current);
    }
    if !group.is_empty() {
        emit_group(&group, &group_path, config, &mut chunks)?;
    }
    Ok(chunks)
}
fn emit_group(
    group: &[&LocatedSection],
    path: &[String],
    config: &ChunkConfig,
    chunks: &mut Vec<LocatedChunk>,
) -> Result<(), LocalDocumentError> {
    let text = group
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if text.len() > MAX_TEXT_BYTES {
        return Err(failed("normalized document text exceeds 2 MiB"));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut source_ranges = Vec::new();
    let mut offset = 0;
    for section in group {
        let end = offset + section.text.chars().count();
        source_ranges.push((offset, end, *section));
        offset = end + 1;
    }
    let mut start = 0;
    while start < chars.len() {
        if chunks.len() >= MAX_CHUNKS {
            return Err(failed(
                "document exceeds 2000 chunks; increase maxChars or split the document",
            ));
        }
        let mut end = (start + config.max_chars).min(chars.len());
        if config.mode != ChunkMode::Fixed && end < chars.len() {
            // Prefer a natural boundary in the final quarter without risking tiny chunks.
            if let Some(boundary) = ((start + config.max_chars * 3 / 4)..end)
                .rev()
                .find(|&index| chars[index] == '\n')
            {
                end = boundary + 1;
            }
        }
        let text: String = chars[start..end].iter().collect();
        if !text.trim().is_empty() {
            let covered: Vec<_> = source_ranges
                .iter()
                .filter(|(begin, finish, _)| *finish > start && *begin < end)
                .collect();
            let first = covered
                .first()
                .map(|(_, _, section)| *section)
                .unwrap_or(group[0]);
            let last = covered
                .last()
                .map(|(_, _, section)| *section)
                .unwrap_or(first);
            let mut location = first.location.clone();
            location.char_start = group[0].location.char_start + start;
            location.char_end = group[0].location.char_start + end;
            location.blocks = covered
                .iter()
                .flat_map(|(_, _, section)| section.location.blocks.iter().cloned())
                .collect();
            location.blocks.sort();
            location.blocks.dedup();
            location.image_resource_codes = covered
                .iter()
                .flat_map(|(_, _, section)| section.location.image_resource_codes.iter().cloned())
                .collect();
            location.cell_ranges = covered
                .iter()
                .flat_map(|(_, _, section)| section.location.cell_ranges.iter().cloned())
                .collect();
            location.cell_ranges.sort();
            location.cell_ranges.dedup();
            location.image_resource_codes.sort();
            location.image_resource_codes.dedup();
            location.image_annotations = covered
                .iter()
                .flat_map(|(_, _, section)| section.location.image_annotations.iter().cloned())
                .collect();
            let mut unique = Vec::new();
            for annotation in location.image_annotations.drain(..) {
                if !unique.contains(&annotation) {
                    unique.push(annotation);
                }
            }
            location.image_annotations = unique;
            location.row_end = last.location.row_end;
            location.paragraph_end = last.location.paragraph_end;
            location.line_end = last.location.line_end;
            let title = path
                .last()
                .cloned()
                .or_else(|| location.sheet.clone())
                .or_else(|| location.page.map(|page| format!("Page {page}")))
                .unwrap_or_else(|| "Document".into());
            chunks.push(LocatedChunk {
                text,
                title,
                heading_path: path.to_vec(),
                location,
                ordinal: chunks.len(),
            });
        }
        if end == chars.len() {
            break;
        }
        start = end - config.overlap_chars;
    }
    Ok(())
}
