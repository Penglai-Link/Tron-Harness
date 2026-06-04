use rusqlite::Connection;

use crate::{
    application::rag::RagConfigRepository,
    domain::rag::{RagError, RagStatus, RagStorageLayout},
    shared::sqlite_extensions::{
        sqlite_fts5_available, sqlite_simple_tokenizer_available, sqlite_vec_available,
    },
};

use super::{
    SqliteRagRepository,
    indexes::{fts_table_name, supported_vector_dimensions, table_exists, vec_table_name},
    sql_plan::quote_identifier,
    storage_error,
};

pub(super) fn apply_extension_index_migration(
    repository: &SqliteRagRepository,
) -> Result<(), RagError> {
    drop_retired_extension_tables(&repository.connection)?;
    for table in configured_text_unit_tables(repository)? {
        if !table_exists(&repository.connection, &table)? {
            continue;
        }
        ensure_fts_table(&repository.connection, &table)?;
        ensure_vector_tables(&repository.connection, &table)?;
    }
    rebuild_all_extension_indexes(repository)
}

fn drop_retired_extension_tables(connection: &Connection) -> Result<(), RagError> {
    connection
        .execute_batch(
            "DROP TABLE IF EXISTS rag_sqlite_vector_indexes;
             DROP TABLE IF EXISTS rag_sqlite_extension_capabilities;",
        )
        .map_err(storage_error)?;
    Ok(())
}

fn configured_text_unit_tables(repository: &SqliteRagRepository) -> Result<Vec<String>, RagError> {
    let mut tables = Vec::new();
    for corpus in repository.list_corpora()? {
        if corpus.status != RagStatus::Enabled {
            continue;
        }
        let table = match corpus.storage_binding.layout {
            RagStorageLayout::EmbeddedSource => corpus.storage_binding.table,
            RagStorageLayout::Split => corpus.storage_binding.text_unit_table,
            RagStorageLayout::GraphSingleNode => None,
        };
        let Some(table) = table else {
            continue;
        };
        if tables.contains(&table) {
            continue;
        }
        quote_identifier(&table)?;
        tables.push(table);
    }
    Ok(tables)
}

fn rebuild_all_extension_indexes(repository: &SqliteRagRepository) -> Result<(), RagError> {
    for table in configured_text_unit_tables(repository)? {
        if !table_exists(&repository.connection, &table)? {
            continue;
        }
        rebuild_table_indexes(&repository.connection, &table)?;
    }
    Ok(())
}

fn rebuild_table_indexes(connection: &Connection, table_name: &str) -> Result<(), RagError> {
    let fts_table_name = fts_table_name(table_name);
    if table_exists(connection, &fts_table_name)? {
        let fts_table = quote_identifier(&fts_table_name)?;
        connection
            .execute(&format!("DELETE FROM {fts_table}"), [])
            .map_err(storage_error)?;
        let source_table = quote_identifier(table_name)?;
        connection
            .execute(
                &format!(
                    "INSERT INTO {fts_table}(rowid, search_text)
                     SELECT id, text
                     FROM {source_table}
                     WHERE status = 'ACTIVE'"
                ),
                [],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn ensure_fts_table(connection: &Connection, table_name: &str) -> Result<(), RagError> {
    if !sqlite_fts5_available(connection) {
        return Ok(());
    }
    let tokenizer = if sqlite_simple_tokenizer_available(connection) {
        "simple"
    } else {
        "unicode61"
    };
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
