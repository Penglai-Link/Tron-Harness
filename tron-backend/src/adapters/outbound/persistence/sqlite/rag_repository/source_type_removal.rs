use std::collections::BTreeSet;

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::domain::rag::RagError;

pub(super) fn apply(connection: &Connection) -> Result<(), RagError> {
    drop_column_if_exists(connection, "rag_memory_text_units", "source_type")?;
    drop_column_if_exists(connection, "rag_project_doc_sources", "source_type")?;
    drop_column_if_exists(connection, "rag_skill_text_units", "source_type")?;
    sanitize_profile_source_type_json(connection)?;
    sanitize_corpus_source_type_json(connection)
}

fn drop_column_if_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<(), RagError> {
    let columns = table_columns_for_known_table(connection, table_name)?;
    if !columns.contains(column_name) {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {table_name} DROP COLUMN {column_name}");
    connection.execute_batch(&sql).map_err(storage_error)
}

fn table_columns_for_known_table(
    connection: &Connection,
    table_name: &str,
) -> Result<BTreeSet<String>, RagError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(storage_error)
}

fn sanitize_profile_source_type_json(connection: &Connection) -> Result<(), RagError> {
    let mut statement = connection
        .prepare("SELECT profile_id, config_json FROM rag_profiles")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    drop(statement);

    for (profile_id, config_json) in rows {
        let mut config =
            serde_json::from_str::<Value>(&config_json).map_err(|_| RagError::StorageFailure)?;
        let changed = remove_json_key_recursive(&mut config, "supportedSourceTypes")
            | remove_empty_source_input_policy(&mut config);
        if changed {
            connection
                .execute(
                    "UPDATE rag_profiles SET config_json = ?2 WHERE profile_id = ?1",
                    params![profile_id, config.to_string()],
                )
                .map_err(storage_error)?;
        }
    }
    Ok(())
}

fn sanitize_corpus_source_type_json(connection: &Connection) -> Result<(), RagError> {
    let mut statement = connection
        .prepare("SELECT corpus_id, field_mapping_json FROM rag_corpora")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    drop(statement);

    for (corpus_id, field_mapping_json) in rows {
        let mut mapping = serde_json::from_str::<Value>(&field_mapping_json)
            .map_err(|_| RagError::StorageFailure)?;
        let changed = mapping
            .get_mut("source")
            .and_then(Value::as_object_mut)
            .is_some_and(|source| source.remove("sourceType").is_some());
        if changed {
            connection
                .execute(
                    "UPDATE rag_corpora SET field_mapping_json = ?2 WHERE corpus_id = ?1",
                    params![corpus_id, mapping.to_string()],
                )
                .map_err(storage_error)?;
        }
    }
    Ok(())
}

fn remove_json_key_recursive(value: &mut Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            let mut changed = object.remove(key).is_some();
            for value in object.values_mut() {
                changed |= remove_json_key_recursive(value, key);
            }
            changed
        }
        Value::Array(values) => {
            let mut changed = false;
            for value in values {
                changed |= remove_json_key_recursive(value, key);
            }
            changed
        }
        _ => false,
    }
}

fn remove_empty_source_input_policy(config: &mut Value) -> bool {
    let Some(object) = config.as_object_mut() else {
        return false;
    };
    let Some(policy) = object.get("sourceInputPolicy") else {
        return false;
    };
    if policy.as_object().is_some_and(serde_json::Map::is_empty) {
        object.remove("sourceInputPolicy");
        return true;
    }
    false
}

fn storage_error(_error: rusqlite::Error) -> RagError {
    RagError::StorageFailure
}
