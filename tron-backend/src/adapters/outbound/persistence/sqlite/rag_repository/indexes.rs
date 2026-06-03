use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::rag::RagTextUnitWrite, domain::rag::RagError,
    shared::sqlite_extensions::sqlite_vec_available,
};

use super::{
    SqliteRagRepository,
    sql_plan::{SqliteRagCorpusPlan, quote_identifier},
    storage_error,
};

const VECTOR_INDEX_DIMENSIONS: &[i64] = &[4096];

pub(super) fn apply_extension_index_migration(
    repository: &SqliteRagRepository,
) -> Result<(), RagError> {
    super::index_schema::apply_extension_index_migration(repository)
}

pub(super) fn delete_text_unit_indexes_for_source(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
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
    delete_text_unit_indexes_for_rowids(connection, plan, &rowids)
}

pub(super) fn sync_text_unit_indexes_for_units(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    units: &[RagTextUnitWrite],
) -> Result<(), RagError> {
    let table = quote_identifier(&plan.text_unit_table)?;
    let sql = format!("SELECT id FROM {table} WHERE text_unit_code = ?1");
    for unit in units {
        let rowid = connection
            .query_row(&sql, params![unit.text_unit_code], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(storage_error)?;
        upsert_fulltext_row(connection, plan, rowid, &unit.search_text)?;
        if let (Some(dimension), Some(embedding)) =
            (unit.embedding_dimension, unit.embedding.as_ref())
        {
            upsert_vector_row(
                connection,
                &plan.text_unit_table,
                rowid,
                dimension,
                embedding,
            )?;
        }
    }
    Ok(())
}

pub(super) fn rebuild_fulltext_indexes_for_source(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    source_code: &str,
    source_version: &str,
    scope_type: &str,
    scope_code: &str,
) -> Result<(), RagError> {
    let fts_table_name = fts_table_name(&plan.text_unit_table);
    if table_exists(connection, &fts_table_name)? {
        let fts_table = quote_identifier(&fts_table_name)?;
        let table = quote_identifier(&plan.text_unit_table)?;
        connection
            .execute(
                &format!(
                    "DELETE FROM {fts_table}
                     WHERE rowid IN (
                        SELECT id
                        FROM {table}
                        WHERE source_code = ?1 AND source_version = ?2
                          AND scope_type = ?3 AND scope_code = ?4
                     )"
                ),
                params![source_code, source_version, scope_type, scope_code],
            )
            .map_err(storage_error)?;
        connection
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO {fts_table}(rowid, search_text)
                     SELECT id, text
                     FROM {table}
                     WHERE source_code = ?1 AND source_version = ?2
                       AND scope_type = ?3 AND scope_code = ?4
                       AND status = 'ACTIVE'"
                ),
                params![source_code, source_version, scope_type, scope_code],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

pub(super) fn upsert_vector_index_for_code(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    text_unit_code: &str,
    dimension: i64,
    embedding: &[u8],
) -> Result<(), RagError> {
    let table = quote_identifier(&plan.text_unit_table)?;
    let sql = format!("SELECT id FROM {table} WHERE text_unit_code = ?1");
    let rowid = connection
        .query_row(&sql, params![text_unit_code], |row| row.get::<_, i64>(0))
        .map_err(storage_error)?;
    upsert_vector_row(
        connection,
        &plan.text_unit_table,
        rowid,
        dimension,
        embedding,
    )
}

fn upsert_fulltext_row(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    rowid: i64,
    search_text: &str,
) -> Result<(), RagError> {
    let fts_table_name = fts_table_name(&plan.text_unit_table);
    if !table_exists(connection, &fts_table_name)? {
        return Ok(());
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
    if !VECTOR_INDEX_DIMENSIONS.contains(&dimension) {
        return Ok(());
    }
    let Some(values) = decode_embedding(embedding) else {
        return Ok(());
    };
    if values.len() != dimension as usize {
        return Ok(());
    }
    let vec_table_name = vec_table_name(table_name, dimension);
    if !table_exists(connection, &vec_table_name)? {
        return Ok(());
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

fn delete_text_unit_indexes_for_rowids(
    connection: &Connection,
    plan: &SqliteRagCorpusPlan,
    rowids: &[i64],
) -> Result<(), RagError> {
    if rowids.is_empty() {
        return Ok(());
    }
    let fts_table_name = fts_table_name(&plan.text_unit_table);
    if table_exists(connection, &fts_table_name)? {
        delete_rowids_from_table(connection, &fts_table_name, rowids)?;
    }
    if sqlite_vec_available(connection) {
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
