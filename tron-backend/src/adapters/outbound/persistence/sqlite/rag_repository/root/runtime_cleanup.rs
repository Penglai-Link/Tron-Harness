use self::{
    indexes::{fts_table_name, supported_vector_dimensions, vec_table_name},
    sql_plan::quote_identifier,
};

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
    let mut candidates = vec![fts_table.clone()];
    for dimension in supported_vector_dimensions() {
        candidates.push(vec_table_name(table_name, *dimension));
    }
    candidates.push(table_name.to_string());
    candidates.extend([
        format!("{fts_table}_config"),
        format!("{fts_table}_content"),
        format!("{fts_table}_data"),
        format!("{fts_table}_docsize"),
        format!("{fts_table}_idx"),
    ]);
    for dimension in supported_vector_dimensions() {
        let vector_table = vec_table_name(table_name, *dimension);
        candidates.extend([
            format!("{vector_table}_chunks"),
            format!("{vector_table}_info"),
            format!("{vector_table}_rowids"),
            format!("{vector_table}_vector_chunks00"),
        ]);
    }

    for candidate in candidates {
        let table = quote_identifier(&candidate)?;
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .map_err(storage_error)?;
    }
    Ok(())
}
