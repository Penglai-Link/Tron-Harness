fn search_tree_route(
    repository: &SqliteRagRepository,
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<Vec<RagSearchRow>, RagError> {
    validate_retriever_available(repository, plan, query, retriever)?;
    let policy = TreeRoutePolicy::from_query(query);
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
                 JOIN {text_table} tu
                   ON tu.text_unit_code = covered.value
                  AND tu.source_code = tn.source_code
                  AND tu.source_version = tn.source_version
                  AND tu.scope_type = tn.scope_type
                  AND tu.scope_code = tn.scope_code"
            )
        }
        SqliteRetrieverKind::TreeVector => {
            let dimension = query_embedding_dimension(query)?;
            let vec_table = quote_identifier(&vec_table_name(tree_table_name, dimension))?;
            format!(
                "FROM {vec_table} vec
                 JOIN {tree_table} tn ON tn.id = vec.rowid
                 JOIN json_each(tn.covered_text_unit_codes) covered
                 JOIN {text_table} tu
                   ON tu.text_unit_code = covered.value
                  AND tu.source_code = tn.source_code
                  AND tu.source_version = tn.source_version
                  AND tu.scope_type = tn.scope_type
                  AND tu.scope_code = tn.scope_code"
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
        "tn.source_version = ?".to_string(),
        "tn.status = 'ACTIVE'".to_string(),
        format!(
            "tu.{} = 'ACTIVE'",
            quote_identifier(&plan.text_unit_status_column)?
        ),
    ];
    match retriever.kind {
        SqliteRetrieverKind::TreeFulltext => {
            clauses.push("tn.summary_fulltext_status = 'INDEXED'".to_string());
        }
        SqliteRetrieverKind::TreeVector => {
            clauses.push("tn.summary_embedding_status = 'INDEXED'".to_string());
        }
        _ => return Err(RagError::SearchRouteUnavailable),
    }
    let mut params = vec![
        SqlValue::Text(query.source_version.clone()),
        SqlValue::Text(query.source_version.clone()),
    ];
    if !policy.include_root {
        clauses.push("tn.parent_node_code != '0'".to_string());
    }
    if let Some(max_covered_units) = policy.max_covered_units_per_node {
        clauses.push("CAST(covered.key AS INTEGER) < ?".to_string());
        params.push(SqlValue::Integer(max_covered_units as i64));
    }
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
                .map(|_| {
                    "(tu.scope_type = ? AND tu.scope_code = ? AND tn.scope_type = ? AND tn.scope_code = ?)"
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(" OR ")
        ));
        for scope in &query.scopes {
            params.push(SqlValue::Text(scope.scope_type.clone()));
            params.push(SqlValue::Text(scope.scope_code.clone()));
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
            let tree_order = tree_order_sql(&policy);
            format!("ORDER BY -bm25({fts_table}) DESC, {tree_order}, tu.ordinal")
        }
        SqliteRetrieverKind::TreeVector => {
            let tree_order = tree_order_sql(&policy);
            format!("ORDER BY vec_distance_cosine(vec.embedding, ?) ASC, {tree_order}, tu.ordinal")
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

#[derive(Debug, Clone, Copy)]
struct TreeRoutePolicy {
    include_root: bool,
    max_covered_units_per_node: Option<usize>,
    prefer_deeper_tree_node: bool,
}

impl TreeRoutePolicy {
    fn from_query(query: &RagRouteSearchQuery) -> Self {
        let policy = query
            .policy
            .get("treePolicy")
            .or_else(|| query.policy.get("policy"))
            .or_else(|| query.policy.get("routePolicy"))
            .unwrap_or(&query.policy);
        Self {
            include_root: policy_bool(policy, "includeRoot", false),
            max_covered_units_per_node: policy_usize(policy, "maxCoveredUnitsPerNode"),
            prefer_deeper_tree_node: policy_bool(policy, "preferDeeperTreeNode", false),
        }
    }
}

fn tree_order_sql(policy: &TreeRoutePolicy) -> &'static str {
    if policy.prefer_deeper_tree_node {
        "tn.level DESC, tn.ordinal"
    } else {
        "tn.ordinal"
    }
}

fn policy_bool(policy: &Value, field: &str, default_value: bool) -> bool {
    policy
        .get(field)
        .and_then(Value::as_bool)
        .unwrap_or(default_value)
}

fn policy_usize(policy: &Value, field: &str) -> Option<usize> {
    policy
        .get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
}
