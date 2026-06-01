use std::hash::{Hash, Hasher};

use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    CompiledRagIngestProfile, RagIngestRequest, RagMemorySourceFields, RagProjectDocSourceFields,
    RagTextUnitWrite,
};

#[derive(Debug, Clone)]
pub(crate) struct PreparedIngestPayload {
    pub(crate) title: Option<String>,
    pub(crate) content_hash: String,
    pub(crate) metadata: Value,
    pub(crate) memory: Option<RagMemorySourceFields>,
    pub(crate) project_doc: Option<RagProjectDocSourceFields>,
    pub(crate) units: Vec<RagTextUnitDraft>,
}

#[derive(Debug, Clone)]
pub(crate) struct RagTextUnitDraft {
    pub(crate) text_unit_code: String,
    pub(crate) document_code: Option<String>,
    pub(crate) text: String,
    pub(crate) heading_path: String,
    pub(crate) ordinal: usize,
    pub(crate) content_hash: String,
}

impl RagTextUnitDraft {
    pub(crate) fn with_embedding(
        self,
        embedding: Vec<f32>,
        embedding_model_code: &str,
        embedding_version: &str,
    ) -> RagTextUnitWrite {
        let embedding_text_hash = stable_hash(&self.text);
        RagTextUnitWrite {
            text_unit_code: self.text_unit_code,
            document_code: self.document_code,
            unit_type: "LEAF".into(),
            token_count: token_count(&self.text),
            search_text: self.text.clone(),
            text: self.text,
            heading_path: self.heading_path,
            ordinal: self.ordinal,
            content_hash: self.content_hash,
            status: "ACTIVE".into(),
            fulltext_status: "INDEXED".into(),
            embedding_model_code: Some(embedding_model_code.to_string()),
            embedding_version: Some(embedding_version.to_string()),
            embedding_text_hash: Some(embedding_text_hash),
            embedding_dimension: Some(embedding.len() as i64),
            embedding: Some(encode_embedding(&embedding)),
            embedding_status: "INDEXED".into(),
            metadata: json!({}),
        }
    }

    pub(crate) fn with_failed_embedding(self) -> RagTextUnitWrite {
        RagTextUnitWrite {
            text_unit_code: self.text_unit_code,
            document_code: self.document_code,
            unit_type: "LEAF".into(),
            token_count: token_count(&self.text),
            search_text: self.text.clone(),
            text: self.text,
            heading_path: self.heading_path,
            ordinal: self.ordinal,
            content_hash: self.content_hash,
            status: "ACTIVE".into(),
            fulltext_status: "INDEXED".into(),
            embedding_model_code: None,
            embedding_version: None,
            embedding_text_hash: None,
            embedding_dimension: None,
            embedding: None,
            embedding_status: "FAILED".into(),
            metadata: json!({}),
        }
    }
}

pub(crate) fn prepare_payload(
    request: &RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
) -> Result<PreparedIngestPayload, RagError> {
    match request.corpus_id.as_str() {
        "memory-card" => prepare_memory_payload(request, compiled),
        "project-doc" => prepare_project_doc_payload(request, compiled),
        _ => Err(RagError::CorpusReserved),
    }
}

fn prepare_memory_payload(
    request: &RagIngestRequest,
    _compiled: &CompiledRagIngestProfile,
) -> Result<PreparedIngestPayload, RagError> {
    if request.source_type != "PLAIN_TEXT" {
        return Err(RagError::IngestRequestInvalid);
    }
    let text = required_payload_text(&request.payload)?;
    let title = payload_string(&request.payload, "title");
    let content_hash = stable_hash(&text);
    let unit = RagTextUnitDraft {
        text_unit_code: text_unit_code(&request.source_code, &request.source_version, 0),
        document_code: None,
        text,
        heading_path: String::new(),
        ordinal: 0,
        content_hash: content_hash.clone(),
    };
    Ok(PreparedIngestPayload {
        title,
        content_hash,
        metadata: request.payload.clone(),
        memory: Some(RagMemorySourceFields {
            memory_type: payload_string(&request.payload, "memoryType"),
            memory_status: payload_string(&request.payload, "memoryStatus"),
            review_status: payload_string(&request.payload, "reviewStatus"),
            version: Some(request.source_version.clone()),
            superseded_by: payload_string(&request.payload, "supersededBy"),
            conflict_group: payload_string(&request.payload, "conflictGroup"),
            duplicate_group: payload_string(&request.payload, "duplicateGroup"),
            injectable: request.payload.get("injectable").and_then(Value::as_bool),
            importance_score: request
                .payload
                .get("importanceScore")
                .and_then(Value::as_i64),
            confidence_score: request
                .payload
                .get("confidenceScore")
                .and_then(Value::as_i64),
            quality_score: request.payload.get("qualityScore").and_then(Value::as_i64),
            pin_level: request.payload.get("pinLevel").and_then(Value::as_i64),
            expires_at: request.payload.get("expiresAt").and_then(Value::as_i64),
            derived_source_type: payload_string(&request.payload, "derivedSourceType"),
            derived_source_code: payload_string(&request.payload, "derivedSourceCode"),
        }),
        project_doc: None,
        units: vec![unit],
    })
}

