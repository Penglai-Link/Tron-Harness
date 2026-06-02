use rusqlite::{Connection, OptionalExtension, params};

use crate::{domain::rag::RagError, shared::sqlite_extensions::SqliteExtensionCapabilities};

use super::{
    SqliteRagRepository,
    sql_plan::{SqliteRagCorpusPlan, quote_identifier},
    storage_error,
};

const VECTOR_INDEX_DIMENSIONS: &[i64] = &[2, 1536, 2048];

pub(super) fn apply_extension_index_migration(
    repository: &SqliteRagRepository,
) -> Result<(), RagError> {
    repository
        .connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS rag_sqlite_vector_indexes (
                id INTEGER PRIMARY KEY,
                corpus_id TEXT NOT NULL,
                text_unit_table TEXT NOT NULL,
                vec_table TEXT NOT NULL,
                embedding_model_code TEXT,
                embedding_version TEXT,
                dimension INTEGER NOT NULL,
                distance_metric TEXT NOT NULL DEFAULT 'COSINE',
                status TEXT NOT NULL DEFAULT 'ACTIVE',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE (
                    corpus_id, text_unit_table, embedding_model_code,
                    embedding_version, dimension
                )
            );",
        )
        .map_err(storage_error)?;

    for table in ["rag_memory_text_units", "rag_project_doc_text_units"] {
        if !table_exists(&repository.connection, table)? {
            continue;
        }
        ensure_fts_table(&repository.connection, table, &repository.sqlite_extensions)?;
        ensure_vector_tables(&repository.connection, table, &repository.sqlite_extensions)?;
    }
    rebuild_all_extension_indexes(repository)
}

