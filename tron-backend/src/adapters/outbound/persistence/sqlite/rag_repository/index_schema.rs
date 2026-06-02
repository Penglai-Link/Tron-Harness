use rusqlite::{Connection, params};

use crate::{
    domain::rag::RagError,
    shared::sqlite_extensions::{
        sqlite_fts5_available, sqlite_simple_tokenizer_available, sqlite_vec_available,
    },
};

use super::{
    SqliteRagRepository,
    indexes::{
        column_exists, decode_embedding, fts_table_name, legacy_vector_columns_exist,
        supported_vector_dimensions, table_exists, vec_table_name,
    },
    sql_plan::quote_identifier,
    storage_error,
};

pub(super) fn apply_extension_index_migration(
    repository: &SqliteRagRepository,
) -> Result<(), RagError> {
    drop_retired_extension_tables(&repository.connection)?;
    for table in ["rag_memory_text_units", "rag_project_doc_text_units"] {
        if !table_exists(&repository.connection, table)? {
            continue;
        }
        ensure_fts_table(&repository.connection, table)?;
        ensure_vector_tables(&repository.connection, table)?;
    }
    rebuild_all_extension_indexes(repository)?;
    drop_retired_text_unit_index_columns(&repository.connection)
}

fn drop_retired_extension_tables(connection: &Connection) -> Result<(), RagError> {
    connection
        .execute_batch(
            "DROP TABLE IF EXISTS rag_sqlite_vector_indexes;
             DROP TABLE IF EXISTS rag_sqlite_extension_capabilities;",
        )
        .map_err(storage_error)?;

    if sqlite_vec_available(connection) {
        connection
            .execute_batch(
                "DROP TABLE IF EXISTS rag_memory_text_units_vec_d2;
                 DROP TABLE IF EXISTS rag_memory_text_units_vec_d1536;
                 DROP TABLE IF EXISTS rag_memory_text_units_vec_d2048;
                 DROP TABLE IF EXISTS rag_project_doc_text_units_vec_d2;
                 DROP TABLE IF EXISTS rag_project_doc_text_units_vec_d1536;
                 DROP TABLE IF EXISTS rag_project_doc_text_units_vec_d2048;",
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn drop_retired_text_unit_index_columns(connection: &Connection) -> Result<(), RagError> {
    if table_exists(connection, "rag_memory_text_units")?
        && column_exists(connection, "rag_memory_text_units", "search_text")?
    {
        rebuild_memory_text_units_without_index_payload(connection)?;
    }
    if table_exists(connection, "rag_project_doc_text_units")?
        && column_exists(connection, "rag_project_doc_text_units", "search_text")?
    {
        rebuild_project_doc_text_units_without_index_payload(connection)?;
    }
    Ok(())
}

fn rebuild_memory_text_units_without_index_payload(
    connection: &Connection,
) -> Result<(), RagError> {
    connection
        .execute_batch(
            "DROP INDEX IF EXISTS idx_rag_memory_text_units_source;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_scope_status;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_fulltext_status;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_embedding_model;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_search_scope;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_vector;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_source_status;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_domain_status;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_domain_rank;
             DROP INDEX IF EXISTS idx_rag_memory_text_units_expires;
             DROP TABLE IF EXISTS rag_memory_text_units_index_payload_old;
             ALTER TABLE rag_memory_text_units RENAME TO rag_memory_text_units_index_payload_old;
             CREATE TABLE rag_memory_text_units (
                id INTEGER PRIMARY KEY,
                text_unit_code TEXT NOT NULL UNIQUE,
                source_code TEXT NOT NULL,
                source_version TEXT NOT NULL,
                scope_type TEXT NOT NULL,
                scope_code TEXT NOT NULL,
                unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
                text TEXT NOT NULL,
                heading_path TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                token_count INTEGER,
                content_hash TEXT,
                status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                source_type TEXT NOT NULL DEFAULT 'PLAIN_TEXT',
                source_status TEXT NOT NULL DEFAULT 'PENDING',
                title TEXT,
                source_content_hash TEXT,
                source_metadata TEXT NOT NULL DEFAULT '{}',
                source_created_at INTEGER NOT NULL DEFAULT 0,
                source_updated_at INTEGER NOT NULL DEFAULT 0,
                memory_type TEXT,
                memory_status TEXT,
                review_status TEXT,
                version TEXT,
                superseded_by TEXT,
                conflict_group TEXT,
                duplicate_group TEXT,
                injectable INTEGER NOT NULL DEFAULT 1,
                importance_score INTEGER,
                confidence_score INTEGER,
                quality_score INTEGER,
                pin_level INTEGER NOT NULL DEFAULT 0,
                expires_at INTEGER,
                derived_source_type TEXT,
                derived_source_code TEXT,
                UNIQUE (source_code, source_version, ordinal)
             );
             INSERT INTO rag_memory_text_units (
                id, text_unit_code, source_code, source_version, scope_type, scope_code,
                unit_type, text, heading_path, ordinal, token_count, content_hash, status,
                metadata, created_at, updated_at, source_type, source_status, title,
                source_content_hash, source_metadata, source_created_at, source_updated_at,
                memory_type, memory_status, review_status, version, superseded_by,
                conflict_group, duplicate_group, injectable, importance_score,
                confidence_score, quality_score, pin_level, expires_at, derived_source_type,
                derived_source_code
             )
             SELECT
                id, text_unit_code, source_code, source_version, scope_type, scope_code,
                unit_type, text, heading_path, ordinal, token_count, content_hash, status,
                metadata, created_at, updated_at, source_type, source_status, title,
                source_content_hash, source_metadata, source_created_at, source_updated_at,
                memory_type, memory_status, review_status, version, superseded_by,
                conflict_group, duplicate_group, injectable, importance_score,
                confidence_score, quality_score, pin_level, expires_at, derived_source_type,
                derived_source_code
             FROM rag_memory_text_units_index_payload_old;
             DROP TABLE rag_memory_text_units_index_payload_old;
             CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_source
                ON rag_memory_text_units (source_code, source_version, ordinal);
             CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_scope_status
                ON rag_memory_text_units (scope_type, scope_code, status);
             CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_source_status
                ON rag_memory_text_units (scope_type, scope_code, source_status);
             CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_domain_status
                ON rag_memory_text_units (memory_type, memory_status, review_status, injectable);
             CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_domain_rank
                ON rag_memory_text_units (pin_level, importance_score, confidence_score, quality_score);
             CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_expires
                ON rag_memory_text_units (expires_at);",
        )
        .map_err(storage_error)
}

fn rebuild_project_doc_text_units_without_index_payload(
    connection: &Connection,
) -> Result<(), RagError> {
    connection
        .execute_batch(
            "DROP INDEX IF EXISTS idx_rag_project_doc_text_units_source;
             DROP INDEX IF EXISTS idx_rag_project_doc_text_units_scope_status;
             DROP INDEX IF EXISTS idx_rag_project_doc_text_units_document;
             DROP INDEX IF EXISTS idx_rag_project_doc_text_units_fulltext_status;
             DROP INDEX IF EXISTS idx_rag_project_doc_text_units_embedding_model;
             DROP INDEX IF EXISTS idx_rag_project_doc_text_units_search_scope;
             DROP INDEX IF EXISTS idx_rag_project_doc_text_units_vector;
             DROP TABLE IF EXISTS rag_project_doc_text_units_index_payload_old;
             ALTER TABLE rag_project_doc_text_units RENAME TO rag_project_doc_text_units_index_payload_old;
             CREATE TABLE rag_project_doc_text_units (
                id INTEGER PRIMARY KEY,
                text_unit_code TEXT NOT NULL UNIQUE,
                source_code TEXT NOT NULL,
                source_version TEXT NOT NULL,
                document_code TEXT,
                scope_type TEXT NOT NULL,
                scope_code TEXT NOT NULL,
                unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
                text TEXT NOT NULL,
                heading_path TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                token_count INTEGER,
                content_hash TEXT,
                status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE (source_code, source_version, ordinal)
             );
             INSERT INTO rag_project_doc_text_units (
                id, text_unit_code, source_code, source_version, document_code, scope_type,
                scope_code, unit_type, text, heading_path, ordinal, token_count, content_hash,
                status, metadata, created_at, updated_at
             )
             SELECT
                id, text_unit_code, source_code, source_version, document_code, scope_type,
                scope_code, unit_type, text, heading_path, ordinal, token_count, content_hash,
                status, metadata, created_at, updated_at
             FROM rag_project_doc_text_units_index_payload_old;
             DROP TABLE rag_project_doc_text_units_index_payload_old;
             CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_source
                ON rag_project_doc_text_units (source_code, source_version, ordinal);
             CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_scope_status
                ON rag_project_doc_text_units (scope_type, scope_code, status);
             CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_document
                ON rag_project_doc_text_units (document_code);",
        )
        .map_err(storage_error)
}

fn rebuild_all_extension_indexes(repository: &SqliteRagRepository) -> Result<(), RagError> {
    for table in ["rag_memory_text_units", "rag_project_doc_text_units"] {
        if !table_exists(&repository.connection, table)? {
            continue;
        }
        rebuild_table_indexes(&repository.connection, table)?;
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
        let search_text_expr = if column_exists(connection, table_name, "search_text")? {
            "search_text"
        } else {
            "text"
        };
        let fulltext_predicate = if column_exists(connection, table_name, "fulltext_status")? {
            " AND fulltext_status = 'INDEXED'"
        } else {
            ""
        };
        connection
            .execute(
                &format!(
                    "INSERT INTO {fts_table}(rowid, search_text)
                     SELECT id, {search_text_expr}
                     FROM {source_table}
                     WHERE status = 'ACTIVE'{fulltext_predicate}"
                ),
                [],
            )
            .map_err(storage_error)?;
    }

    if sqlite_vec_available(connection) && legacy_vector_columns_exist(connection, table_name)? {
        for dimension in supported_vector_dimensions() {
            let vec_table_name = vec_table_name(table_name, *dimension);
            if !table_exists(connection, &vec_table_name)? {
                continue;
            }
            let vec_table = quote_identifier(&vec_table_name)?;
            connection
                .execute(&format!("DELETE FROM {vec_table}"), [])
                .map_err(storage_error)?;
        }
        insert_eligible_vector_rows(connection, table_name)?;
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

fn insert_eligible_vector_rows(connection: &Connection, table_name: &str) -> Result<(), RagError> {
    let table = quote_identifier(table_name)?;
    let sql = format!(
        "SELECT id, embedding_dimension, embedding
         FROM {table}
         WHERE status = 'ACTIVE' AND embedding_status = 'INDEXED'
           AND embedding_dimension IS NOT NULL AND embedding IS NOT NULL"
    );
    insert_vector_rows_from_query(connection, table_name, &sql, [])
}

fn insert_vector_rows_from_query<P>(
    connection: &Connection,
    table_name: &str,
    sql: &str,
    params: P,
) -> Result<(), RagError>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql).map_err(storage_error)?;
    let rows = statement
        .query_map(params, |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    drop(statement);

    for (rowid, dimension, embedding) in rows {
        if !supported_vector_dimensions().contains(&dimension) {
            continue;
        }
        let Some(values) = decode_embedding(&embedding) else {
            continue;
        };
        if values.len() != dimension as usize {
            continue;
        }
        let vec_table_name = vec_table_name(table_name, dimension);
        if !table_exists(connection, &vec_table_name)? {
            continue;
        }
        let vec_table = quote_identifier(&vec_table_name)?;
        let embedding_json =
            serde_json::to_string(&values).map_err(|_| RagError::StorageFailure)?;
        connection
            .execute(
                &format!("INSERT OR REPLACE INTO {vec_table}(rowid, embedding) VALUES (?1, ?2)"),
                params![rowid, embedding_json],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}
