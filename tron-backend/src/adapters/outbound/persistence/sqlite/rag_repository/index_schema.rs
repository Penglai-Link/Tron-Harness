use super::SqliteRagRepositorySql;
use std::collections::BTreeSet;

use rusqlite::Connection;
use serde_json::Value;

use crate::{
    domain::rag::{RagCorpusRecord, RagError},
    shared::sqlite_extensions::{
        sqlite_fts5_available, sqlite_simple_tokenizer_available, sqlite_vec_available,
    },
};

use super::{
    indexes::{fts_table_name, supported_vector_dimensions, vec_table_name},
    sql_plan::quote_identifier,
    storage_error,
};

mod tree_schema;
use tree_schema::ensure_tree_storage_table;

pub(super) fn ensure_runtime_storage_for_corpus(
    repository: &SqliteRagRepositorySql<'_>,
    corpus: &RagCorpusRecord,
) -> Result<(), RagError> {
    let binding = &corpus.storage_binding;
    let text_unit_table = binding.sqlite_table_for_segment("textUnit");
    if !has_sqlite_runtime_table(corpus) {
        return Ok(());
    }
    let source_table = binding.sqlite_table_for_segment("source");
    if let Some(source_table) = source_table.filter(|table| Some(*table) != text_unit_table) {
        ensure_source_table(&repository.connection, source_table, corpus)?;
    }
    if let Some(text_unit_table) = text_unit_table {
        ensure_text_unit_table(&repository.connection, text_unit_table, corpus)?;
        if binding.sqlite_table_for_segment("vector") == Some(text_unit_table) {
            ensure_vector_tables(&repository.connection, text_unit_table)?;
        }
    }

    if let Some(fulltext_table) = binding.sqlite_table_for_segment("fulltext") {
        if text_unit_table == Some(fulltext_table) {
            ensure_fts_table(&repository.connection, fulltext_table)?;
        } else {
            ensure_fulltext_storage_table(&repository.connection, fulltext_table, corpus)?;
            ensure_fts_table(&repository.connection, fulltext_table)?;
        }
    }

    if let Some(vector_table) = binding
        .sqlite_table_for_segment("vector")
        .filter(|table| text_unit_table != Some(*table))
    {
        ensure_vector_storage_table(&repository.connection, vector_table, corpus)?;
        ensure_vector_tables(&repository.connection, vector_table)?;
    }
    if let Some(tree_table) = binding.sqlite_table_for_segment("tree") {
        ensure_tree_storage_table(&repository.connection, tree_table)?;
        ensure_fts_table(&repository.connection, tree_table)?;
        ensure_vector_tables(&repository.connection, tree_table)?;
    }
    if let Some(diagnostics_table) = binding.diagnostics_table() {
        ensure_diagnostics_storage_table(&repository.connection, diagnostics_table)?;
    }
    Ok(())
}

fn has_sqlite_runtime_table(corpus: &RagCorpusRecord) -> bool {
    [
        "source",
        "textUnit",
        "fulltext",
        "vector",
        "tree",
        "diagnostics",
    ]
    .into_iter()
    .any(|segment| {
        corpus
            .storage_binding
            .sqlite_table_for_segment(segment)
            .is_some()
    })
}

fn ensure_fts_table(connection: &Connection, table_name: &str) -> Result<(), RagError> {
    if !sqlite_fts5_available(connection) || !sqlite_simple_tokenizer_available(connection) {
        return Err(RagError::SearchRouteUnavailable);
    }
    let tokenizer = "simple";
    let fts_table = quote_identifier(&fts_table_name(table_name))?;
    connection
        .execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS {fts_table}
             USING fts5(search_text, tokenize='{tokenizer}');"
        ))
        .map_err(storage_error)
}

fn ensure_vector_tables(connection: &Connection, table_name: &str) -> Result<(), RagError> {
    if !sqlite_vec_available(connection) {
        return Ok(());
    }
    for dimension in supported_vector_dimensions() {
        let vec_table_name = vec_table_name(table_name, *dimension);
        let vec_table = quote_identifier(&vec_table_name)?;
        connection
            .execute_batch(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS {vec_table}
                 USING vec0(embedding float[{dimension}]);"
            ))
            .map_err(storage_error)?;
    }
    Ok(())
}

fn ensure_vector_storage_table(
    connection: &Connection,
    table_name: &str,
    corpus: &RagCorpusRecord,
) -> Result<(), RagError> {
    let columns = carrier_text_unit_columns(corpus)?;
    let table = quote_identifier(table_name)?;
    let source_index = quote_identifier(&format!("idx_{table_name}_source_scope"))?;
    let column_sql = dynamic_columns_sql(&columns);
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                {column_sql},
                UNIQUE (text_unit_code),
                UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
            );
            CREATE INDEX IF NOT EXISTS {source_index}
                ON {table} (source_code, source_version, scope_type, scope_code, ordinal);"
        ))
        .map_err(storage_error)?;
    validate_storage_columns(connection, table_name, &columns)
}

