use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::application::rag::{CandidateHit, RagIngestAccessContext};

use super::{
    dto::{MemoryRagWriteResult, MemoryRunContext, MemorySaveCommand, MemorySaveResponse},
    error::MemoryError,
    payload::agent_metadata_from_source,
};

pub(super) const MEMORY_SOURCE_VERSION: &str = "v1";
pub(super) const MAIN_WORK_DOMAIN_CODE: &str = "main-domain";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SearchMode {
    Search,
    ContextPack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MemoryScope {
    User,
    WorkDomain,
    Space,
}

impl MemoryScope {
    pub(super) fn parse(value: &str) -> Result<Self, MemoryError> {
        match value.trim().to_ascii_uppercase().as_str() {
            "U" | "USER" => Ok(Self::User),
            "D" | "WORK_DOMAIN" => Ok(Self::WorkDomain),
            "S" | "SPACE" => Ok(Self::Space),
            _ => Err(MemoryError::Validation(
                "scope must be USER, WORK_DOMAIN, or SPACE".into(),
            )),
        }
    }

    pub(super) fn as_public(self) -> &'static str {
        match self {
            Self::User => "USER",
            Self::WorkDomain => "WORK_DOMAIN",
            Self::Space => "SPACE",
        }
    }

    pub(super) fn as_rag(self) -> &'static str {
        match self {
            Self::User => "USER",
            Self::WorkDomain => "AGENT",
            Self::Space => "SPACE",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::User => "U",
            Self::WorkDomain => "D",
            Self::Space => "S",
        }
    }

    fn source_prefix(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::WorkDomain => "work-domain",
            Self::Space => "space",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MemoryLocator {
    pub(super) scope: MemoryScope,
    pub(super) scope_code: String,
    pub(super) source_code: String,
    pub(super) source_version: String,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PayloadOverrides {
    pub(super) title: Option<String>,
    pub(super) text: Option<String>,
    pub(super) memory_type: Option<String>,
    pub(super) memory_status: Option<String>,
    pub(super) review_status: Option<String>,
    pub(super) injectable: Option<bool>,
    pub(super) importance_score: Option<i64>,
    pub(super) confidence_score: Option<i64>,
    pub(super) quality_score: Option<i64>,
    pub(super) expires_at: Option<i64>,
    pub(super) derived_source_type: Option<String>,
    pub(super) derived_source_code: Option<String>,
    pub(super) source_refs: Option<Value>,
    pub(super) semantic_key: Option<String>,
    pub(super) metadata: Option<Value>,
}

impl PayloadOverrides {
    pub(super) fn from_save(command: MemorySaveCommand) -> Result<Self, MemoryError> {
        Ok(Self {
            title: normalized_optional(command.title.as_deref()),
            text: normalized_optional(command.text.as_deref()),
            memory_type: normalized_optional(command.memory_type.as_deref()),
            memory_status: normalized_optional(command.memory_status.as_deref()),
            review_status: normalized_optional(command.review_status.as_deref()),
            injectable: command.injectable,
            importance_score: command
                .importance_score
                .as_ref()
                .map(|value| score_to_i64(value, "importanceScore"))
                .transpose()?,
            confidence_score: command
                .confidence_score
                .as_ref()
                .map(|value| score_to_i64(value, "confidenceScore"))
                .transpose()?,
            quality_score: command
                .quality_score
                .as_ref()
                .map(|value| score_to_i64(value, "qualityScore"))
                .transpose()?,
            expires_at: command.expires_at,
            derived_source_type: normalized_optional(command.derived_source_type.as_deref()),
            derived_source_code: normalized_optional(command.derived_source_code.as_deref()),
            source_refs: command.source_refs,
            semantic_key: normalized_optional(command.semantic_key.as_deref()),
            metadata: command.metadata,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CurrentMemory {
    pub(super) locator: MemoryLocator,
    pub(super) title: Option<String>,
    pub(super) text: String,
    pub(super) token_count: i64,
    pub(super) memory_type: Option<String>,
    pub(super) memory_status: Option<String>,
    pub(super) review_status: Option<String>,
    pub(super) injectable: Option<bool>,
    pub(super) importance_score: Option<i64>,
    pub(super) confidence_score: Option<i64>,
    pub(super) quality_score: Option<i64>,
    pub(super) expires_at: Option<i64>,
    pub(super) derived_source_type: Option<String>,
    pub(super) derived_source_code: Option<String>,
    pub(super) updated_at: Option<i64>,
    pub(super) metadata: Value,
}

impl CurrentMemory {
    pub(super) fn blank(locator: &MemoryLocator, text: String) -> Self {
        Self {
            locator: locator.clone(),
            title: None,
            text,
            token_count: 0,
            memory_type: None,
            memory_status: None,
            review_status: None,
            injectable: None,
            importance_score: None,
            confidence_score: None,
            quality_score: None,
            expires_at: None,
            derived_source_type: None,
            derived_source_code: None,
            updated_at: None,
            metadata: json!({}),
        }
    }

    pub(super) fn from_hit(
        locator: MemoryLocator,
        hit: &CandidateHit,
    ) -> Result<Self, MemoryError> {
        if hit.source_code != locator.source_code || hit.source_version != locator.source_version {
            return Err(MemoryError::Rag(
                "RAG memory current hit locator mismatch".into(),
            ));
        }
        Ok(Self {
            locator,
            title: hit.title.clone(),
            text: hit.text.clone(),
            token_count: memory_token_count(&hit.text),
            memory_type: field_string(&hit.fields, "memoryType"),
            memory_status: field_string(&hit.fields, "memoryStatus"),
            review_status: field_string(&hit.fields, "reviewStatus"),
            injectable: field_bool(&hit.fields, "injectable"),
            importance_score: field_i64(&hit.fields, "importanceScore"),
            confidence_score: field_i64(&hit.fields, "confidenceScore"),
            quality_score: field_i64(&hit.fields, "qualityScore"),
            expires_at: field_i64(&hit.fields, "expiresAt"),
            derived_source_type: field_string(&hit.fields, "derivedSourceType"),
            derived_source_code: field_string(&hit.fields, "derivedSourceCode"),
            updated_at: field_i64(&hit.fields, "sourceUpdatedAt"),
            metadata: agent_metadata_from_source(&hit.metadata),
        })
    }
}

pub fn encode_memory_code_parts(
    scope: &str,
    scope_code: &str,
    source_code: &str,
    source_version: &str,
) -> Result<String, MemoryError> {
    let locator = MemoryLocator {
        scope: MemoryScope::parse(scope)?,
        scope_code: validate_memory_code_part(scope_code, "scopeCode")?,
        source_code: validate_memory_code_part(source_code, "sourceCode")?,
        source_version: validate_memory_code_part(source_version, "sourceVersion")?,
    };
    Ok(encode_memory_code(&locator))
}

pub fn decode_memory_code_parts(
    memory_code: &str,
) -> Result<(String, String, String, String), MemoryError> {
    let locator = decode_memory_code(memory_code)?;
    Ok((
        locator.scope.as_public().into(),
        locator.scope_code,
        locator.source_code,
        locator.source_version,
    ))
}

pub(super) fn encode_memory_code(locator: &MemoryLocator) -> String {
    format!(
        "{}:{}:{}:{}",
        locator.scope.prefix(),
        locator.scope_code,
        locator.source_code,
        locator.source_version
    )
}

pub(super) fn decode_memory_code(memory_code: &str) -> Result<MemoryLocator, MemoryError> {
    let parts = memory_code.trim().split(':').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(MemoryError::Validation(
            "memoryCode must use U/D/S:scopeCode:sourceCode:sourceVersion".into(),
        ));
    }
    Ok(MemoryLocator {
        scope: MemoryScope::parse(parts[0])?,
        scope_code: validate_memory_code_part(parts[1], "scopeCode")?,
        source_code: validate_memory_code_part(parts[2], "sourceCode")?,
        source_version: validate_memory_code_part(parts[3], "sourceVersion")?,
    })
}

fn validate_memory_code_part(value: &str, field: &str) -> Result<String, MemoryError> {
    let value = value.trim();
    if value.is_empty() || value.contains(':') {
        return Err(MemoryError::Validation(format!(
            "{field} in memoryCode must be non-empty and must not contain ':'"
        )));
    }
    Ok(value.to_string())
}

pub(super) fn validate_locator(
    locator: &MemoryLocator,
    context: &MemoryRunContext,
) -> Result<(), MemoryError> {
    validate_scope_code(locator.scope, &locator.scope_code, context)?;
    if locator.source_version != MEMORY_SOURCE_VERSION {
        return Err(MemoryError::Validation(
            "memoryCode sourceVersion is not supported".into(),
        ));
    }
    Ok(())
}

pub(super) fn access_context(context: &MemoryRunContext) -> RagIngestAccessContext {
    RagIngestAccessContext {
        user_code: context.user_code.clone(),
        agent_code: context.work_domain_code.clone(),
        workspace_code: context.workspace_code.clone(),
    }
}

pub(super) fn normalize_search_mode(value: Option<&str>) -> Result<SearchMode, MemoryError> {
    match value
        .unwrap_or("SEARCH")
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "SEARCH" => Ok(SearchMode::Search),
        "CONTEXT_PACK" => Ok(SearchMode::ContextPack),
        _ => Err(MemoryError::Validation(
            "mode must be SEARCH or CONTEXT_PACK".into(),
        )),
    }
}

pub(super) fn normalize_delete_mode(value: Option<&str>) -> Result<String, MemoryError> {
    match value
        .unwrap_or("ARCHIVE")
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "ARCHIVE" => Ok("ARCHIVE".into()),
        "DISABLE" => Ok("DISABLE".into()),
        _ => Err(MemoryError::Validation(
            "deleteMode must be ARCHIVE or DISABLE".into(),
        )),
    }
}

pub(super) fn validate_limit(limit: usize) -> Result<usize, MemoryError> {
    if (1..=100).contains(&limit) {
        Ok(limit)
    } else {
        Err(MemoryError::Validation(
            "limit must be between 1 and 100".into(),
        ))
    }
}

pub(super) fn resolve_scope_code(
    scope: MemoryScope,
    requested: Option<&str>,
    context: &MemoryRunContext,
) -> Result<String, MemoryError> {
    if let Some(value) = normalized_optional(requested) {
        validate_scope_code(scope, &value, context)?;
        return Ok(value);
    }
    Ok(match scope {
        MemoryScope::User => context.user_code.clone(),
        MemoryScope::WorkDomain => context.work_domain_code.clone(),
        MemoryScope::Space => context.workspace_code.clone(),
    })
}

pub(super) fn validate_scope_code(
    scope: MemoryScope,
    scope_code: &str,
    context: &MemoryRunContext,
) -> Result<(), MemoryError> {
    let expected = match scope {
        MemoryScope::User => context.user_code.as_str(),
        MemoryScope::WorkDomain => context.work_domain_code.as_str(),
        MemoryScope::Space => context.workspace_code.as_str(),
    };
    if scope_code == expected {
        Ok(())
    } else {
        Err(MemoryError::Validation(
            "scopeCode is outside the current agent run context".into(),
        ))
    }
}

pub(super) fn validate_known_scope_code(
    scope_code: &str,
    context: &MemoryRunContext,
) -> Result<(), MemoryError> {
    if scope_code == context.user_code
        || scope_code == context.work_domain_code
        || scope_code == context.workspace_code
    {
        Ok(())
    } else {
        Err(MemoryError::Validation(
            "scopeCode is outside the current agent run context".into(),
        ))
    }
}

pub(super) fn derive_source_code(
    semantic_key: Option<&str>,
    scope: MemoryScope,
    scope_code: Option<&str>,
    text: &str,
) -> Result<String, MemoryError> {
    let seed = if let Some(value) = normalized_optional(semantic_key) {
        value
    } else {
        let mut seed = BTreeMap::new();
        seed.insert("scopeType", scope.as_public().to_string());
        seed.insert(
            "scopeCode",
            normalized_optional(scope_code).unwrap_or_default(),
        );
        seed.insert("text", text.to_string());
        serde_json::to_string(&seed)
            .map_err(|_| MemoryError::Validation("sourceCode seed is invalid".into()))?
    };
    let digest = Sha256::digest(seed.as_bytes());
    let hex = format!("{digest:x}");
    Ok(format!("memory-{}-{}", scope.source_prefix(), &hex[..24]))
}

pub(super) fn normalized_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn normalized_required(value: &str, field: &str) -> Result<String, MemoryError> {
    normalized_optional(Some(value))
        .ok_or_else(|| MemoryError::Validation(format!("{field} is required")))
}

fn score_to_i64(value: &Value, field: &str) -> Result<i64, MemoryError> {
    if let Some(value) = value.as_i64() {
        return Ok(value);
    }
    let Some(value) = value.as_f64() else {
        return Err(MemoryError::Validation(format!("{field} must be a number")));
    };
    if !value.is_finite() {
        return Err(MemoryError::Validation(format!("{field} must be finite")));
    }
    Ok(value.round() as i64)
}

pub(super) fn merge_optional_metadata(
    target: &mut Map<String, Value>,
    metadata: Option<&Value>,
) -> Result<(), MemoryError> {
    if let Some(metadata) = metadata {
        let object = metadata
            .as_object()
            .ok_or_else(|| MemoryError::Validation("metadata must be an object".into()))?;
        for (key, value) in object {
            target.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

pub(super) fn object_from_value(value: Value) -> Result<Map<String, Value>, MemoryError> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| MemoryError::Validation("metadata must be an object".into()))
}

pub(super) fn ensure_array_or_object_array(value: &Value, field: &str) -> Result<(), MemoryError> {
    if value
        .as_array()
        .is_some_and(|items| items.iter().all(Value::is_object))
    {
        Ok(())
    } else {
        Err(MemoryError::Validation(format!(
            "{field} must be an array of objects"
        )))
    }
}

pub(super) fn insert_optional_string(
    map: &mut Map<String, Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value.and_then(|value| normalized_optional(Some(value.as_str()))) {
        map.insert(key.into(), json!(value));
    }
}

pub(super) fn insert_optional_i64(map: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    if let Some(value) = value {
        map.insert(key.into(), json!(value));
    }
}

pub(super) fn save_response(
    action: &str,
    locator: MemoryLocator,
    payload: Value,
    rag: MemoryRagWriteResult,
) -> MemorySaveResponse {
    MemorySaveResponse {
        success: true,
        action: action.to_string(),
        memory_code: encode_memory_code(&locator),
        scope: locator.scope.as_public().into(),
        scope_code: locator.scope_code,
        memory_type: string_from_payload(&payload, "memoryType"),
        memory_status: string_from_payload(&payload, "memoryStatus"),
        review_status: string_from_payload(&payload, "reviewStatus"),
        injectable: bool_from_payload(&payload, "injectable"),
        rag,
    }
}

pub(super) fn string_from_payload(payload: &Value, field: &str) -> Option<String> {
    payload
        .get("domainFields")
        .and_then(Value::as_object)
        .and_then(|fields| fields.get(field))
        .and_then(Value::as_str)
        .or_else(|| payload.get(field).and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

pub(super) fn bool_from_payload(payload: &Value, field: &str) -> Option<bool> {
    payload
        .get("domainFields")
        .and_then(Value::as_object)
        .and_then(|fields| fields.get(field))
        .and_then(Value::as_bool)
        .or_else(|| payload.get(field).and_then(Value::as_bool))
}

fn field_string(fields: &Map<String, Value>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn field_i64(fields: &Map<String, Value>, key: &str) -> Option<i64> {
    fields.get(key).and_then(Value::as_i64)
}

fn field_bool(fields: &Map<String, Value>, key: &str) -> Option<bool> {
    fields.get(key).and_then(Value::as_bool)
}

pub(super) fn memory_token_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}
