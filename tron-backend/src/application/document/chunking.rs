//! Tron maps normalized source facts to Framework strategies and restores product citations.
use super::*;
use eliteforge_agent_core::{
    ChunkBlock, ChunkBlockKind, ChunkDocument, ChunkPlan, ChunkSourceSpan, ChunkStrategies,
    ChunkTable, ChunkTextRange, ChunkingOptions,
};

pub struct ChunkedDocument {
    pub chunks: Vec<LocatedChunk>,
    pub contexts: Vec<eliteforge_agent_core::ChunkContext>,
    pub warnings: Vec<String>,
}

pub fn chunk_document_input(
    document: &ParsedDocument,
    id: &str,
    title: &str,
    depth: u8,
) -> ChunkDocument {
    let source_locations = source_fact_locations(document);
    let mut headings: Vec<(u8, String)> = Vec::new();
    let blocks = document
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            if let Some(level) = section.heading_level {
                headings.retain(|(previous, _)| *previous < level);
                headings.push((
                    level,
                    section.location.heading.clone().unwrap_or_else(|| {
                        section
                            .text
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .trim_matches('#')
                            .trim()
                            .to_owned()
                    }),
                ));
            }
            let kind = if section.heading_level.is_some() {
                ChunkBlockKind::Heading
            } else {
                match section.location.block_kind.as_deref() {
                    Some("table") => ChunkBlockKind::Table,
                    Some("list" | "list_item") => ChunkBlockKind::List,
                    Some("image") => ChunkBlockKind::Image,
                    Some("code") => ChunkBlockKind::Code,
                    _ if section
                        .text
                        .lines()
                        .take(3)
                        .any(|line| line.trim_start().starts_with('|')) =>
                    {
                        ChunkBlockKind::Table
                    }
                    _ => ChunkBlockKind::Paragraph,
                }
            };
            ChunkBlock {
                id: format!("block-{index}"),
                text: section.text.clone(),
                kind,
                heading_path: if section
                    .location
                    .source_block_id
                    .as_deref()
                    .is_some_and(|id| id.starts_with("unmapped-asset/"))
                {
                    vec![]
                } else {
                    headings
                        .iter()
                        .filter(|(level, _)| *level <= depth)
                        .map(|(_, title)| title.clone())
                        .collect()
                },
                // An unlinked image must never inherit a neighbouring paragraph's context.
                boundary: if kind == ChunkBlockKind::Image {
                    Some(
                        section
                            .location
                            .source_block_id
                            .clone()
                            .unwrap_or_else(|| format!("image-{index}")),
                    )
                } else {
                    section.location.sheet.clone()
                },
                table: if kind == ChunkBlockKind::Table {
                    table_ranges(&section.text)
                } else {
                    None
                },
                location: source_locations[index].clone(),
            }
        })
        .collect();
    ChunkDocument {
        id: id.into(),
        title: title.into(),
        blocks,
    }
}

fn table_ranges(text: &str) -> Option<ChunkTable> {
    if text.to_ascii_lowercase().contains("<table") {
        return html_table_ranges(text);
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let end = start + line.chars().count();
        if !line.trim().is_empty() {
            ranges.push((ChunkTextRange { start, end }, line.trim()));
        }
        start = end;
    }
    let first = ranges.iter().position(|(_, line)| line.starts_with('|'))?;
    let last = ranges.iter().rposition(|(_, line)| line.starts_with('|'))?;
    if first >= last {
        return None;
    }
    let header_end = ranges
        .get(first + 1)
        .filter(|(_, line)| line.chars().all(|c| "|:- \t".contains(c)))
        .map(|(r, _)| r.end)
        .unwrap_or(ranges[first].0.end);
    let rows = ranges
        .iter()
        .skip(first + 1)
        .take(last - first)
        .filter(|(r, _)| r.start >= header_end)
        .map(|(r, _)| *r)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return None;
    }
    Some(ChunkTable {
        header: ChunkTextRange {
            start: 0,
            end: header_end,
        },
        rows,
        footer: (ranges[last].0.end < text.chars().count()).then_some(ChunkTextRange {
            start: ranges[last].0.end,
            end: text.chars().count(),
        }),
    })
}

pub fn split_document_with_strategies(
    document: &ParsedDocument,
    config: &ChunkConfig,
    strategies: &ChunkStrategies,
    id: &str,
    title: &str,
) -> Result<ChunkedDocument, LocalDocumentError> {
    config.validate()?;
    let size: usize = document
        .sections
        .iter()
        .map(|section| section.text.len())
        .sum();
    if size > MAX_PARSED_TEXT_BYTES {
        return Err(failed("document.parse.normalized_too_large"));
    }
    let input = chunk_document_input(document, id, title, config.heading_depth);
    let options = ChunkingOptions {
        max_chars: config.max_chars,
        overlap_chars: config.overlap_chars,
        max_chunks: if size > MAX_TEXT_BYTES {
            10_000
        } else {
            MAX_CHUNKS
        },
        max_context_chars: (config.max_chars * 4).min(32000),
    };
    let plan = strategies
        .split(config.mode.strategy_id(), &input, &options)
        .map_err(failed_detail)?;
    located_plan(document, plan)
}

