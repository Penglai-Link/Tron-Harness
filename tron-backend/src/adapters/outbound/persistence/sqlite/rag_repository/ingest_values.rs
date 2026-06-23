use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Transaction, params_from_iter, types::Value as SqlValue};
use serde_json::Value;

use crate::{application::rag::RagPersistIngestCommand, domain::rag::RagError};

use super::{
    sql_plan::{SqliteRagCorpusPlan, quote_identifier},
    storage_error,
};

pub(crate) fn validate_domain_field_targets(
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    for logical_name in command.domain_fields.keys() {
        if !plan.source_field_mapping.contains_key(logical_name)
            && !plan.text_unit_field_mapping.contains_key(logical_name)
            && !plan.field_mapping.contains_key(logical_name)
        {
            return Err(RagError::StorageBindingInvalid);
        }
    }
    Ok(())
}

pub(crate) fn push_domain_values(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    mapping: &BTreeMap<String, String>,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    for (logical_name, value) in &command.domain_fields {
        let Some(column) = mapping.get(logical_name) else {
            continue;
        };
        if !columns.contains(column) {
            return Err(RagError::StorageBindingInvalid);
        }
        if !values.iter().any(|(existing, _)| existing == column) {
            values.push((column.clone(), json_to_sql_value(value)?));
        }
    }
    Ok(())
}

pub(crate) fn push_available_domain_values(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    mapping: &BTreeMap<String, String>,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    for (logical_name, value) in &command.domain_fields {
        let Some(column) = mapping.get(logical_name) else {
            continue;
        };
        if columns.contains(column) && !values.iter().any(|(existing, _)| existing == column) {
            values.push((column.clone(), json_to_sql_value(value)?));
        }
    }
    Ok(())
}

pub(crate) fn push_embedded_source_values(
    values: &mut Vec<(String, SqlValue)>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
    now: i64,
) -> Result<(), RagError> {
    push_text(
        values,
        &plan.text_unit_columns,
        "source_type",
        &command.source_type,
    );
    push_dynamic_text(
        values,
        &plan.text_unit_columns,
        &plan.source_status_column,
        &command.source_status,
    );
    push_optional_text(
        values,
        &plan.text_unit_columns,
        "title",
        command.title.as_deref(),
    );
    if let Some(column) = plan.source_content_hash_column.as_deref() {
        push_dynamic_text(
            values,
            &plan.text_unit_columns,
            column,
            &command.content_hash,
        );
    }
    if let Some(column) = plan.source_metadata_column.as_deref() {
        push_dynamic_text(
            values,
            &plan.text_unit_columns,
            column,
            &json_string(&command.metadata)?,
        );
    }
    if let Some(column) = plan.source_created_at_column.as_deref() {
        push_integer(values, &plan.text_unit_columns, column, now);
    }
    if let Some(column) = plan.source_updated_at_column.as_deref() {
        push_integer(values, &plan.text_unit_columns, column, now);
    }
    let mapping = if plan.source_field_mapping.is_empty() {
        &plan.field_mapping
    } else {
        &plan.source_field_mapping
    };
    push_domain_values(values, &plan.text_unit_columns, mapping, command)?;
    Ok(())
}

pub(crate) fn execute_upsert(
    tx: &Transaction<'_>,
    table_name: &str,
    values: Vec<(String, SqlValue)>,
    conflict_columns: &[&str],
    skip_update_columns: &[Option<&str>],
) -> Result<(), RagError> {
    if values.is_empty() {
        return Err(RagError::StorageBindingInvalid);
    }
    let table = quote_identifier(table_name)?;
    let columns = values
        .iter()
        .map(|(column, _)| quote_identifier(column))
        .collect::<Result<Vec<_>, _>>()?;
    let placeholders = (1..=values.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>();
    let conflict = conflict_columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let skipped = skip_update_columns
        .iter()
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>();
    let update_columns = values
        .iter()
        .map(|(column, _)| column.as_str())
        .filter(|column| !conflict_columns.contains(column) && !skipped.contains(column))
        .collect::<Vec<_>>();
    let update_sql = if update_columns.is_empty() {
        "DO NOTHING".to_string()
    } else {
        format!(
            "DO UPDATE SET {}",
            update_columns
                .iter()
                .map(|column| {
                    let column = quote_identifier(column)?;
                    Ok(format!("{column} = excluded.{column}"))
                })
                .collect::<Result<Vec<_>, RagError>>()?
                .join(", ")
        )
    };
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT ({conflict}) {update_sql}",
        columns.join(", "),
        placeholders.join(", ")
    );
    tx.execute(
        &sql,
        params_from_iter(values.into_iter().map(|(_, value)| value)),
    )
    .map_err(storage_error)?;
    Ok(())
}

fn json_to_sql_value(value: &Value) -> Result<SqlValue, RagError> {
    match value {
        Value::Null => Ok(SqlValue::Null),
        Value::Bool(value) => Ok(SqlValue::Integer(i64::from(*value))),
        Value::Number(value) => value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
            .map(SqlValue::Integer)
            .or_else(|| value.as_f64().map(|value| SqlValue::Real(value)))
            .ok_or(RagError::StorageBindingInvalid),
        Value::String(value) => Ok(SqlValue::Text(value.to_string())),
        value => json_string(value).map(SqlValue::Text),
    }
}

pub(crate) fn push_text(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    column: &str,
    value: &str,
) {
    if columns.contains(column) {
        values.push((column.to_string(), SqlValue::Text(value.to_string())));
    }
}

pub(crate) fn push_dynamic_text(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    column: &str,
    value: &str,
) {
    push_text(values, columns, column, value);
}

pub(crate) fn push_optional_text(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    column: &str,
    value: Option<&str>,
) {
    if columns.contains(column) {
        values.push((
            column.to_string(),
            value
                .map(|value| SqlValue::Text(value.to_string()))
                .unwrap_or(SqlValue::Null),
        ));
    }
}

pub(crate) fn push_integer(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    column: &str,
    value: i64,
) {
    if columns.contains(column) {
        values.push((column.to_string(), SqlValue::Integer(value)));
    }
}

pub(crate) fn push_optional_blob(
    values: &mut Vec<(String, SqlValue)>,
    columns: &BTreeSet<String>,
    column: &str,
    value: Option<&[u8]>,
) {
    if columns.contains(column) {
        values.push((
            column.to_string(),
            value
                .map(|value| SqlValue::Blob(value.to_vec()))
                .unwrap_or(SqlValue::Null),
        ));
    }
}

pub(crate) fn json_string(value: &Value) -> Result<String, RagError> {
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