fn ensure_source_table(
    connection: &Connection,
    table_name: &str,
    corpus: &RagCorpusRecord,
) -> Result<(), RagError> {
    let mut columns = BTreeSet::from([
        "source_code".to_string(),
        "source_version".to_string(),
        "scope_type".to_string(),
        "scope_code".to_string(),
        "status".to_string(),
        "title".to_string(),
        "metadata".to_string(),
        "body_hash".to_string(),
        "content_hash".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
    ]);
    extend_columns_from_mapping(&mut columns, corpus.source_field_mapping.values())?;
    extend_columns_from_filter_schema(
        &mut columns,
        &corpus.filter_schema,
        corpus.storage_binding.is_sqlite_backend(),
    )?;
    let table = quote_identifier(table_name)?;
    let source_index = quote_identifier(&format!("idx_{table_name}_source_scope"))?;
    let column_sql = dynamic_columns_sql(&columns);
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                {column_sql},
                UNIQUE (source_code, source_version, scope_type, scope_code)
            );
            CREATE INDEX IF NOT EXISTS {source_index}
                ON {table} (source_code, source_version, scope_type, scope_code);"
        ))
        .map_err(storage_error)?;
    validate_storage_columns(connection, table_name, &columns)
}

fn ensure_text_unit_table(
    connection: &Connection,
    table_name: &str,
    corpus: &RagCorpusRecord,
) -> Result<(), RagError> {
    let mut columns = BTreeSet::from([
        "text_unit_code".to_string(),
        "source_code".to_string(),
        "source_version".to_string(),
        "scope_type".to_string(),
        "scope_code".to_string(),
        "unit_type".to_string(),
        "text".to_string(),
        "search_text".to_string(),
        "fulltext_status".to_string(),
        "embedding_model_code".to_string(),
        "embedding_version".to_string(),
        "embedding_text_hash".to_string(),
        "embedding_dimension".to_string(),
        "embedding".to_string(),
        "embedding_status".to_string(),
        "heading_path".to_string(),
        "ordinal".to_string(),
        "token_count".to_string(),
        "status".to_string(),
        "metadata".to_string(),
        "content_hash".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
    ]);
    if has_embedded_sqlite_source(corpus) {
        columns.extend(["title".to_string(), "body_hash".to_string()]);
    }
    extend_columns_from_mapping(&mut columns, corpus.text_unit_field_mapping.values())?;
    if has_embedded_sqlite_source(corpus) {
        extend_columns_from_mapping(&mut columns, corpus.source_field_mapping.values())?;
    }
    extend_columns_from_filter_schema(
        &mut columns,
        &corpus.filter_schema,
        corpus.storage_binding.is_sqlite_backend(),
    )?;
    let table = quote_identifier(table_name)?;
    let source_index = quote_identifier(&format!("idx_{table_name}_source_scope"))?;
    let column_sql = dynamic_columns_sql(&columns);
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                {column_sql},
                UNIQUE (source_code, source_version, scope_type, scope_code, ordinal),
                UNIQUE (text_unit_code)
            );
            CREATE INDEX IF NOT EXISTS {source_index}
                ON {table} (source_code, source_version, scope_type, scope_code, ordinal);"
        ))
        .map_err(storage_error)?;
    validate_storage_columns(connection, table_name, &columns)
}

fn has_embedded_sqlite_source(corpus: &RagCorpusRecord) -> bool {
    let binding = &corpus.storage_binding;
    binding.sqlite_table_for_segment("source") == binding.sqlite_table_for_segment("textUnit")
}

fn ensure_diagnostics_storage_table(
    connection: &Connection,
    table_name: &str,
) -> Result<(), RagError> {
    let table = quote_identifier(table_name)?;
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                source_code TEXT NOT NULL,
                source_version TEXT NOT NULL,
                scope_type TEXT NOT NULL,
                scope_code TEXT NOT NULL,
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                message TEXT,
                details_json TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );"
        ))
        .map_err(storage_error)?;
    validate_storage_columns(
        connection,
        table_name,
        &BTreeSet::from([
            "source_code".to_string(),
            "source_version".to_string(),
            "scope_type".to_string(),
            "scope_code".to_string(),
            "stage".to_string(),
            "status".to_string(),
            "message".to_string(),
            "details_json".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
        ]),
    )?;
    let source_index = quote_identifier(&format!("idx_{table_name}_source_scope"))?;
    connection
        .execute_batch(&format!(
            "CREATE INDEX IF NOT EXISTS {source_index}
                ON {table} (source_code, source_version, scope_type, scope_code, stage);"
        ))
        .map_err(storage_error)
}

