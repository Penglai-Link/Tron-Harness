use std::collections::BTreeMap;

use serde_json::Value;

use crate::domain::rag::RagError;

use super::{parse_json_value, to_sql_error};

pub(super) fn parse_field_mapping(
    raw: &str,
) -> rusqlite::Result<(
    BTreeMap<String, String>,
    BTreeMap<String, String>,
    BTreeMap<String, String>,
)> {
    let value: Value = parse_json_value(raw)?;
    let object = value
        .as_object()
        .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
    if object.get("source").is_some() || object.get("textUnit").is_some() {
        let source = parse_field_mapping_group(object.get("source"))?;
        let text_unit = parse_field_mapping_group(object.get("textUnit"))?;
        let mut merged = source.clone();
        merged.extend(text_unit.clone());
        return Ok((merged, source, text_unit));
    }
    let flat = parse_field_mapping_group(Some(&value))?;
    Ok((flat.clone(), flat, BTreeMap::new()))
}

fn parse_field_mapping_group(value: Option<&Value>) -> rusqlite::Result<BTreeMap<String, String>> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let object = value
        .as_object()
        .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
    let mut mapping = BTreeMap::new();
    for (logical_name, column) in object {
        let column = column
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| to_sql_error(RagError::StorageBindingInvalid))?;
        mapping.insert(logical_name.clone(), column.to_string());
    }
    Ok(mapping)
}
