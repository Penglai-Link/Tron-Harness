use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::domain::rag::{RagCorpusRecord, RagError};

#[derive(Debug, Clone, PartialEq, Eq)]
enum FilterType {
    String,
    Integer,
    Boolean,
}

#[derive(Debug, Clone)]
struct FilterField {
    field_type: FilterType,
}

pub fn merge_search_filters(
    corpus: &RagCorpusRecord,
    profile_filters: &Value,
    request_filters: &Value,
) -> Result<Value, RagError> {
    let schema = parse_schema(&corpus.filter_schema);
    let mut merged = normalize_filters(&schema, profile_filters)?;
    let request = normalize_filters(&schema, request_filters)?;
    for (field, request_value) in request {
        if let Some(profile_value) = merged.get(&field) {
            let intersection = intersect_filter_values(profile_value, &request_value)
                .ok_or(RagError::SearchFilterInvalid)?;
            merged.insert(field, intersection);
        } else {
            merged.insert(field, request_value);
        }
    }
    Ok(Value::Object(merged.into_iter().collect()))
}

fn normalize_filters(
    schema: &BTreeMap<String, FilterField>,
    filters: &Value,
) -> Result<BTreeMap<String, Value>, RagError> {
    let object = filters.as_object().ok_or(RagError::SearchFilterInvalid)?;
    let mut normalized = BTreeMap::new();
    for (field, raw_value) in object {
        let schema_field = schema.get(field).ok_or(RagError::SearchFilterInvalid)?;
        let (operator, value) = extract_operator(raw_value);
        if !matches!(operator, "eq" | "in") || !value_matches(&schema_field.field_type, value) {
            return Err(RagError::SearchFilterInvalid);
        }
        let normalized_value = if operator == "in" {
            json!({ "op": "in", "value": value.clone() })
        } else {
            json!({ "op": "eq", "value": value.clone() })
        };
        normalized.insert(field.clone(), normalized_value);
    }
    Ok(normalized)
}

fn intersect_filter_values(left: &Value, right: &Value) -> Option<Value> {
    let left_values = filter_values(left)?;
    let right_values = filter_values(right)?;
    let intersection = left_values
        .intersection(&right_values)
        .cloned()
        .collect::<BTreeSet<_>>();
    if intersection.is_empty() || intersection.len() != right_values.len() {
        return None;
    }
    let values = intersection
        .into_iter()
        .map(|value| serde_json::from_str::<Value>(&value).ok())
        .collect::<Option<Vec<_>>>()?;
    if values.len() == 1 {
        Some(json!({ "op": "in", "value": values }))
    } else {
        Some(json!({ "op": "in", "value": values }))
    }
}

fn filter_values(value: &Value) -> Option<BTreeSet<String>> {
    let operator = value.get("op")?.as_str()?;
    let raw = value.get("value")?;
    if operator == "in" {
        raw.as_array()?
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<BTreeSet<_>, _>>()
            .ok()
    } else {
        Some(BTreeSet::from([serde_json::to_string(raw).ok()?]))
    }
}

fn parse_schema(value: &Value) -> BTreeMap<String, FilterField> {
    value
        .get("fields")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(name, raw)| {
                    let field_type = match raw.get("type").and_then(Value::as_str) {
                        Some("integer" | "number") => FilterType::Integer,
                        Some("boolean") => FilterType::Boolean,
                        _ => FilterType::String,
                    };
                    (name.clone(), FilterField { field_type })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_operator(raw_value: &Value) -> (&str, &Value) {
    if let Some(object) = raw_value.as_object()
        && let (Some(operator), Some(value)) = (
            object.get("op").and_then(Value::as_str),
            object.get("value"),
        )
    {
        return (operator, value);
    }
    if raw_value.is_array() {
        ("in", raw_value)
    } else {
        ("eq", raw_value)
    }
}

fn value_matches(field_type: &FilterType, value: &Value) -> bool {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .all(|item| single_value_matches(field_type, item));
    }
    single_value_matches(field_type, value)
}

fn single_value_matches(field_type: &FilterType, value: &Value) -> bool {
    match field_type {
        FilterType::String => value.is_string(),
        FilterType::Integer => value.as_i64().is_some(),
        FilterType::Boolean => value.is_boolean(),
    }
}
