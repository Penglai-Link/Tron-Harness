fn text_unit_unique_key(
    repository: &SqliteRagRepositorySql<'_>,
    table_name: &str,
) -> Result<Vec<String>, RagError> {
    preferred_unique_key(
        repository,
        table_name,
        &[
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "scope_type".to_string(),
                "scope_code".to_string(),
                "ordinal".to_string(),
            ],
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "ordinal".to_string(),
            ],
        ],
    )
}

fn vector_unique_key(
    repository: &SqliteRagRepositorySql<'_>,
    table_name: &str,
    columns: &BTreeSet<String>,
) -> Result<Vec<String>, RagError> {
    ensure_required_columns(columns, &["text_unit_code"])?;
    preferred_unique_key(
        repository,
        table_name,
        &[
            vec!["text_unit_code".to_string()],
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "scope_type".to_string(),
                "scope_code".to_string(),
                "ordinal".to_string(),
            ],
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "ordinal".to_string(),
            ],
        ],
    )
}

fn tree_unique_key(
    repository: &SqliteRagRepositorySql<'_>,
    table_name: &str,
    columns: &BTreeSet<String>,
) -> Result<Vec<String>, RagError> {
    ensure_required_columns(columns, &["tree_node_code"])?;
    preferred_unique_key(
        repository,
        table_name,
        &[
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "scope_type".to_string(),
                "scope_code".to_string(),
                "tree_node_code".to_string(),
            ],
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "scope_type".to_string(),
                "scope_code".to_string(),
                "path_key".to_string(),
            ],
            vec!["tree_node_code".to_string()],
        ],
    )
}

fn preferred_unique_key(
    repository: &SqliteRagRepositorySql<'_>,
    table_name: &str,
    candidates: &[Vec<String>],
) -> Result<Vec<String>, RagError> {
    let unique_indexes = unique_indexes(repository, table_name)?;
    for candidate in candidates {
        if unique_indexes.contains(candidate) {
            return Ok(candidate.clone());
        }
    }
    Err(RagError::StorageBindingInvalid)
}

fn unique_indexes(
    repository: &SqliteRagRepositorySql<'_>,
    table_name: &str,
) -> Result<Vec<Vec<String>>, RagError> {
    let mut statement = repository.connection.prepare(
        "SELECT indexes.name,columns.name FROM pragma_index_list(?1) indexes JOIN pragma_index_info(indexes.name) columns WHERE indexes.\"unique\"=1 ORDER BY indexes.seq,columns.seqno"
    ).map_err(storage_error)?;
    let rows = statement
        .query_map([table_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?;
    let mut indexes = std::collections::BTreeMap::<String, Vec<String>>::new();
    for row in rows {
        let (name, column) = row.map_err(storage_error)?;
        indexes.entry(name).or_default().push(column);
    }
    Ok(indexes.into_values().collect())
}

fn is_return_field_reserved(logical_name: &str) -> bool {
    matches!(
        logical_name,
        "sourceCode"
            | "sourceVersion"
            | "title"
            | "score"
            | "text"
            | "metadata"
            | "scopeType"
            | "scopeCode"
            | "targetType"
            | "targetCode"
            | "routeIds"
            | "preview"
            | "status"
            | "sourceStatus"
            | "sourceMetadata"
            | "searchText"
            | "embedding"
    )
}
