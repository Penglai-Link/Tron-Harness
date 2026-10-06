use super::*;

fn exists(tx: &Transaction<'_>, name: &str) -> Result<bool, IngestionQueueError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )
    .map_err(storage)
}

pub(super) fn validate_vectors(
    tx: &Transaction<'_>,
    kb: &str,
    source: &str,
    count: usize,
) -> Result<(), IngestionQueueError> {
    let vector = format!("rag_{kb}_units_vec_d1024");
    if exists(tx, &vector)? {
        let actual: usize = tx.query_row(&format!("SELECT count(*) FROM \"rag_{kb}_units\" u JOIN \"{vector}\" v ON v.rowid=u.id WHERE u.source_code=?1 AND u.source_status='STAGING'"), [source], |r| r.get(0)).map_err(storage)?;
        if actual != count {
            return Err(IngestionQueueError::Conflict);
        }
    }
    Ok(())
}

/// The product corpus uses one embedded text/vector table. Retire extension
/// rows in the same publication transaction so stale generations cannot fill KNN.
pub(super) fn retire_indexes(
    tx: &Transaction<'_>,
    kb: &str,
    source: &str,
) -> Result<(), IngestionQueueError> {
    if source.is_empty() {
        return Ok(());
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
    for name in [
        format!("rag_{kb}_units_fts"),
        format!("rag_{kb}_units_vec_d1024"),
    ] {
        if exists(tx, &name)? {
            for id in &ids {
                tx.execute(&format!("DELETE FROM \"{name}\" WHERE rowid=?1"), [id])
                    .map_err(storage)?;
            }
        }
    }
    Ok(())
}