fn located_plan(
    document: &ParsedDocument,
    plan: ChunkPlan,
) -> Result<ChunkedDocument, LocalDocumentError> {
    let source_locations = source_fact_locations(document);
    let chunks = plan
        .chunks
        .into_iter()
        .enumerate()
        .map(|(ordinal, chunk)| {
            let indexes = chunk
                .spans
                .iter()
                .filter_map(|span| span.block_id.strip_prefix("block-")?.parse::<usize>().ok())
                .collect::<Vec<_>>();
            let first_index = *indexes
                .first()
                .ok_or_else(|| failed("document.chunk.source_missing"))?;
            let first = document
                .sections
                .get(first_index)
                .ok_or_else(|| failed("document.chunk.source_invalid"))?;
            let last = &document.sections[*indexes.last().unwrap()];
            let mut location = first.location.clone();
            location.char_start += chunk.spans.first().map_or(0, |span| span.start);
            location.char_end =
                last.location.char_start + chunk.spans.last().map_or(0, |span| span.end);
            location.pages = indexes
                .iter()
                .filter_map(|&i| document.sections[i].location.page)
                .collect();
            location.pages.sort_unstable();
            location.pages.dedup();
            location.blocks = indexes
                .iter()
                .flat_map(|&i| document.sections[i].location.blocks.clone())
                .collect();
            location.blocks.sort();
            location.blocks.dedup();
            location.image_resource_codes = indexes
                .iter()
                .flat_map(|&i| document.sections[i].location.image_resource_codes.clone())
                .collect();
            location.image_resource_codes.sort();
            location.image_resource_codes.dedup();
            location.cell_ranges = indexes
                .iter()
                .flat_map(|&i| document.sections[i].location.cell_ranges.clone())
                .collect();
            location.cell_ranges.sort();
            location.cell_ranges.dedup();
            location.image_annotations = indexes
                .iter()
                .flat_map(|&i| document.sections[i].location.image_annotations.clone())
                .collect();
            location.row_end = last.location.row_end;
            location.paragraph_end = last.location.paragraph_end;
            location.line_end = last.location.line_end;
            let source_locations = chunk
                .spans
                .iter()
                .map(|span| span_location_from(document, &source_locations, span))
                .collect::<Result<Vec<_>, _>>()?;
            let title = chunk
                .heading_path
                .last()
                .cloned()
                .or_else(|| location.sheet.clone())
                .or_else(|| location.page.map(|page| format!("Page {page}")))
                .unwrap_or_else(|| "Document".into());
            Ok(LocatedChunk {
                text: chunk.text,
                title,
                heading_path: chunk.heading_path,
                location,
                ordinal,
                index_text: chunk.index_text,
                source_spans: chunk.spans,
                source_locations,
                context_id: chunk.context_id,
            })
        })
        .collect::<Result<Vec<_>, LocalDocumentError>>()?;
    Ok(ChunkedDocument {
        chunks,
        contexts: plan.contexts,
        warnings: plan.warnings,
    })
}

pub fn span_location(
    document: &ParsedDocument,
    span: &ChunkSourceSpan,
) -> Result<serde_json::Value, LocalDocumentError> {
    span_location_from(document, &source_fact_locations(document), span)
}
fn span_location_from(
    document: &ParsedDocument,
    locations: &[serde_json::Value],
    span: &ChunkSourceSpan,
) -> Result<serde_json::Value, LocalDocumentError> {
    let index = span
        .block_id
        .strip_prefix("block-")
        .and_then(|id| id.parse::<usize>().ok())
        .ok_or_else(|| failed("document.chunk.source_block_invalid"))?;
    let section = document
        .sections
        .get(index)
        .ok_or_else(|| failed("document.chunk.source_block_absent"))?;
    let mut location = locations
        .get(index)
        .cloned()
        .ok_or_else(|| failed("document.chunk.source_location_absent"))?;
    location["charStart"] = (section.location.char_start + span.start).into();
    location["charEnd"] = (section.location.char_start + span.end).into();
    Ok(location)
}

pub fn split_document(
    document: &ParsedDocument,
    config: &ChunkConfig,
) -> Result<Vec<LocatedChunk>, LocalDocumentError> {
    split_document_with_strategies(
        document,
        config,
        &eliteforge_agent_capability_chunking::built_in_chunk_strategies(),
        "document",
        "",
    )
    .map(|plan| plan.chunks)
}

// HTML remains original source text. Ranges refer to complete rows, preserving
// captions, units and footnotes without inventing or re-parsing provider content.
fn html_table_ranges(text: &str) -> Option<ChunkTable> {
    let lower = text.to_ascii_lowercase();
    let mut rows = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("<tr") {
        let begin = cursor + relative;
        let end = begin + lower[begin..].find("</tr>")? + 5;
        rows.push(ChunkTextRange {
            start: text[..begin].chars().count(),
            end: text[..end].chars().count(),
        });
        cursor = end;
    }
    if rows.len() < 2 {
        return None;
    }
    let header_end = if let Some(end) = lower.find("</thead>") {
        text[..end + 8].chars().count()
    } else {
        rows[0].end
    };
    rows.retain(|row| row.start >= header_end);
    let last = rows.last()?.end;
    Some(ChunkTable {
        header: ChunkTextRange {
            start: 0,
            end: header_end,
        },
        rows,
        footer: (last < text.chars().count()).then_some(ChunkTextRange {
            start: last,
            end: text.chars().count(),
        }),
    })
}
