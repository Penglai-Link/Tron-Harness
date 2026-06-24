use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
};

use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    CompiledRagIngestProfile, RagIngestRequest, RagTextUnitWrite, ingest_artifact::ArtifactBundle,
};

#[derive(Debug, Clone)]
pub(crate) struct PreparedIngestPayload {
    pub(crate) title: Option<String>,
    pub(crate) content_hash: String,
    pub(crate) body_hash: Option<String>,
    pub(crate) metadata: Value,
    pub(crate) domain_fields: BTreeMap<String, Value>,
    pub(crate) units: Vec<RagTextUnitDraft>,
    pub(crate) tree_hints: Vec<RagTreeHintDraft>,
}

#[derive(Debug, Clone)]
pub(crate) struct RagTextUnitDraft {
    pub(crate) text_unit_code: String,
    pub(crate) text: String,
    pub(crate) index_text: String,
    pub(crate) heading_path: String,
    pub(crate) ordinal: usize,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RagTreeHintDraft {
    pub(crate) tree_node_code: String,
    pub(crate) parent_node_code: String,
    pub(crate) path_key: String,
    pub(crate) level: i64,
    pub(crate) ordinal: i64,
    pub(crate) title: String,
    pub(crate) covered_text_unit_codes: Vec<String>,
}

impl RagTextUnitDraft {
    pub(crate) fn with_embedding(
        self,
        embedding: Vec<f32>,
        embedding_model_code: &str,
        embedding_version: &str,
    ) -> RagTextUnitWrite {
        let embedding_text_hash = stable_hash(&self.index_text);
        RagTextUnitWrite {
            text_unit_code: self.text_unit_code,
            unit_type: "LEAF".into(),
            token_count: token_count(&self.text),
            search_text: self.index_text,
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
            unit_type: "LEAF".into(),
            token_count: token_count(&self.text),
            search_text: self.index_text,
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
    bundle: &ArtifactBundle,
) -> Result<PreparedIngestPayload, RagError> {
    match compiled.payload_strategy_id.as_str() {
        "RAG_TEXT_CONTRACT_PAYLOAD_V1" => prepare_text_contract_payload(request, compiled),
        "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1" => {
            prepare_document_ref_contract_payload(request, compiled, bundle)
        }
        _ => Err(RagError::IngestProfileInvalid),
    }
}

fn prepare_text_contract_payload(
    request: &RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
) -> Result<PreparedIngestPayload, RagError> {
    validate_contract_source_type(request, compiled)?;
    let text = payload_string(&request.payload, "text").ok_or(RagError::IngestRequestInvalid)?;
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(RagError::IngestRequestInvalid);
    }
    let title = payload_string(&request.payload, "title");
    let domain_fields = contract_domain_fields(&request.payload)?;
    let content_hash = stable_hash(&text);
    let split = split_text_payload(
        &request.source.code,
        &request.source.version,
        title.as_deref(),
        &text,
        compiled,
    )?;
    Ok(PreparedIngestPayload {
        title,
        content_hash,
        body_hash: None,
        metadata: contract_metadata(&request.payload)?,
        domain_fields,
        units: split.units,
        tree_hints: split.tree_hints,
    })
}

fn prepare_document_ref_contract_payload(
    request: &RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
    bundle: &ArtifactBundle,
) -> Result<PreparedIngestPayload, RagError> {
    validate_contract_source_type(request, compiled)?;
    if request.source.source_type != "PROJECT_DOC_REF" {
        return Err(RagError::IngestRequestInvalid);
    }
    if request
        .payload
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return Err(RagError::IngestRequestInvalid);
    }
    let format = document_payload_string(&request.payload, "format")
        .ok_or(RagError::DocumentFormatUnsupported)?;
    if !compiled.supports_document_format(&format) {
        return Err(RagError::DocumentFormatUnsupported);
    }
    let document_uri =
        document_payload_string(&request.payload, "uri").ok_or(RagError::IngestRequestInvalid)?;
    let parsed_document = bundle.parsed_document()?;
    if parsed_document.document_uri != document_uri {
        return Err(RagError::SourceResolveFailed);
    }
    let text = parsed_document.content_text.trim().to_string();
    if text.is_empty() {
        return Err(RagError::DocumentParseFailed);
    }
    let domain_fields = contract_domain_fields(&request.payload)?;
    let mut metadata = contract_metadata(&request.payload)?;
    if let Value::Object(metadata) = &mut metadata {
        if compiled.record_resolver_info {
            metadata.insert(
                "_resolver".into(),
                json!({
                    "strategyId": compiled.source_resolver_strategy_id.as_deref().unwrap_or("PROJECT_DOC_RESOLVE_SOURCE"),
                    "documentUri": document_uri,
                    "contentLength": parsed_document.content_length
                }),
            );
        }
        if compiled.record_parser_info {
            metadata.insert(
                "_parser".into(),
                json!({
                    "strategyId": compiled.document_parser_strategy_id.as_deref().unwrap_or("RAG_PARSE_MARKDOWN_DOCUMENT"),
                    "format": format
                }),
            );
        }
    }
    let title = payload_string(&request.payload, "title");
    let split = split_text_payload(
        &request.source.code,
        &request.source.version,
        title.as_deref(),
        &text,
        compiled,
    )?;
    Ok(PreparedIngestPayload {
        title,
        content_hash: stable_hash(&text),
        body_hash: Some(stable_hash(&text)),
        metadata,
        domain_fields,
        units: split.units,
        tree_hints: split.tree_hints,
    })
}

fn payload_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn document_payload_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get("document")
        .and_then(|document| document.get(field))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn contract_domain_fields(payload: &Value) -> Result<BTreeMap<String, Value>, RagError> {
    let Some(value) = payload.get("domainFields") else {
        return Ok(BTreeMap::new());
    };
    let object = value.as_object().ok_or(RagError::IngestRequestInvalid)?;
    Ok(object
        .iter()
        .map(|(field, value)| {
            let value = match value {
                Value::String(value) => Value::String(value.trim().to_string()),
                value => value.clone(),
            };
            (field.clone(), value)
        })
        .collect())
}

fn contract_metadata(payload: &Value) -> Result<Value, RagError> {
    let Some(value) = payload.get("metadata") else {
        return Ok(json!({}));
    };
    let object = value.as_object().ok_or(RagError::IngestRequestInvalid)?;
    if object.keys().any(|key| key.starts_with('_')) {
        return Err(RagError::IngestRequestInvalid);
    }
    Ok(Value::Object(object.clone()))
}

fn validate_contract_source_type(
    request: &RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
) -> Result<(), RagError> {
    if compiled.supported_source_types.is_empty()
        || compiled
            .supported_source_types
            .iter()
            .any(|source_type| source_type == &request.source.source_type)
    {
        Ok(())
    } else {
        Err(RagError::IngestRequestInvalid)
    }
}

#[derive(Clone, Copy)]
enum SplitTextStrategy {
    Plain,
    MarkdownHeading,
}

struct SplitTextPayload {
    units: Vec<RagTextUnitDraft>,
    tree_hints: Vec<RagTreeHintDraft>,
}

fn split_text_payload(
    source_code: &str,
    source_version: &str,
    title: Option<&str>,
    text: &str,
    compiled: &CompiledRagIngestProfile,
) -> Result<SplitTextPayload, RagError> {
    let units = match split_text_strategy(compiled)? {
        SplitTextStrategy::Plain => vec![plain_text_unit(source_code, source_version, title, text)],
        SplitTextStrategy::MarkdownHeading => markdown_heading_text_units(
            source_code,
            source_version,
            text,
            compiled.markdown_heading_depth,
            compiled.include_heading_in_index_text,
        )?,
    };
    let tree_hints = tree_hints_from_units(source_code, source_version, title, &units);
    Ok(SplitTextPayload { units, tree_hints })
}

fn split_text_strategy(compiled: &CompiledRagIngestProfile) -> Result<SplitTextStrategy, RagError> {
    match compiled.split_strategy_id.as_str() {
        "RAG_SPLIT_PLAIN_TEXT_V1" => Ok(SplitTextStrategy::Plain),
        "RAG_SPLIT_MARKDOWN_HEADING_V1" => Ok(SplitTextStrategy::MarkdownHeading),
        "RAG_SPLIT_TEXT"
            if compiled.payload_strategy_id == "RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1" =>
        {
            Ok(SplitTextStrategy::MarkdownHeading)
        }
        "RAG_SPLIT_TEXT" => Ok(SplitTextStrategy::Plain),
        _ => Err(RagError::IngestProfileInvalid),
    }
}

fn plain_text_unit(
    source_code: &str,
    source_version: &str,
    title: Option<&str>,
    text: &str,
) -> RagTextUnitDraft {
    let text = text.trim().to_string();
    RagTextUnitDraft {
        text_unit_code: text_unit_code(source_code, source_version, 0),
        index_text: text.clone(),
        content_hash: stable_hash(&text),
        text,
        heading_path: title.unwrap_or_default().to_string(),
        ordinal: 0,
    }
}

fn markdown_heading_text_units(
    source_code: &str,
    source_version: &str,
    text: &str,
    heading_depth: usize,
    include_heading_in_index_text: bool,
) -> Result<Vec<RagTextUnitDraft>, RagError> {
    let sections = split_markdown_heading_units(text, heading_depth);
    if sections.is_empty() {
        return Err(RagError::DocumentParseFailed);
    }
    Ok(sections
        .into_iter()
        .enumerate()
        .map(|(ordinal, (heading_path, section_text))| RagTextUnitDraft {
            text_unit_code: text_unit_code(source_code, source_version, ordinal),
            index_text: markdown_index_text(
                &heading_path,
                &section_text,
                include_heading_in_index_text,
            ),
            content_hash: stable_hash(&section_text),
            text: section_text,
            heading_path,
            ordinal,
        })
        .collect())
}

fn markdown_index_text(
    heading_path: &str,
    section_text: &str,
    include_heading_in_index_text: bool,
) -> String {
    if include_heading_in_index_text && !heading_path.trim().is_empty() {
        format!("{}\n{}", heading_path.trim(), section_text.trim())
    } else {
        section_text.trim().to_string()
    }
}

fn split_markdown_heading_units(text: &str, heading_depth: usize) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut heading_stack: Vec<(usize, String)> = Vec::new();
    let mut current_heading = String::new();
    let mut current_lines = Vec::new();
    let mut current_has_body = false;

