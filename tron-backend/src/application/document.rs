//! Local-only document extraction and bounded, source-located splitting.
//! No network clients, model calls, jobs, or user-resource writes belong here.
pub mod preflight;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::ZipArchive;

pub const MAX_RAW_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_EXPANDED_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
/// Bounded normalized output from a document parser; independent of local text uploads.
pub const MAX_PARSED_TEXT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CHUNKS: usize = 2_000;
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDocumentError(pub String);
impl fmt::Display for LocalDocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LocalDocumentError {}
fn failed(message: impl fmt::Display) -> LocalDocumentError {
    LocalDocumentError(message.to_string())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ChunkMode {
    #[default]
    Auto,
    Heading,
    Fixed,
    Structure,
    Custom(String),
}
impl ChunkMode {
    pub fn strategy_id(&self) -> &str {
        match self {
            Self::Auto => "auto",
            Self::Heading => "heading",
            Self::Fixed => "fixed",
            Self::Structure => "structure",
            Self::Custom(id) => id,
        }
    }
}
impl Serialize for ChunkMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Auto => "AUTO",
            Self::Heading => "HEADING",
            Self::Fixed => "FIXED",
            Self::Structure => "Structure",
            Self::Custom(id) => id,
        })
    }
}
impl<'de> Deserialize<'de> for ChunkMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Ok(match id.as_str() {
            "AUTO" | "Auto" | "auto" => Self::Auto,
            "HEADING" | "Heading" | "heading" => Self::Heading,
            "FIXED" | "Fixed" | "fixed" => Self::Fixed,
            "STRUCTURE" | "Structure" | "structure" => Self::Structure,
            _ => Self::Custom(id),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ChunkConfig {
    pub mode: ChunkMode,
    pub max_chars: usize,
    pub overlap_chars: usize,
    pub heading_depth: u8,
}
impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            mode: ChunkMode::Auto,
            max_chars: 1_200,
            overlap_chars: 120,
            heading_depth: 2,
        }
    }
}
impl ChunkConfig {
    pub fn validate(&self) -> Result<(), LocalDocumentError> {
        if !(200..=8_000).contains(&self.max_chars) {
            return Err(failed(
                "maxChars must be between 200 and 8000 Unicode characters",
            ));
        }
        if self.overlap_chars > self.max_chars / 4 {
            return Err(failed("overlapChars must not exceed 25% of maxChars"));
        }
        if !(1..=6).contains(&self.heading_depth) {
            return Err(failed("headingDepth must be between 1 and 6"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLocation {
    /// Identity within the immutable parse artifact, including segment and nested block path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_block_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_source_block_id: Option<String>,
    /// A supplier segment is not an original physical page number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_segment: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_page: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<u32>,
    /// Normalized source-page rectangle; retained only when the parser supplied it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_page: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chart_sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_end: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph_end: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_resource_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_annotations: Vec<ImageAnnotation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_ranges: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_kind: Option<String>,
    pub char_start: usize,
    pub char_end: usize,
}
/// Text provenance from the pinned docvortex.middle v2 image schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAnnotation {
    pub kind: String,
    pub source: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    pub block: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_resource_code: Option<String>,
    /// Present only when the parser or description artifact identifies one asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_code: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAnalysisSummary {
    pub image_count: usize,
    pub originals_only: usize,
    pub with_source_annotations: usize,
    pub with_generated_descriptions: usize,
    pub with_unattributed_body_text: usize,
    pub generation_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplement_status: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocatedSection {
    pub text: String,
    pub heading_level: Option<u8>,
    pub location: DocumentLocation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedDocument {
    pub format: String,
    pub sections: Vec<LocatedSection>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_analysis: Option<ImageAnalysisSummary>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocatedChunk {
    pub text: String,
    pub title: String,
    pub heading_path: Vec<String>,
    pub location: DocumentLocation,
    pub ordinal: usize,
    #[serde(default)]
    pub index_text: String,
    #[serde(default)]
    pub source_spans: Vec<eliteforge_agent_core::ChunkSourceSpan>,
    #[serde(default)]
    pub source_locations: Vec<serde_json::Value>,
    #[serde(default)]
    pub context_id: String,
}

/// Extensions describe the supported input formats, rather than invoking external converters.
pub fn parse_local_document(
    filename: &str,
    bytes: &[u8],
) -> Result<ParsedDocument, LocalDocumentError> {
    parse_document(filename, bytes, MAX_TEXT_BYTES)
}

/// Parse normalized supplier Markdown after binary image payloads have been removed.
pub fn parse_normalized_markdown(bytes: &[u8]) -> Result<ParsedDocument, LocalDocumentError> {
    parse_document("normalized.md", bytes, MAX_PARSED_TEXT_BYTES)
}

fn parse_document(
    filename: &str,
    bytes: &[u8],
    text_limit: usize,
) -> Result<ParsedDocument, LocalDocumentError> {
    if bytes.len() > MAX_RAW_BYTES {
        return Err(failed("document exceeds 20 MiB upload limit"));
    }
    let extension = Path::new(filename)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut document = match extension.as_str() {
        "md" | "markdown" => parse_markdown(bytes, text_limit)?,
        "txt" | "rst" | "tex" => parse_text(bytes)?,
        "docx" => parse_docx(bytes)?,
        "xlsx" => parse_xlsx(bytes)?,
        "pdf" => parse_pdf(bytes)?,
        "doc" | "xls" => {
            return Err(failed(
                "legacy .doc/.xls are unsupported; save as .docx/.xlsx first",
            ));
        }
        _ => {
            return Err(failed(
                "supported local formats: .md, .docx, .xlsx, text .pdf (OCR is unsupported)",
            ));
        }
    };
    let mut total_bytes = 0usize;
    let mut offset = 0usize;
    document
        .sections
        .retain(|section| !section.text.trim().is_empty());
    for section in &mut document.sections {
        total_bytes = total_bytes
            .checked_add(section.text.len() + 1)
            .ok_or_else(|| failed("document text too large"))?;
        if total_bytes > text_limit {
            return Err(failed(format!(
                "normalized document text exceeds {} MiB",
                text_limit / 1024 / 1024
            )));
        }
        section.location.char_start = offset;
        offset += section.text.chars().count();
        section.location.char_end = offset;
        offset += 1; // A newline separates adjacent sections in the canonical text.
    }
    if document.sections.is_empty() {
        return Err(failed(
            "document contains no extractable text; scanned PDFs require OCR, which is unsupported",
        ));
    }
    Ok(document)
}
fn parsed(format: &str, sections: Vec<LocatedSection>) -> ParsedDocument {
    ParsedDocument {
        format: format.into(),
        sections,
        warnings: Vec::new(),
        image_analysis: None,
    }
}
pub fn decode_plain_text(bytes: &[u8]) -> Result<String, LocalDocumentError> {
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(failed("文字文件超过2 MiB上限"));
    }
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if bytes.len() % 2 != 0 {
            return Err(failed("UTF-16文字文件编码无效"));
        }
        let little = bytes[0] == 0xff;
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|b| {
                if little {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect::<Vec<_>>();
        String::from_utf16(&units).map_err(failed)?
    } else {
        std::str::from_utf8(bytes)
            .map_err(|_| failed("文字文件需为UTF-8或带BOM的UTF-16"))?
            .trim_start_matches('\u{feff}')
            .to_owned()
    };
    if text.contains('\0') {
        return Err(failed("文字文件包含二进制内容"));
    }
    if text.len() > MAX_TEXT_BYTES {
        return Err(failed("解码文字超过2 MiB上限"));
    }
    Ok(text)
}
fn parse_text(bytes: &[u8]) -> Result<ParsedDocument, LocalDocumentError> {
    let text = decode_plain_text(bytes)?;
    let count = text.lines().count() as u32;
    Ok(parsed(
        "txt",
        vec![section(
            text,
            None,
            DocumentLocation {
                line_start: Some(1),
                line_end: Some(count),
                ..Default::default()
            },
        )],
    ))
}
fn section(text: String, heading_level: Option<u8>, location: DocumentLocation) -> LocatedSection {
    LocatedSection {
        text,
        heading_level,
        location,
    }
}
fn parse_markdown(bytes: &[u8], text_limit: usize) -> Result<ParsedDocument, LocalDocumentError> {
    if bytes.len() > text_limit {
        return Err(failed(format!(
            "normalized document text exceeds {} MiB",
            text_limit / 1024 / 1024
        )));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| failed("Markdown must be UTF-8"))?
        .trim_start_matches('\u{feff}');
    if text.contains('\0') {
        return Err(failed("Markdown contains binary content"));
    }
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut heading = None;
    let mut level = None;
    let mut start_line = 1;
    let mut end_line = 0;
    let mut fence: Option<(char, usize)> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let first = trimmed.chars().next().unwrap_or(' ');
        let fence_count = trimmed.chars().take_while(|c| *c == first).count();
        if (first == '`' || first == '~') && fence_count >= 3 {
            match fence {
                Some((symbol, width))
                    if symbol == first
                        && fence_count >= width
                        && trimmed[fence_count..].trim().is_empty() =>
                {
                    fence = None
                }
                None => fence = Some((first, fence_count)),
                _ => {}
            }
        }
        if line.trim().is_empty() && fence.is_none() {
            if !current.is_empty() {
                sections.push(section(
                    std::mem::take(&mut current),
                    level.take(),
                    DocumentLocation {
                        heading: heading.clone(),
                        line_start: Some(start_line),
                        line_end: Some(end_line),
                        ..Default::default()
                    },
                ));
            }
            start_line = index as u32 + 2;
            continue;
        }
        let hashes = if fence.is_none() && line.len() - trimmed.len() <= 3 {
            trimmed.chars().take_while(|c| *c == '#').count()
        } else {
            0
        };
        let is_heading = (1..=6).contains(&hashes)
            && trimmed
                .as_bytes()
                .get(hashes)
                .is_none_or(u8::is_ascii_whitespace);
        if is_heading {
            if !current.is_empty() {
                sections.push(section(
                    std::mem::take(&mut current),
                    level,
                    DocumentLocation {
                        heading: heading.clone(),
                        line_start: Some(start_line),
                        line_end: Some(end_line),
                        ..Default::default()
                    },
                ));
            }
            heading = Some(
                trimmed[hashes..]
                    .trim()
                    .trim_end_matches('#')
                    .trim_end()
                    .to_string(),
            );
            level = Some(hashes as u8);
            start_line = index as u32 + 1;
        }
        if !current.is_empty() {
            current.push('\n');
        } else {
            start_line = index as u32 + 1;
        }
        current.push_str(line);
        end_line = index as u32 + 1;
        if is_heading {
            sections.push(section(
                std::mem::take(&mut current),
                level.take(),
                DocumentLocation {
                    heading: heading.clone(),
                    line_start: Some(start_line),
                    line_end: Some(end_line),
                    ..Default::default()
                },
            ));
        }
    }
    if !current.is_empty() {
        sections.push(section(
            current,
            level,
            DocumentLocation {
                heading,
                line_start: Some(start_line),
                line_end: Some(end_line),
                ..Default::default()
            },
        ));
    }
    Ok(parsed("md", sections))
}
/// Read and validate each archive entry before handing XLSX to its parser.
/// Both declared and actual decompressed lengths are checked; no archive paths are written.
pub(super) fn validate_office(
    bytes: &[u8],
    wanted: Option<&str>,
) -> Result<Option<Vec<u8>>, LocalDocumentError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| failed(format!("invalid Office package: {e}")))?;
    if archive.len() > 4_096 {
        return Err(failed("Office package exceeds 4096 entries"));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut total_declared = 0u64;
    let mut total_actual = 0usize;
    let mut selected = None;
    let mut buffer = [0u8; 16 * 1024];
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(failed)?;
        if !names.insert(file.name().to_string()) {
            return Err(failed("duplicate Office package entry"));
        }
        total_declared = total_declared
            .checked_add(file.size())
            .ok_or_else(|| failed("Office package too large"))?;
        if total_declared > MAX_EXPANDED_BYTES as u64 {
            return Err(failed("expanded Office package exceeds 100 MiB"));
        }
        let capture = wanted == Some(file.name());
        let mut captured = Vec::new();
        loop {
            let count = file.read(&mut buffer).map_err(failed)?;
            if count == 0 {
                break;
            }
            total_actual += count;
            if total_actual > MAX_EXPANDED_BYTES {
                return Err(failed("expanded Office package exceeds 100 MiB"));
            }
            if capture {
                captured.extend_from_slice(&buffer[..count]);
            }
        }
        if capture {
            selected = Some(captured);
        }
    }
    Ok(selected)
}
fn parse_docx(bytes: &[u8]) -> Result<ParsedDocument, LocalDocumentError> {
    let xml_bytes = validate_office(bytes, Some("word/document.xml"))?
        .ok_or_else(|| failed("DOCX is missing word/document.xml"))?;
    let xml = std::str::from_utf8(&xml_bytes).map_err(failed)?;
    let doc = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            nodes_limit: 300_000,
            ..Default::default()
        },
    )
    .map_err(failed)?;
    let mut sections = Vec::new();
    let mut total = 0;
    let mut heading = None;
    for (index, paragraph) in doc
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "p")))
        .enumerate()
    {
        let mut text = String::new();
        for node in paragraph.descendants().filter(|node| {
            node.ancestors()
                .find(|parent| parent.has_tag_name((WORD_NS, "p")))
                == Some(paragraph)
        }) {
            if node.has_tag_name((WORD_NS, "t")) {
                text.push_str(node.text().unwrap_or_default());
            } else if node.has_tag_name((WORD_NS, "br")) || node.has_tag_name((WORD_NS, "cr")) {
                text.push('\n');
            } else if node.has_tag_name((WORD_NS, "tab")) {
                text.push('\t');
            }
        }
        total += text.len() + 1;
        if total > MAX_TEXT_BYTES {
            return Err(failed("normalized document text exceeds 2 MiB"));
        }
        let level = paragraph
            .descendants()
            .find(|node| node.has_tag_name((WORD_NS, "pStyle")))
            .and_then(|node| node.attribute((WORD_NS, "val")))
            .and_then(|style| {
                style
                    .to_ascii_lowercase()
                    .strip_prefix("heading")
                    .and_then(|level| level.parse::<u8>().ok())
            })
            .filter(|level| (1..=6).contains(level));
        if level.is_some() {
            heading = Some(text.clone());
        }
        sections.push(section(
            text,
            level,
            DocumentLocation {
                heading: heading.clone(),
                paragraph: Some(index as u32 + 1),
                paragraph_end: Some(index as u32 + 1),
                ..Default::default()
            },
        ));
    }
    Ok(parsed("docx", sections))
}
pub(super) fn parse_xlsx(bytes: &[u8]) -> Result<ParsedDocument, LocalDocumentError> {
    validate_office(bytes, None)?;
    let book =
        umya_spreadsheet::reader::xlsx::read_reader(Cursor::new(bytes), true).map_err(failed)?;
    let mut sections = Vec::new();
    let mut total = 0usize;
    for sheet in book.sheet_collection() {
        // Iterate present cells, not the rectangular dimensions (which may be sparse or hostile).
        let mut rows: BTreeMap<u32, Vec<(u32, String)>> = BTreeMap::new();
        for cell in sheet.cells() {
            let value = cell.value().to_string();
            if value.trim().is_empty() {
                continue;
            }
            total += value.len() + 32;
            if total > MAX_TEXT_BYTES {
                return Err(failed("normalized document text exceeds 2 MiB"));
            }
            rows.entry(cell.coordinate().row_num())
                .or_default()
                .push((cell.coordinate().col_num(), value));
        }
        for (row, mut cells) in rows {
            cells.sort_by_key(|(column, _)| *column);
            let cell_ranges = cells
                .iter()
                .map(|(column, _)| format!("{}{}", excel_column(*column), row))
                .collect();
            let text = cells
                .into_iter()
                .map(|(column, value)| {
                    format!("{}!{}{}: {value}", sheet.name(), excel_column(column), row)
                })
                .collect::<Vec<_>>()
                .join(" | ");
            sections.push(section(
                text,
                None,
                DocumentLocation {
                    sheet: Some(sheet.name().to_string()),
                    row_start: Some(row),
                    row_end: Some(row),
                    cell_ranges,
                    ..Default::default()
                },
            ));
        }
    }
    let mut result = parsed("xlsx", sections);
    result
        .warnings
        .push("Formula values are cached workbook values; formulas are not recalculated.".into());
    Ok(result)
}
struct BoundedText {
    bytes: Vec<u8>,
    remaining: usize,
}
impl Write for BoundedText {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > self.remaining {
            return Err(std::io::Error::other(
                "normalized document text exceeds 2 MiB",
            ));
        }
        self.bytes.extend_from_slice(buf);
        self.remaining -= buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn parse_pdf(bytes: &[u8]) -> Result<ParsedDocument, LocalDocumentError> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(failed("file is not a PDF"));
    }
    let doc = pdf_extract::Document::load_mem_with_options(
        bytes,
        pdf_extract::LoadOptions {
            strict: true,
            ..Default::default()
        },
    )
    .map_err(|e| failed(format!("invalid PDF: {e}")))?;
    if doc.is_encrypted() {
        return Err(failed("encrypted PDF is unsupported"));
    }
    let pages = doc.get_pages();
    if pages.len() > 1_000 {
        return Err(failed("PDF exceeds 1000 pages"));
    }
    let mut remaining = MAX_TEXT_BYTES;
    let mut sections = Vec::new();
    let mut empty_pages = Vec::new();
    for page in pages.keys().copied() {
        let mut output = BoundedText {
            bytes: Vec::new(),
            remaining,
        };
        pdf_extract::output_doc_page(
            &doc,
            &mut pdf_extract::PlainTextOutput::new(&mut output as &mut dyn Write),
            page,
        )
        .map_err(|e| failed(format!("PDF page {page} extraction failed: {e}")))?;
        remaining = output.remaining;
        let text = String::from_utf8(output.bytes).map_err(failed)?;
        if text.trim().is_empty() {
            empty_pages.push(page);
        } else {
            sections.push(section(
                text,
                None,
                DocumentLocation {
                    page: Some(page),
                    ..Default::default()
                },
            ));
        }
    }
    if sections.is_empty() {
        return Err(failed(
            "PDF contains no extractable text; scanned PDFs require OCR, which is unsupported",
        ));
    }
    let mut result = parsed("pdf", sections);
    if !empty_pages.is_empty() {
        result.warnings.push(format!(
            "Pages {empty_pages:?} contain no extractable text; OCR is unsupported."
        ));
    }
    Ok(result)
}

mod chunking;
mod source_facts;
pub use chunking::{
    ChunkedDocument, chunk_document_input, span_location, split_document,
    split_document_with_strategies,
};
pub use source_facts::source_fact_locations;



fn excel_column(mut index: u32) -> String {
    let mut letters = Vec::new();
    while index > 0 {
        index -= 1;
        letters.push((b'A' + (index % 26) as u8) as char);
        index /= 26;
    }
    letters.into_iter().rev().collect()
}
