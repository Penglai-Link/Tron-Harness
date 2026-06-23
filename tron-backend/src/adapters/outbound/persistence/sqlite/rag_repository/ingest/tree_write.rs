fn persist_tree_nodes(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    if !plan.has_sqlite_tree() {
        return Ok(());
    }
    delete_tree_rows_for_source(tx, plan, command)?;
    let now = now_millis();
    for node in &command.tree_nodes {
        persist_tree_node(tx, plan, node, now)?;
        sync_tree_indexes(tx, plan, node)?;
    }
    Ok(())
}

fn delete_tree_rows_for_source(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let table_name = plan.tree_storage_table()?;
    let rowids = tree_rowids_for_source(tx, table_name, command)?;
    let fts_table = fts_table_name(table_name);
    if table_exists(tx, &fts_table)? {
        delete_rowids(tx, &fts_table, &rowids)?;
    }
    let vec_table = vec_table_name(table_name, 4096);
    if table_exists(tx, &vec_table)? {
        delete_rowids(tx, &vec_table, &rowids)?;
    }
    let table = quote_identifier(table_name)?;
    tx.execute(
        &format!(
            "DELETE FROM {table}
             WHERE source_code = ?1
               AND source_version = ?2
               AND scope_type = ?3
               AND scope_code = ?4"
        ),
        params![
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            command.scope.scope_code,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn tree_rowids_for_source(
    tx: &Transaction<'_>,
    table_name: &str,
    command: &RagPersistIngestCommand,
) -> Result<Vec<i64>, RagError> {
    let table = quote_identifier(table_name)?;
    let mut statement = tx
        .prepare(&format!(
            "SELECT id FROM {table}
             WHERE source_code = ?1
               AND source_version = ?2
               AND scope_type = ?3
               AND scope_code = ?4"
        ))
        .map_err(storage_error)?;
    statement
        .query_map(
            params![
                command.source_code,
                command.source_version,
                command.scope.scope_type,
                command.scope.scope_code,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn delete_rowids(
    tx: &Transaction<'_>,
    table_name: &str,
    rowids: &[i64],
) -> Result<(), RagError> {
    if rowids.is_empty() {
        return Ok(());
    }
    let table = quote_identifier(table_name)?;
    for rowid in rowids {
        tx.execute(&format!("DELETE FROM {table} WHERE rowid = ?1"), params![rowid])
            .map_err(storage_error)?;
    }
    Ok(())
}

fn persist_tree_node(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    node: &RagTreeNodeWrite,
    now: i64,
) -> Result<(), RagError> {
    let mut values = Vec::new();
    push_text(&mut values, &plan.tree_columns, "tree_node_code", &node.tree_node_code);
    push_text(
        &mut values,
        &plan.tree_columns,
        "parent_node_code",
        &node.parent_node_code,
    );
    push_text(&mut values, &plan.tree_columns, "source_code", &node.source_code);
    push_text(
        &mut values,
        &plan.tree_columns,
        "source_version",
        &node.source_version,
    );
    push_text(&mut values, &plan.tree_columns, "scope_type", &node.scope_type);
    push_text(&mut values, &plan.tree_columns, "scope_code", &node.scope_code);
    push_text(&mut values, &plan.tree_columns, "path_key", &node.path_key);
    push_integer(&mut values, &plan.tree_columns, "level", node.level);
    push_integer(&mut values, &plan.tree_columns, "ordinal", node.ordinal);
    push_text(&mut values, &plan.tree_columns, "title", &node.title);
    push_text(&mut values, &plan.tree_columns, "summary_text", &node.summary_text);
    push_integer(
        &mut values,
        &plan.tree_columns,
        "summary_token_count",
        node.summary_token_count,
    );
    push_text(
        &mut values,
        &plan.tree_columns,
        "summary_content_hash",
        &node.summary_content_hash,
    );
    push_text(
        &mut values,
        &plan.tree_columns,
        "summary_fulltext_status",
        &node.summary_fulltext_status,
    );
    push_text(
        &mut values,
        &plan.tree_columns,
        "summary_embedding_status",
        &node.summary_embedding_status,
    );
    push_optional_text(
        &mut values,
        &plan.tree_columns,
        "summary_embedding_model_code",
        node.summary_embedding_model_code.as_deref(),
    );
    push_optional_text(
        &mut values,
        &plan.tree_columns,
        "summary_embedding_version",
        node.summary_embedding_version.as_deref(),
    );
    push_optional_text(
        &mut values,
        &plan.tree_columns,
        "summary_embedding_text_hash",
        node.summary_embedding_text_hash.as_deref(),
    );
    if let Some(dimension) = node.summary_embedding_dimension {
        push_integer(
            &mut values,
            &plan.tree_columns,
            "summary_embedding_dimension",
            dimension,
        );
    }
    push_optional_blob(
        &mut values,
        &plan.tree_columns,
        "summary_embedding",
        node.summary_embedding.as_deref(),
    );
    push_text(
        &mut values,
        &plan.tree_columns,
        "covered_text_unit_codes",
        &serde_json::to_string(&node.covered_text_unit_codes)
            .map_err(|_| RagError::StorageFailure)?,
    );
    push_integer(
        &mut values,
        &plan.tree_columns,
        "covered_text_unit_count",
        node.covered_text_unit_count,
    );
    push_text(
        &mut values,
        &plan.tree_columns,
        "metadata",
        &json_string(&node.metadata)?,
    );
    push_text(&mut values, &plan.tree_columns, "status", &node.status);
    push_integer(&mut values, &plan.tree_columns, "created_at", now);
    push_integer(&mut values, &plan.tree_columns, "updated_at", now);
    execute_upsert(
        tx,
        plan.tree_storage_table()?,
        values,
        &plan
            .tree_unique_key
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        &[Some("created_at")],
    )
}

fn sync_tree_indexes(
    tx: &Transaction<'_>,
    plan: &SqliteRagCorpusPlan,
    node: &RagTreeNodeWrite,
) -> Result<(), RagError> {
    let table_name = plan.tree_storage_table()?;
    let rowid = tree_rowid_for_code(tx, table_name, &node.tree_node_code)?;
    let fts_table = fts_table_name(table_name);
    if table_exists(tx, &fts_table)? {
        let quoted_fts = quote_identifier(&fts_table)?;
        tx.execute(
            &format!("INSERT OR REPLACE INTO {quoted_fts}(rowid, search_text) VALUES (?1, ?2)"),
            params![rowid, node.summary_text],
        )
        .map_err(storage_error)?;
    }
    if let (Some(dimension), Some(raw_embedding)) = (
        node.summary_embedding_dimension,
        node.summary_embedding.as_deref(),
    ) {
        let Some(embedding) = decode_embedding(raw_embedding) else {
            return Ok(());
        };
        if embedding.len() != dimension as usize {
            return Ok(());
        }
        let vec_table = vec_table_name(table_name, dimension);
        if table_exists(tx, &vec_table)? {
            let quoted_vec = quote_identifier(&vec_table)?;
            let embedding_json =
                serde_json::to_string(&embedding).map_err(|_| RagError::StorageFailure)?;
            tx.execute(
                &format!("INSERT OR REPLACE INTO {quoted_vec}(rowid, embedding) VALUES (?1, ?2)"),
                params![rowid, embedding_json],
            )
            .map_err(storage_error)?;
        }
    }
    Ok(())
}

fn tree_rowid_for_code(
    tx: &Transaction<'_>,
    table_name: &str,
    tree_node_code: &str,
) -> Result<i64, RagError> {
    let table = quote_identifier(table_name)?;
    tx.query_row(
        &format!("SELECT id FROM {table} WHERE tree_node_code = ?1"),
        params![tree_node_code],
        |row| row.get::<_, i64>(0),
    )
    .map_err(storage_error)
}