    for line in text.lines() {
        if let Some((level, heading)) = parse_markdown_heading(line) {
            if level > heading_depth {
                if !current_heading.is_empty() {
                    current_lines.push(line.to_string());
                    current_has_body = true;
                }
                continue;
            }
            if !current_heading.is_empty() && !current_lines.is_empty() && current_has_body {
                sections.push((
                    current_heading.clone(),
                    current_lines.join("\n").trim().to_string(),
                ));
                current_lines = Vec::new();
            }
            heading_stack.retain(|(existing_level, _)| *existing_level < level);
            heading_stack.push((level, heading));
            current_heading = heading_stack
                .iter()
                .map(|(_, heading)| heading.as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            current_lines.push(line.trim().to_string());
            current_has_body = false;
        } else if !current_heading.is_empty() {
            if !line.trim().is_empty() {
                current_has_body = true;
            }
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

fn tree_hints_from_units(
    source_code: &str,
    source_version: &str,
    title: Option<&str>,
    units: &[RagTextUnitDraft],
) -> Vec<RagTreeHintDraft> {
    let all_codes = units
        .iter()
        .map(|unit| unit.text_unit_code.clone())
        .collect::<Vec<_>>();
    let root_title = title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(source_code)
        .to_string();
    let mut hints = vec![RagTreeHintDraft {
        tree_node_code: tree_node_code(source_code, source_version, "root"),
        parent_node_code: "0".into(),
        path_key: "/".into(),
        level: 0,
        ordinal: 0,
        title: root_title,
        covered_text_unit_codes: all_codes,
    }];
    let mut seen_paths = BTreeSet::new();
    let mut ordinal = 1_i64;
    for unit in units {
        let parts = split_heading_parts(&unit.heading_path);
        for index in 0..parts.len() {
            let path_key = parts[..=index].join("/");
            if !seen_paths.insert(path_key.clone()) {
                continue;
            }
            let covered_text_unit_codes = units
                .iter()
                .filter(|candidate| heading_covers(&candidate.heading_path, &parts[..=index]))
                .map(|candidate| candidate.text_unit_code.clone())
                .collect::<Vec<_>>();
            let parent_node_code = if index == 0 {
                tree_node_code(source_code, source_version, "root")
            } else {
                tree_node_code(source_code, source_version, &parts[..index].join("/"))
            };
            hints.push(RagTreeHintDraft {
                tree_node_code: tree_node_code(source_code, source_version, &path_key),
                parent_node_code,
                path_key,
                level: (index + 1) as i64,
                ordinal,
                title: parts[index].clone(),
                covered_text_unit_codes,
            });
            ordinal += 1;
        }
    }
    hints
}

fn split_heading_parts(heading_path: &str) -> Vec<String> {
    heading_path
        .split('>')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn heading_covers(heading_path: &str, prefix: &[String]) -> bool {
    let parts = split_heading_parts(heading_path);
    parts.len() >= prefix.len()
        && parts
            .iter()
            .zip(prefix.iter())
            .all(|(part, expected)| part == expected)
}

fn tree_node_code(source_code: &str, source_version: &str, path_key: &str) -> String {
    format!(
        "RAG_TREE_{}",
        stable_hash(&format!("{source_code}:{source_version}:{path_key}")).to_ascii_uppercase()
    )
}

fn token_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

fn stable_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
include!("ingest_payload_tests.rs");

fn encode_embedding(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>()
}