fn prepare_project_doc_payload(
    request: &RagIngestRequest,
    _compiled: &CompiledRagIngestProfile,
) -> Result<PreparedIngestPayload, RagError> {
    if request.source_type != "MARKDOWN" {
        return Err(RagError::IngestRequestInvalid);
    }
    let text = required_payload_text(&request.payload)?;
    let document_code = payload_string(&request.payload, "documentCode");
    let sections = split_markdown_heading_units(&text);
    if sections.is_empty() {
        return Err(RagError::IngestRequestInvalid);
    }
    let units = sections
        .into_iter()
        .enumerate()
        .map(|(ordinal, (heading_path, section_text))| RagTextUnitDraft {
            text_unit_code: text_unit_code(&request.source_code, &request.source_version, ordinal),
            document_code: document_code.clone(),
            content_hash: stable_hash(&section_text),
            text: section_text,
            heading_path,
            ordinal,
        })
        .collect();
    Ok(PreparedIngestPayload {
        title: payload_string(&request.payload, "title"),
        content_hash: stable_hash(&text),
        metadata: request.payload.clone(),
        memory: None,
        project_doc: Some(RagProjectDocSourceFields {
            document_code,
            document_type: payload_string(&request.payload, "documentType"),
            path: payload_string(&request.payload, "path"),
            body_hash: stable_hash(&text),
            repo_code: payload_string(&request.payload, "repoCode"),
            module_code: payload_string(&request.payload, "moduleCode"),
            module_name: payload_string(&request.payload, "moduleName"),
        }),
        units,
    })
}

fn required_payload_text(payload: &Value) -> Result<String, RagError> {
    payload
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(RagError::IngestRequestInvalid)
}

fn payload_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn split_markdown_heading_units(text: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut heading_stack: Vec<(usize, String)> = Vec::new();
    let mut current_heading = String::new();
    let mut current_lines = Vec::new();

    for line in text.lines() {
        if let Some((level, heading)) = parse_markdown_heading(line) {
            if !current_heading.is_empty() && !current_lines.is_empty() {
                sections.push((
                    current_heading.clone(),
                    current_lines.join("\n").trim().to_string(),
                ));
            }
            heading_stack.retain(|(existing_level, _)| *existing_level < level);
            heading_stack.push((level, heading));
            current_heading = heading_stack
                .iter()
                .map(|(_, heading)| heading.as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            current_lines = vec![line.trim().to_string()];
        } else if !current_heading.is_empty() {
            current_lines.push(line.to_string());
        }
    }
    if !current_heading.is_empty() && !current_lines.is_empty() {
        sections.push((current_heading, current_lines.join("\n").trim().to_string()));
    }
    if sections.is_empty() && !text.trim().is_empty() {
        sections.push((String::new(), text.trim().to_string()));
    }
    sections
}

fn parse_markdown_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();
    let level = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if level == 0 || level > 6 || !trimmed.chars().nth(level).is_some_and(|c| c == ' ') {
        return None;
    }
    let heading = trimmed[level..].trim();
    (!heading.is_empty()).then(|| (level, heading.to_string()))
}

fn text_unit_code(source_code: &str, source_version: &str, ordinal: usize) -> String {
    format!("{source_code}:{source_version}:{ordinal}")
}

fn token_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

fn stable_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn encode_embedding(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>()
}