pub(super) fn delete_text_unit_indexes_for_source(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    capabilities: &SqliteExtensionCapabilities,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    let table = quote_identifier(&plan.text_unit_table)?;
    let source_row_sql = format!(
        "SELECT id FROM {table}
         WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4"
    );
    let mut statement = connection.prepare(&source_row_sql).map_err(storage_error)?;
    let rowids = statement
        .query_map(
            params![source_code, source_version, scope_type, scope_code],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    drop(statement);
    delete_text_unit_indexes_for_rowids(connection, plan, capabilities, &rowids)
}

pub(super) fn sync_text_unit_indexes_for_source(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    capabilities: &SqliteExtensionCapabilities,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    delete_text_unit_indexes_for_source(
        connection,
        plan,
        capabilities,
        source_code,
        source_version,
        scope_type,
        scope_code,
    )?;
    insert_eligible_source_indexes(
        connection,
        plan,
        capabilities,
        source_code,
        source_version,
        scope_type,
        scope_code,
    )
}

pub(super) fn sync_text_unit_indexes_for_code(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    capabilities: &SqliteExtensionCapabilities,
    text_unit_code: &str,
) -> Result<(), RagError> {
    let table = quote_identifier(&plan.text_unit_table)?;
    let sql = format!(
        "SELECT source_code, source_version, scope_type, scope_code
         FROM {table}
         WHERE text_unit_code = ?1"
    );
    let source = connection
        .query_row(&sql, params![text_unit_code], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(storage_error)?;
    sync_text_unit_indexes_for_source(
        connection,
        plan,
        capabilities,
        &source.0,
        &source.1,
        &source.2,
        &source.3,
    )
}

fn rebuild_all_extension_indexes(repository: &SqliteRagRepository) -> Result<(), RagError> {
    for table in ["rag_memory_text_units", "rag_project_doc_text_units"] {
        if !table_exists(&repository.connection, table)? {
            continue;
        }
        rebuild_table_indexes(&repository.connection, table, &repository.sqlite_extensions)?;
    }
    Ok(())
}

fn rebuild_table_indexes(
    connection: &Connection,
    table_name: &str,
    capabilities: &SqliteExtensionCapabilities,
) -> Result<(), RagError> {
    if capabilities.extension_fulltext_available() {
        let fts_table = quote_identifier(&fts_table_name(table_name))?;
        connection
            .execute(&format!("DELETE FROM {fts_table}"), [])
            .map_err(storage_error)?;
        let source_table = quote_identifier(table_name)?;
        connection
            .execute(
                &format!(
                    "INSERT INTO {fts_table}(rowid, search_text)
                     SELECT id, search_text
                     FROM {source_table}
                     WHERE status = 'ACTIVE' AND fulltext_status = 'INDEXED'"
                ),
                [],
            )
            .map_err(storage_error)?;
    }

    if capabilities.extension_vector_available() {
        for dimension in VECTOR_INDEX_DIMENSIONS {
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

fn ensure_fts_table(
    connection: &Connection,
    table_name: &str,
    capabilities: &SqliteExtensionCapabilities,
) -> Result<(), RagError> {
    if !capabilities.extension_fulltext_available() {
        return Ok(());
    }
    let tokenizer = if capabilities.simple_tokenizer {
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

fn ensure_vector_tables(
    connection: &Connection,
    table_name: &str,
    capabilities: &SqliteExtensionCapabilities,
) -> Result<(), RagError> {
    if !capabilities.extension_vector_available() {
        return Ok(());
    }
    for dimension in VECTOR_INDEX_DIMENSIONS {
        let vec_table_name = vec_table_name(table_name, *dimension);
        let vec_table = quote_identifier(&vec_table_name)?;
        connection
            .execute_batch(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS {vec_table}
                 USING vec0(embedding float[{dimension}]);"
            ))
            .map_err(storage_error)?;
        connection
            .execute(
                "INSERT INTO rag_sqlite_vector_indexes (
                    corpus_id, text_unit_table, vec_table, dimension,
                    distance_metric, status, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, 'COSINE', 'ACTIVE', ?5, ?5)
                ON CONFLICT (
                    corpus_id, text_unit_table, embedding_model_code,
                    embedding_version, dimension
                )
                DO UPDATE SET
                    vec_table = excluded.vec_table,
                    status = excluded.status,
                    updated_at = excluded.updated_at",
                params![
                    corpus_id_for_table(table_name),
                    table_name,
                    vec_table_name,
                    dimension,
                    now_millis()
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn insert_eligible_source_indexes(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    capabilities: &SqliteExtensionCapabilities,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    if capabilities.extension_fulltext_available() {
        let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
        let table = quote_identifier(&plan.text_unit_table)?;
        connection
            .execute(
                &format!(
                    "INSERT INTO {fts_table}(rowid, search_text)
                     SELECT id, search_text
                     FROM {table}
                     WHERE source_code = ?1 AND source_version = ?2
                       AND scope_type = ?3 AND scope_code = ?4
                       AND status = 'ACTIVE' AND fulltext_status = 'INDEXED'"
                ),
                params![source_code, source_version, scope_type, scope_code],
            )
            .map_err(storage_error)?;
    }

    if capabilities.extension_vector_available() {
        insert_eligible_vector_rows_for_source(
            connection,
            &plan.text_unit_table,
            source_code,
            source_version,
            scope_type,
            scope_code,
        )?;
    }
    Ok(())
}

fn insert_eligible_vector_rows_for_source(
    connection: &Connection,
    table_name: &str,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    let table = quote_identifier(table_name)?;
    let sql = format!(
        "SELECT id, embedding_dimension, embedding
         FROM {table}
         WHERE source_code = ?1 AND source_version = ?2
           AND scope_type = ?3 AND scope_code = ?4
           AND status = 'ACTIVE' AND embedding_status = 'INDEXED'
           AND embedding_dimension IS NOT NULL AND embedding IS NOT NULL"
    );
    insert_vector_rows_from_query(
        connection,
        table_name,
        &sql,
        params![source_code, source_version, scope_type, scope_code],
    )
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
        if !VECTOR_INDEX_DIMENSIONS.contains(&dimension) {
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

fn delete_text_unit_indexes_for_rowids(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    capabilities: &SqliteExtensionCapabilities,
    rowids: &[i64],
) -> Result<(), RagError> {
    if rowids.is_empty() {
        return Ok(());
    }
    if capabilities.extension_fulltext_available() {
        delete_rowids_from_table(connection, &fts_table_name(&plan.text_unit_table), rowids)?;
    }
    if capabilities.extension_vector_available() {
        for dimension in VECTOR_INDEX_DIMENSIONS {
            let table_name = vec_table_name(&plan.text_unit_table, *dimension);
            if table_exists(connection, &table_name)? {
                delete_rowids_from_table(connection, &table_name, rowids)?;
            }
        }
    }
    Ok(())
}

fn delete_rowids_from_table(
    connection: &Connection,
    table_name: &str,
    rowids: &[i64],
) -> Result<(), RagError> {
    let table = quote_identifier(table_name)?;
    for rowid in rowids {
        connection
            .execute(
                &format!("DELETE FROM {table} WHERE rowid = ?1"),
                params![rowid],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

pub(super) fn fts_table_name(text_unit_table: &str) -> String {
    format!("{text_unit_table}_fts")
}

pub(super) fn vec_table_name(text_unit_table: &str, dimension: i64) -> String {
    format!("{text_unit_table}_vec_d{dimension}")
}

pub(super) fn supported_vector_dimensions() -> &'static [i64] {
    VECTOR_INDEX_DIMENSIONS
}

fn corpus_id_for_table(table_name: &str) -> &'static str {
    match table_name {
        "rag_memory_text_units" => "memory-card",
        "rag_project_doc_text_units" => "project-doc",
        _ => "unknown",
    }
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, RagError> {
    connection
        .query_row(
            "SELECT 1
             FROM sqlite_schema
             WHERE type IN ('table', 'view')
               AND name = ?1
             LIMIT 1",
            params![table_name],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(storage_error)
}

fn decode_embedding(raw: &[u8]) -> Option<Vec<f32>> {
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    Some(
        raw.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
