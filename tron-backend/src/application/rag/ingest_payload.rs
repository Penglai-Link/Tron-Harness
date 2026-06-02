use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    CompiledRagIngestProfile, RagIngestRequest, RagTextUnitWrite, ingest_artifact::ArtifactBundle,
};

mod json_text;

use json_text::prepare_json_text_payload;

#[derive(Debug, Clone)]
pub(crate) struct PreparedIngestPayload {
    pub(crate) title: Option<String>,
    pub(crate) content_hash: String,
    pub(crate) metadata: Value,
    pub(crate) domain_fields: BTreeMap<String, Value>,
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
    bundle: &ArtifactBundle,
) -> Result<PreparedIngestPayload, RagError> {
    match compiled.payload_strategy_id.as_str() {
        "MEMORY_PLAIN_TEXT_PAYLOAD_V1" => prepare_memory_payload(request, compiled),
        "RAG_JSON_TEXT_PAYLOAD_V1" => prepare_json_text_payload(request, compiled),
        "PROJECT_DOC_MARKDOWN_REF_PAYLOAD_V1" => {
            prepare_project_doc_ref_payload(request, compiled, bundle)
        }
        _ => Err(RagError::IngestProfileInvalid),
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
    let mut domain_fields = BTreeMap::new();
    insert_optional_string(
        &mut domain_fields,
        "memoryType",
        payload_string(&request.payload, "memoryType"),
    );
    insert_optional_string(
        &mut domain_fields,
        "memoryStatus",
        payload_string(&request.payload, "memoryStatus"),
    );
    insert_optional_string(
        &mut domain_fields,
        "reviewStatus",
        payload_string(&request.payload, "reviewStatus"),
    );
    insert_string(
        &mut domain_fields,
        "version",
        request.source_version.clone(),
    );
    insert_optional_string(
        &mut domain_fields,
        "supersededBy",
        payload_string(&request.payload, "supersededBy"),
    );
    insert_optional_string(
        &mut domain_fields,
        "conflictGroup",
        payload_string(&request.payload, "conflictGroup"),
    );
    insert_optional_string(
        &mut domain_fields,
        "duplicateGroup",
        payload_string(&request.payload, "duplicateGroup"),
    );
    if let Some(value) = request.payload.get("injectable").and_then(Value::as_bool) {
        domain_fields.insert("injectable".into(), Value::Bool(value));
    }
    insert_optional_integer(
        &mut domain_fields,
        "importanceScore",
        request
            .payload
            .get("importanceScore")
            .and_then(Value::as_i64),
    );
    insert_optional_integer(
        &mut domain_fields,
        "confidenceScore",
        request
            .payload
            .get("confidenceScore")
            .and_then(Value::as_i64),
    );
    insert_optional_integer(
        &mut domain_fields,
        "qualityScore",
        request.payload.get("qualityScore").and_then(Value::as_i64),
    );
    if let Some(value) = request.payload.get("pinLevel").and_then(Value::as_i64) {
        domain_fields.insert("pinLevel".into(), json!(value));
    }
    insert_optional_integer(
        &mut domain_fields,
        "expiresAt",
        request.payload.get("expiresAt").and_then(Value::as_i64),
    );
    insert_optional_string(
        &mut domain_fields,
        "derivedSourceType",
        payload_string(&request.payload, "derivedSourceType"),
    );
    insert_optional_string(
        &mut domain_fields,
        "derivedSourceCode",
        payload_string(&request.payload, "derivedSourceCode"),
    );
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
        metadata: payload_metadata(&request.payload),
        domain_fields,
        units: vec![unit],
    })
}

fn prepare_project_doc_ref_payload(
    request: &RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
    bundle: &ArtifactBundle,
) -> Result<PreparedIngestPayload, RagError> {
    if request.source_type != "PROJECT_DOC_REF" {
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
    let format =
        payload_string(&request.payload, "format").ok_or(RagError::DocumentFormatUnsupported)?;
    if format != "MARKDOWN" {
        return Err(RagError::DocumentFormatUnsupported);
    }
    let document_uri =
        payload_string(&request.payload, "documentUri").ok_or(RagError::IngestRequestInvalid)?;
    let parsed_document = bundle.parsed_document()?;
    if parsed_document.document_uri != document_uri {
        return Err(RagError::SourceResolveFailed);
    }
    let text = parsed_document.content_text.trim().to_string();
    if text.is_empty() {
        return Err(RagError::DocumentParseFailed);
    }
    let document_code = payload_string(&request.payload, "documentCode");
    let mut domain_fields = BTreeMap::new();
    insert_optional_string(&mut domain_fields, "documentCode", document_code.clone());
    insert_optional_string(
        &mut domain_fields,
        "documentType",
        payload_string(&request.payload, "documentType"),
    );
    insert_optional_string(
        &mut domain_fields,
        "path",
        payload_string(&request.payload, "path"),
    );
    insert_string(&mut domain_fields, "bodyHash", stable_hash(&text));
    insert_optional_string(
        &mut domain_fields,
        "repoCode",
        payload_string(&request.payload, "repoCode"),
    );
    insert_optional_string(
        &mut domain_fields,
        "moduleCode",
        payload_string(&request.payload, "moduleCode"),
    );
    insert_optional_string(
        &mut domain_fields,
        "moduleName",
        payload_string(&request.payload, "moduleName"),
    );
    let sections = split_markdown_heading_units(&text, compiled.markdown_heading_depth);
    if sections.is_empty() {
        return Err(RagError::DocumentParseFailed);
    }
    let mut metadata = payload_metadata(&request.payload);
    if let Value::Object(metadata) = &mut metadata {
        if compiled.record_resolver_info {
            metadata.insert(
                "resolver".into(),
                json!({
                    "strategyId": compiled.source_resolver_strategy_id.as_deref().unwrap_or("PROJECT_DOC_MARKDOWN_RESOLVE_SOURCE_V1"),
                    "documentUri": document_uri,
                    "contentLength": parsed_document.content_length
                }),
            );
        }
        if compiled.record_parser_info {
            metadata.insert(
                "parser".into(),
                json!({
                    "strategyId": compiled.document_parser_strategy_id.as_deref().unwrap_or("PROJECT_DOC_MARKDOWN_PARSE_DOCUMENT_V1"),
                    "format": format
                }),
            );
        }
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
        metadata,
        domain_fields,
        units,
    })
}

