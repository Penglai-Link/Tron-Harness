fn text_unit_unique_key(
    repository: &SqliteRagRepository,
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
    repository: &SqliteRagRepository,
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
    repository: &SqliteRagRepository,
    table_name: &str,
    columns: &BTreeSet<String>,
) -> Result<Vec<String>, RagError> {
    ensure_required_columns(columns, &["tree_node_code"])?;
    preferred_unique_key(
        repository,
        table_name,
        &[
            vec!["tree_node_code".to_string()],
            vec![
                "source_code".to_string(),
                "source_version".to_string(),
                "scope_type".to_string(),
                "scope_code".to_string(),
                "path_key".to_string(),
            ],
        ],
    )
}

fn preferred_unique_key(
    repository: &SqliteRagRepository,
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
    repository: &SqliteRagRepository,
    table_name: &str,
) -> Result<Vec<Vec<String>>, RagError> {
    let pragma = format!("PRAGMA index_list({})", quote_identifier(table_name)?);
    let mut statement = repository
        .connection
        .prepare(&pragma)
        .map_err(storage_error)?;
    let index_names = statement
        .query_map([], |row| {
            let unique = row.get::<_, i64>(2)? == 1;
            let name = row.get::<_, String>(1)?;
            Ok(unique.then_some(name))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let mut indexes = Vec::new();
    for index_name in index_names {
        let pragma = format!("PRAGMA index_info({})", quote_identifier(&index_name)?);
        let mut statement = repository
            .connection
            .prepare(&pragma)
            .map_err(storage_error)?;
        let columns = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(2)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let mut columns = columns;
        columns.sort_by_key(|(sequence, _)| *sequence);
        indexes.push(
            columns
                .into_iter()
                .map(|(_, column)| column)
                .collect::<Vec<_>>(),
        );
    }
    Ok(indexes)
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
