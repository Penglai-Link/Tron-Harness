use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::rag::RagTextUnitWrite,
    domain::rag::RagError,
    shared::sqlite_extensions::{TRON_VECTOR_INDEX_DIMENSION, sqlite_vec_available},
};

use super::{
    sql_plan::{SqliteRagCorpusPlan, quote_identifier},
    storage_error,
};

const VECTOR_INDEX_DIMENSIONS: &[i64] = &[TRON_VECTOR_INDEX_DIMENSION as i64];

pub(super) fn delete_text_unit_indexes_for_source(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    if plan.has_sqlite_fulltext() {
        let fulltext_rowids = source_rowids(
            connection,
            plan.fulltext_storage_table(),
            source_code,
            source_version,
            scope_type,
            scope_code,
        )?;
        delete_fulltext_indexes_for_rowids(connection, plan, &fulltext_rowids)?;
        if plan.has_distinct_fulltext_table() || !plan.has_sqlite_text_units() {
            delete_rows_for_source(
                connection,
                plan.fulltext_storage_table(),
                source_code,
                source_version,
                scope_type,
                scope_code,
            )?;
        }
    }

    if plan.has_sqlite_vector() {
        if plan.has_distinct_vector_table() {
            let vector_rowids = source_rowids(
                connection,
                plan.vector_storage_table(),
                source_code,
                source_version,
                scope_type,
                scope_code,
            )?;
            delete_vector_indexes_for_rowids(connection, plan, &vector_rowids)?;
            delete_rows_for_source(
                connection,
                plan.vector_storage_table(),
                source_code,
                source_version,
                scope_type,
                scope_code,
            )?;
        } else {
            let text_rowids = source_rowids(
                connection,
                &plan.text_unit_table,
                source_code,
                source_version,
                scope_type,
                scope_code,
            )?;
            delete_vector_indexes_for_rowids(connection, plan, &text_rowids)?;
            if !plan.has_sqlite_text_units() {
                delete_rows_for_source(
                    connection,
                    plan.vector_storage_table(),
                    source_code,
                    source_version,
                    scope_type,
                    scope_code,
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn sync_text_unit_indexes_for_units(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    units: &[RagTextUnitWrite],
) -> Result<(), RagError> {
    for unit in units {
        let text_rowid =
            rowid_for_text_unit_code(connection, &plan.text_unit_table, &unit.text_unit_code)?;
        if plan.has_sqlite_fulltext() && unit.fulltext_status == "INDEXED" {
            let fulltext_rowid = if plan.has_distinct_fulltext_table() {
                rowid_for_text_unit_code(
                    connection,
                    plan.fulltext_storage_table(),
                    &unit.text_unit_code,
                )?
            } else {
                text_rowid
            };
            upsert_fulltext_row(connection, plan, fulltext_rowid, &unit.search_text)?;
        }
        if let (Some(dimension), Some(embedding)) =
            (unit.embedding_dimension, unit.embedding.as_ref())
        {
            if !plan.has_sqlite_vector() {
                continue;
            }
            let vector_rowid = if plan.has_distinct_vector_table() {
                rowid_for_text_unit_code(
                    connection,
                    plan.vector_storage_table(),
                    &unit.text_unit_code,
                )?
            } else {
                text_rowid
            };
            upsert_vector_row(
                connection,
                plan.vector_storage_table(),
                vector_rowid,
                dimension,
                embedding,
            )?;
        }
    }
    Ok(())
}

fn upsert_fulltext_row(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    rowid: i64,
    search_text: &str,
) -> Result<(), RagError> {
    if !plan.has_sqlite_fulltext() {
        return Ok(());
    }
    let fts_table_name = fts_table_name(plan.fulltext_storage_table());
    if !table_exists(connection, &fts_table_name)? {
        return Err(RagError::StorageFailure);
    }
    let fts_table = quote_identifier(&fts_table_name)?;
    connection
        .execute(
            &format!("INSERT OR REPLACE INTO {fts_table}(rowid, search_text) VALUES (?1, ?2)"),
            params![rowid, search_text],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn upsert_vector_row(
    connection: &Connection,
    table_name: &str,
    rowid: i64,
    dimension: i64,
    embedding: &[u8],
) -> Result<(), RagError> {
    if !(1..=8192).contains(&dimension) {
        return Err(RagError::EmbeddingFailed);
    }
    let values = decode_embedding(embedding).ok_or(RagError::EmbeddingFailed)?;
    if values.len() != dimension as usize || values.iter().any(|value| !value.is_finite()) {
        return Err(RagError::EmbeddingFailed);
    }
    let vec_table_name = vec_table_name(table_name, dimension);
    if !table_exists(connection, &vec_table_name)? {
        connection.execute_batch(&format!("CREATE VIRTUAL TABLE {} USING vec0(embedding float[{dimension}] distance_metric=cosine)",quote_identifier(&vec_table_name)?)).map_err(storage_error)?;
    }
    let vec_table = quote_identifier(&vec_table_name)?;
    let embedding_json = serde_json::to_string(&values).map_err(|_| RagError::StorageFailure)?;
    connection
        .execute(
            &format!("INSERT OR REPLACE INTO {vec_table}(rowid, embedding) VALUES (?1, ?2)"),
            params![rowid, embedding_json],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn delete_fulltext_indexes_for_rowids(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    rowids: &[i64],
) -> Result<(), RagError> {
    if !plan.has_sqlite_fulltext() || rowids.is_empty() {
        return Ok(());
    }
    let fts_table_name = fts_table_name(plan.fulltext_storage_table());
    if table_exists(connection, &fts_table_name)? {
        delete_rowids_from_table(connection, &fts_table_name, rowids)?;
    }
    Ok(())
}

fn delete_vector_indexes_for_rowids(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    rowids: &[i64],
) -> Result<(), RagError> {
    if !plan.has_sqlite_vector() || !sqlite_vec_available(connection) || rowids.is_empty() {
        return Ok(());
    }
    for table_name in vector_index_tables(connection, plan.vector_storage_table())? {
        if table_exists(connection, &table_name)? {
            delete_rowids_from_table(connection, &table_name, rowids)?;
        }
    }
    Ok(())
}

fn source_rowids(
    connection: &Connection,
    table_name: &str,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<Vec<i64>, RagError> {
    let table = quote_identifier(table_name)?;
    let source_row_sql = format!(
        "SELECT id FROM {table}
         WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4"
    );
    let mut statement = connection.prepare(&source_row_sql).map_err(storage_error)?;
    statement
        .query_map(
            params![source_code, source_version, scope_type, scope_code],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn delete_rows_for_source(
    connection: &Connection,
    table_name: &str,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    let table = quote_identifier(table_name)?;
    connection
        .execute(
            &format!(
                "DELETE FROM {table}
                 WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4"
            ),
            params![source_code, source_version, scope_type, scope_code],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn rowid_for_text_unit_code(
    connection: &Connection,
    table_name: &str,
    text_unit_code: &str,
) -> Result<i64, RagError> {
    let table = quote_identifier(table_name)?;
    let sql = format!("SELECT id FROM {table} WHERE text_unit_code = ?1");
    connection
        .query_row(&sql, params![text_unit_code], |row| row.get::<_, i64>(0))
        .map_err(storage_error)
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

pub(super) fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, RagError> {
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

pub(super) fn decode_embedding(raw: &[u8]) -> Option<Vec<f32>> {
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    Some(
        raw.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

pub(super) fn vector_index_tables(
    connection: &Connection,
    table: &str,
) -> Result<Vec<String>, RagError> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE ?1")
        .map_err(storage_error)?;
    let prefix = format!("{table}_vec_d");
    let tables = statement
        .query_map([format!("{prefix}%")], |row| row.get::<_, String>(0))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(tables
        .into_iter()
        .filter(|name| {
            name.strip_prefix(&prefix)
                .is_some_and(|suffix| suffix.parse::<usize>().is_ok())
        })
        .collect())
}
