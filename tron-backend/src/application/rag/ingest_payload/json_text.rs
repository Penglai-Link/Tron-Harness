use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    CompiledRagIngestProfile, PreparedIngestPayload, RagIngestRequest, RagTextUnitDraft,
    stable_hash, text_unit_code,
};

pub(super) fn prepare_json_text_payload(
    request: &RagIngestRequest,
    compiled: &CompiledRagIngestProfile,
) -> Result<PreparedIngestPayload, RagError> {
    let mapping = compiled
        .payload_mapping
        .as_object()
        .ok_or(RagError::IngestProfileInvalid)?;
    validate_supported_source_type(mapping, &request.source_type)?;
    validate_required_fields(mapping, &request.payload)?;
    let domain_fields = extract_domain_fields(mapping, &request.payload)?;
    let title = mapping
        .get("title")
        .and_then(Value::as_str)
        .and_then(|path| path_string(&request.payload, path));
    let text = if let Some(template) = mapping.get("textTemplate").and_then(Value::as_str) {
        render_text_template(template, &domain_fields)?
    } else {
        mapping
            .get("text")
            .and_then(Value::as_str)
            .and_then(|path| path_string(&request.payload, path))
            .ok_or(RagError::IngestRequestInvalid)?
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(RagError::IngestRequestInvalid);
    }
    let heading_path = mapping
        .get("headingPath")
        .and_then(Value::as_str)
        .and_then(|path| path_string(&request.payload, path))
        .or_else(|| title.clone())
        .unwrap_or_default();
    let content_hash = stable_hash(&text);
    let unit = RagTextUnitDraft {
        text_unit_code: text_unit_code(&request.source_code, &request.source_version, 0),
        document_code: None,
        text,
        heading_path,
        ordinal: 0,
        content_hash: content_hash.clone(),
    };
    Ok(PreparedIngestPayload {
        title,
        content_hash,
        metadata: payload_metadata_from_mapping(mapping, &request.payload),
        domain_fields,
        units: vec![unit],
    })
}

fn validate_supported_source_type(
    mapping: &serde_json::Map<String, Value>,
    source_type: &str,
) -> Result<(), RagError> {
    let supported = mapping
        .get("supportedSourceTypes")
        .and_then(Value::as_array)
        .ok_or(RagError::IngestProfileInvalid)?;
    if supported
        .iter()
        .filter_map(Value::as_str)
        .any(|value| value == source_type)
    {
        Ok(())
    } else {
        Err(RagError::IngestRequestInvalid)
    }
}

fn validate_required_fields(
    mapping: &serde_json::Map<String, Value>,
    payload: &Value,
) -> Result<(), RagError> {
    let Some(required) = mapping.get("required").and_then(Value::as_array) else {
        return Ok(());
    };
    for field in required {
        let field = field.as_str().ok_or(RagError::IngestProfileInvalid)?;
        let value = path_value(payload, field).ok_or(RagError::IngestRequestInvalid)?;
        if value_is_empty(value) {
            return Err(RagError::IngestRequestInvalid);
        }
    }
    Ok(())
}

pub(super) fn extract_domain_fields(
    mapping: &serde_json::Map<String, Value>,
    payload: &Value,
) -> Result<BTreeMap<String, Value>, RagError> {
    let mut domain_fields = BTreeMap::new();
    let Some(fields) = mapping.get("fields").and_then(Value::as_object) else {
        return Ok(domain_fields);
    };
    for (logical_name, raw_field) in fields {
        let raw_field = raw_field
            .as_object()
            .ok_or(RagError::IngestProfileInvalid)?;
        let path = raw_field
            .get("path")
            .and_then(Value::as_str)
            .ok_or(RagError::IngestProfileInvalid)?;
        let value = path_value(payload, path).cloned().unwrap_or(Value::Null);
        if value.is_null()
            && raw_field
                .get("omitIfMissing")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            continue;
        }
        let value = match value {
            Value::String(value) => Value::String(value.trim().to_string()),
            value => value,
        };
        domain_fields.insert(logical_name.clone(), value);
    }
    Ok(domain_fields)
}

pub(super) fn payload_metadata_from_mapping(
    mapping: &serde_json::Map<String, Value>,
    payload: &Value,
) -> Value {
    mapping
        .get("metadataFrom")
        .and_then(Value::as_str)
        .and_then(|path| path_value(payload, path))
        .and_then(Value::as_object)
        .map(|object| Value::Object(object.clone()))
        .unwrap_or_else(|| json!({}))
}

fn render_text_template(
    template: &str,
    domain_fields: &BTreeMap<String, Value>,
) -> Result<String, RagError> {
    let mut rendered = template.to_string();
    for (field, value) in domain_fields {
        let placeholder = format!("{{{{{field}}}}}");
        rendered = rendered.replace(&placeholder, &template_value(value));
    }
    if rendered.contains("{{") || rendered.contains("}}") {
        return Err(RagError::IngestRequestInvalid);
    }
    Ok(rendered)
}

fn path_string(payload: &Value, path: &str) -> Option<String> {
    path_value(payload, path)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn path_value<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let trimmed = path.trim();
    let normalized = trimmed.strip_prefix("$.").unwrap_or(trimmed);
    if normalized.is_empty() || normalized == "$" {
        return Some(payload);
    }
    let mut current = payload;
    for segment in normalized.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

fn value_is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        _ => false,
    }
}

fn template_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.trim().to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        value => serde_json::to_string(value).unwrap_or_default(),
    }
}
