use super::*;

pub(super) fn exists(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    name: &str,
) -> Result<bool, IngestionQueueError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )
    .map_err(storage)
}

pub(super) fn validate_vectors(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    kb: &str,
    source: &str,
    count: usize,
    dimension: usize,
) -> Result<(), IngestionQueueError> {
    let vector = format!("rag_{kb}_units_vec_d{dimension}");
    if exists(tx, &vector)? {
        let actual: usize = tx.query_row(&format!("SELECT count(*) FROM \"rag_{kb}_units\" u JOIN \"{vector}\" v ON v.rowid=u.id WHERE u.source_code=?1 AND u.source_status='STAGING'"), [source], |r| r.get(0)).map_err(storage)?;
        if actual != count {
            return Err(IngestionQueueError::Conflict);
        }
    } else {
        return Err(IngestionQueueError::Conflict);
    }
    Ok(())
}

/// The product corpus uses one embedded text/vector table. Retire extension
/// rows in the same publication transaction so stale generations cannot fill KNN.
pub(super) fn retire_indexes(
    tx: &crate::adapters::outbound::persistence::sqlite::framework_storage::BusinessTransaction<'_>,
    kb: &str,
    source: &str,
) -> Result<(), IngestionQueueError> {
    if source.is_empty() {
        return Ok(());
    }
    let scopes = {
        let mut statement=tx.prepare(&format!("SELECT DISTINCT scope_type,scope_code,source_version FROM \"rag_{kb}_units\" WHERE source_code=?1")).map_err(storage)?;
        statement
            .query_map([source], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?
    };
    for (scope_type, scope_code, version) in scopes {
        let partition = crate::domain::rag::graph::GraphPartition {
            corpus_id: kb.into(),
            scope_type,
            scope_code,
        };
        super::super::rag_graph_index::retire_graph_source_version(
            tx, &partition, source, &version,
        )
        .map_err(storage)?;
    }
    let mut stmt = tx
        .prepare(&format!(
            "SELECT rowid FROM \"rag_{kb}_units\" WHERE source_code=?1"
        ))
        .map_err(storage)?;
    let ids = stmt
        .query_map([source], |r| r.get::<_, i64>(0))
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    let mut names=crate::adapters::outbound::persistence::sqlite::rag_repository::vector_index_tables_for_cleanup(tx,&format!("rag_{kb}_units")).map_err(storage)?;
    names.push(format!("rag_{kb}_units_fts"));
    for name in names {
        if exists(tx, &name)? {
            for id in &ids {
                tx.execute(&format!("DELETE FROM \"{name}\" WHERE rowid=?1"), [id])
                    .map_err(storage)?;
            }
        }
    }
    tx.execute(
        "DELETE FROM knowledge_chunk_contexts WHERE knowledge_base_id=?1 AND source_code=?2",
        params![kb, source],
    )
    .map_err(storage)?;

    Ok(())
}
