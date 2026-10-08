use self::{indexes::fts_table_name, sql_plan::quote_identifier};

/// Join the caller's transaction on the same host connection.
pub(super) fn delete_managed_corpus_on_connection(
    connection: &Connection,
    corpus_id: &str,
) -> Result<bool, RagError> {
    SqliteRagRepositorySql { connection }.delete_managed_corpus(corpus_id)
}

impl<'a> SqliteRagRepositorySql<'a> {
    fn unshared_sqlite_runtime_tables(
        &self,
        corpus_id: &str,
        record: &RagCorpusRecord,
    ) -> Result<BTreeSet<String>, RagError> {
        let candidate_tables = sqlite_runtime_base_tables(&record.storage_binding);
        let mut shared_tables = BTreeSet::new();
        for corpus in self.list_corpora()? {
            if corpus.corpus_id == corpus_id {
                continue;
            }
            shared_tables.extend(sqlite_runtime_base_tables(&corpus.storage_binding));
        }
        Ok(candidate_tables
            .difference(&shared_tables)
            .cloned()
            .collect())
    }
}

fn sqlite_runtime_base_tables(binding: &RagStorageBinding) -> BTreeSet<String> {
    binding
        .segment_targets
        .values()
        .filter(|target| target.backend == RagStorageBackend::Sqlite)
        .filter_map(|target| target.table.clone())
        .collect()
}

fn drop_sqlite_runtime_table_family(
    connection: &Connection,
    table_name: &str,
) -> Result<(), RagError> {
    let fts_table = fts_table_name(table_name);
    let mut candidates = vec![fts_table];
    candidates.extend(indexes::vector_index_tables(connection, table_name)?);
    candidates.push(table_name.to_string());

    for candidate in candidates {
        let table = quote_identifier(&candidate)?;
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .map_err(storage_error)?;
    }
    Ok(())
}
