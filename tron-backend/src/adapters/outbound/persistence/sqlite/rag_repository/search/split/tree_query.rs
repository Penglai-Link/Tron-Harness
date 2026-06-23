fn search_tree_route(
    repository: &SqliteRagRepository,
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<Vec<RagSearchRow>, RagError> {
    validate_retriever_available(repository, plan, query, retriever)?;
    let text_table = quote_identifier(&plan.text_unit_table)?;
    let tree_table_name = plan.tree_storage_table()?;
    let tree_table = quote_identifier(tree_table_name)?;
    let from_sql = match retriever.kind {
        SqliteRetrieverKind::TreeFulltext => {
            let fts_table = quote_identifier(&fts_table_name(tree_table_name))?;
            format!(
                "FROM {fts_table}
                 JOIN {tree_table} tn ON tn.id = {fts_table}.rowid
                 JOIN json_each(tn.covered_text_unit_codes) covered
                 JOIN {text_table} tu ON tu.text_unit_code = covered.value"
            )
        }
        SqliteRetrieverKind::TreeVector => {
            let dimension = query_embedding_dimension(query)?;
            let vec_table = quote_identifier(&vec_table_name(tree_table_name, dimension))?;
            format!(
                "FROM {vec_table} vec
                 JOIN {tree_table} tn ON tn.id = vec.rowid
                 JOIN json_each(tn.covered_text_unit_codes) covered
                 JOIN {text_table} tu ON tu.text_unit_code = covered.value"
            )
        }
        _ => return Err(RagError::SearchRouteUnavailable),
    };
    let from_sql = if let Some(source_table) = plan.source_table.as_deref() {
        let source_table = quote_identifier(source_table)?;
        format!(
            "{from_sql}
             JOIN {source_table} src
               ON src.source_code = tu.source_code
              AND src.source_version = tu.source_version
              AND src.scope_type = tu.scope_type
              AND src.scope_code = tu.scope_code"
        )
    } else {
        from_sql
    };
    let mut clauses = vec![
        "tu.source_version = ?".to_string(),
        "tn.status = 'ACTIVE'".to_string(),
        "tn.summary_fulltext_status = 'INDEXED'".to_string(),
        format!(
            "tu.{} = 'ACTIVE'",
            quote_identifier(&plan.text_unit_status_column)?
        ),
    ];
    let mut params = vec![SqlValue::Text(query.source_version.clone())];
    if plan.source_table.is_some() {
        clauses.push(format!(
            "src.{} = 'ACTIVE'",
            quote_identifier(&plan.source_status_column)?
        ));
    } else {
        clauses.push(format!(
            "tu.{} = 'ACTIVE'",
            quote_identifier(&plan.source_status_column)?
        ));
    }
    match retriever.kind {
        SqliteRetrieverKind::TreeFulltext => {
            let fts_query = fts_query(&query.query);
            if !fts_query.is_empty() {
                let fts_table = quote_identifier(&fts_table_name(tree_table_name))?;
                clauses.push(format!("{fts_table} MATCH ?"));
                params.push(SqlValue::Text(fts_query));
            }
        }
        SqliteRetrieverKind::TreeVector => {
            query_embedding_dimension(query)?;
        }
        _ => {}
    }
    if !query.scopes.is_empty() {
        clauses.push(format!(
            "({})",
            query
                .scopes
                .iter()
                .map(|_| "(tu.scope_type = ? AND tu.scope_code = ?)".to_string())
                .collect::<Vec<_>>()
                .join(" OR ")
        ));
        for scope in &query.scopes {
            params.push(SqlValue::Text(scope.scope_type.clone()));
            params.push(SqlValue::Text(scope.scope_code.clone()));
        }
    }
    append_filter_clauses(plan, &query.filters, &mut clauses, &mut params)?;
    let score_expr = match retriever.kind {
        SqliteRetrieverKind::TreeFulltext => {
            let fts_table = quote_identifier(&fts_table_name(tree_table_name))?;
            format!("-bm25({fts_table})")
        }
        SqliteRetrieverKind::TreeVector => "1.0 - vec_distance_cosine(vec.embedding, ?)".into(),
        _ => return Err(RagError::SearchRouteUnavailable),
    };
    let order_sql = match retriever.kind {
        SqliteRetrieverKind::TreeFulltext => {
            let fts_table = quote_identifier(&fts_table_name(tree_table_name))?;
            format!("ORDER BY -bm25({fts_table}) DESC, tn.ordinal, tu.ordinal")
        }
        SqliteRetrieverKind::TreeVector => {
            "ORDER BY vec_distance_cosine(vec.embedding, ?) ASC, tn.ordinal, tu.ordinal".into()
        }
        _ => return Err(RagError::SearchRouteUnavailable),
    };
    let mut select_params = Vec::new();
    if retriever.kind == SqliteRetrieverKind::TreeVector {
        select_params.push(SqlValue::Text(query_embedding_json(query)?));
    }
    let mut all_params = select_params;
    all_params.extend(params);
    if retriever.kind == SqliteRetrieverKind::TreeVector {
        all_params.push(SqlValue::Text(query_embedding_json(query)?));
    }
    all_params.push(SqlValue::Integer(query.limit.max(1) as i64));
    let sql = format!(
        "SELECT
            tu.text_unit_code,
            tu.source_code,
            tu.source_version,
            tu.scope_type,
            tu.scope_code,
            tu.text,
            tn.summary_text,
            tu.heading_path,
            tu.ordinal,
            tu.token_count,
            {embedding},
            {title},
            {source_metadata},
            {text_metadata},
            tn.tree_node_code,
            tn.title,
            {score_expr}
         {from_sql}
         WHERE {}
         {order_sql}
         LIMIT ?",
        clauses.join(" AND "),
        embedding = embedding_expr(plan, retriever)?,
        title = optional_source_expr(plan, "title", "NULL")?,
        source_metadata = source_metadata_expr(plan)?,
        text_metadata = text_metadata_expr(plan)?,
    );
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    let rows = statement
        .query_map(params_from_iter(all_params), |row| {
            map_tree_row(row, plan, retriever)
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(dedup_tree_rows(rows))
}

fn dedup_tree_rows(rows: Vec<RagSearchRow>) -> Vec<RagSearchRow> {
    let mut seen = std::collections::BTreeSet::new();
    rows.into_iter()
        .filter(|row| seen.insert(row.target_code.clone()))
        .collect()
}