fn dynamic_columns_sql(columns: &BTreeSet<String>) -> String {
    columns
        .iter()
        .map(|column| {
            let column_type = if matches!(
                column.as_str(),
                "embedding_dimension" | "summary_embedding_dimension"
            ) {
                "INTEGER"
            } else if matches!(
                column.as_str(),
                "ordinal"
                    | "embedding_dimension"
                    | "token_count"
                    | "created_at"
                    | "updated_at"
                    | "level"
                    | "summary_token_count"
                    | "summary_embedding_dimension"
                    | "covered_text_unit_count"
                    | "child_start_ordinal"
                    | "child_end_ordinal"
            ) {
                "INTEGER NOT NULL DEFAULT 0"
            } else if column == "embedding" {
                "BLOB"
            } else if column == "text" {
                "TEXT NOT NULL"
            } else {
                "TEXT"
            };
            format!("\"{column}\" {column_type}")
        })
        .collect::<Vec<_>>()
        .join(",\n                ")
}

fn validate_storage_columns(
    connection: &Connection,
    table_name: &str,
    columns: &BTreeSet<String>,
) -> Result<(), RagError> {
    let existing = table_columns(connection, table_name)?;
    // Schema v2 has a fixed shape. Never convert old storage in place.
    if columns.iter().any(|column| !existing.contains(column)) {
        return Err(RagError::SchemaIncompatible);
    }
    Ok(())
}

fn extend_columns_from_mapping<'a>(
    columns: &mut BTreeSet<String>,
    mapped_columns: impl Iterator<Item = &'a String>,
) -> Result<(), RagError> {
    for column in mapped_columns {
        quote_identifier(column)?;
        columns.insert(column.clone());
    }
    Ok(())
}

fn extend_columns_from_filter_schema(
    columns: &mut BTreeSet<String>,
    filter_schema: &Value,
    strict: bool,
) -> Result<(), RagError> {
    let Some(fields) = filter_schema.get("fields").and_then(Value::as_object) else {
        return Ok(());
    };
    for column in fields
        .values()
        .filter_map(|field| field.get("column").and_then(Value::as_str))
    {
        if quote_identifier(column).is_ok() {
            columns.insert(column.to_string());
        } else if strict {
            return Err(RagError::StorageBindingInvalid);
        }
    }
    Ok(())
}

fn ensure_fulltext_storage_table(
    connection: &Connection,
    table_name: &str,
    corpus: &RagCorpusRecord,
) -> Result<(), RagError> {
    let columns = carrier_text_unit_columns(corpus)?;
    let table = quote_identifier(table_name)?;
    let source_index = quote_identifier(&format!("idx_{table_name}_source_scope"))?;
    let column_sql = dynamic_columns_sql(&columns);
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                {column_sql},
                UNIQUE (text_unit_code),
                UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
            );
            CREATE INDEX IF NOT EXISTS {source_index}
                ON {table} (source_code, source_version, scope_type, scope_code, ordinal);"
        ))
        .map_err(storage_error)?;
    validate_storage_columns(connection, table_name, &columns)
}

fn carrier_text_unit_columns(corpus: &RagCorpusRecord) -> Result<BTreeSet<String>, RagError> {
    let mut columns = BTreeSet::from([
        "text_unit_code".to_string(),
        "source_code".to_string(),
        "source_version".to_string(),
        "scope_type".to_string(),
        "scope_code".to_string(),
        "unit_type".to_string(),
        "text".to_string(),
        "search_text".to_string(),
        "fulltext_status".to_string(),
        "embedding_model_code".to_string(),
        "embedding_version".to_string(),
        "embedding_text_hash".to_string(),
        "embedding_dimension".to_string(),
        "embedding".to_string(),
        "embedding_status".to_string(),
        "heading_path".to_string(),
        "ordinal".to_string(),
        "token_count".to_string(),
        "status".to_string(),
        "metadata".to_string(),
        "content_hash".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
    ]);
    extend_valid_columns_from_mapping(&mut columns, corpus.text_unit_field_mapping.values());
    extend_columns_from_filter_schema(&mut columns, &corpus.filter_schema, false)?;
    Ok(columns)
}

fn extend_valid_columns_from_mapping<'a>(
    columns: &mut BTreeSet<String>,
    mapped_columns: impl Iterator<Item = &'a String>,
) {
    for column in mapped_columns {
        if quote_identifier(column).is_ok() {
            columns.insert(column.clone());
        }
    }
}

fn table_columns(connection: &Connection, table_name: &str) -> Result<BTreeSet<String>, RagError> {
    let table = quote_identifier(table_name)?;
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(storage_error)?;
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(storage_error)
}