fn insert_string(fields: &mut BTreeMap<String, Value>, key: &str, value: String) {
    fields.insert(key.into(), Value::String(value));
}

fn insert_optional_string(fields: &mut BTreeMap<String, Value>, key: &str, value: Option<String>) {
    fields.insert(key.into(), value.map(Value::String).unwrap_or(Value::Null));
}

fn insert_optional_integer(fields: &mut BTreeMap<String, Value>, key: &str, value: Option<i64>) {
    fields.insert(
        key.into(),
        value.map(|value| json!(value)).unwrap_or(Value::Null),
    );
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

fn payload_metadata(payload: &Value) -> Value {
    payload
        .get("metadata")
        .and_then(Value::as_object)
        .map(|object| Value::Object(object.clone()))
        .unwrap_or_else(|| json!({}))
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

fn token_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

fn stable_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::rag::{RagIngestAccessContext, RagIngestScope};

    #[test]
    fn prepares_skill_payload_from_name_and_description() {
        let request = skill_request(json!({
            "skillName": "reviewer",
            "skillDescription": "Reviews code changes before release.",
            "metadata": {
                "source": "registry"
            }
        }));
        let bundle = ArtifactBundle::from_request(&request);

        let prepared = prepare_payload(&request, &skill_profile(), &bundle).unwrap();

        assert_eq!(prepared.title.as_deref(), Some("reviewer"));
        assert_eq!(
            prepared.domain_fields.get("skillName"),
            Some(&json!("reviewer"))
        );
        assert_eq!(
            prepared.domain_fields.get("skillDescription"),
            Some(&json!("Reviews code changes before release."))
        );
        assert_eq!(prepared.units.len(), 1);
        assert_eq!(prepared.units[0].heading_path, "reviewer");
        assert_eq!(
            prepared.units[0].text,
            "Skill: reviewer\nDescription: Reviews code changes before release."
        );
    }

    #[test]
    fn rejects_skill_payload_without_description() {
        let request = skill_request(json!({
            "skillName": "reviewer"
        }));
        let bundle = ArtifactBundle::from_request(&request);

        let error = prepare_payload(&request, &skill_profile(), &bundle).unwrap_err();

        assert_eq!(error, RagError::IngestRequestInvalid);
    }

    fn skill_request(payload: Value) -> RagIngestRequest {
        RagIngestRequest {
            corpus_id: "skill".into(),
            ingest_profile_id: "skill-ingest-v1".into(),
            workspace_code: "workspace-alpha".into(),
            access_context: RagIngestAccessContext {
                user_code: "user-alpha".into(),
                agent_code: "agent-alpha".into(),
                workspace_code: "workspace-alpha".into(),
            },
            target_scope: RagIngestScope {
                scope_type: "SPACE".into(),
                scope_code: "workspace-alpha".into(),
            },
            source_code: "skill:workspace-alpha:reviewer".into(),
            source_version: "v1".into(),
            source_type: "SKILL".into(),
            payload,
        }
    }

    fn skill_profile() -> CompiledRagIngestProfile {
        CompiledRagIngestProfile {
            profile_id: "skill-ingest-v1".into(),
            corpus_id: "skill".into(),
            stage_order: vec![],
            payload_strategy_id: "RAG_JSON_TEXT_PAYLOAD_V1".into(),
            payload_mapping: json!({
                "supportedSourceTypes": ["SKILL"],
                "required": ["skillName", "skillDescription"],
                "title": "$.skillName",
                "textTemplate": "Skill: {{skillName}}\nDescription: {{skillDescription}}",
                "metadataFrom": "$.metadata",
                "fields": {
                    "skillName": {
                        "path": "$.skillName",
                        "target": "skill_name",
                        "type": "string"
                    },
                    "skillDescription": {
                        "path": "$.skillDescription",
                        "target": "skill_description",
                        "type": "string"
                    }
                }
            }),
            source_resolver_strategy_id: None,
            document_parser_strategy_id: None,
            source_resolver_max_bytes: 1024,
            record_resolver_info: false,
            record_parser_info: false,
            markdown_heading_depth: 6,
            degrade_policy: "SKIP_VECTOR_WITH_WARNING".into(),
            embedding_model_code: None,
            embedding_version: "v1".into(),
        }
    }
}

fn encode_embedding(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>()
}
