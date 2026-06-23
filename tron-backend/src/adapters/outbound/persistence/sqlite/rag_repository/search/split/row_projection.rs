fn map_row(
    row: &Row<'_>,
    plan: &SqliteRagCorpusPlan,
    select: &SelectProjection,
) -> rusqlite::Result<RagSearchRow> {
    let heading_path = split_heading_path(&row.get::<_, String>(7)?);
    let mut field_values = Map::new();
    for (offset, column) in select.return_field_columns.iter().enumerate() {
        let value = json_from_row(row, select.return_field_start + offset, column.value_type)?;
        if !value.is_null() {
            field_values.insert(column.logical_name.clone(), value);
        }
    }
    Ok(RagSearchRow {
        corpus_id: plan.corpus_id.clone(),
        source_code: row.get(1)?,
        source_version: row.get(2)?,
        target_code: row.get(0)?,
        scope_type: row.get(3)?,
        scope_code: row.get(4)?,
        title: row.get(11)?,
        text: row.get(5)?,
        search_text: row.get(6)?,
        heading_path,
        ordinal: row.get(8)?,
        token_count: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
        embedding: embedding_from_row(row, 10)?,
        storage_score: row.get::<_, Option<f64>>(select.storage_score_index)?,
        source_metadata: parse_metadata_json(
            row.get::<_, Option<String>>(12)?.as_deref().unwrap_or("{}"),
        ),
        text_metadata: parse_metadata_json(
            row.get::<_, Option<String>>(13)?.as_deref().unwrap_or("{}"),
        ),
        field_values,
    })
}

fn map_tree_row(
    row: &Row<'_>,
    plan: &SqliteRagCorpusPlan,
    retriever: SqliteRetrieverStrategy,
) -> rusqlite::Result<RagSearchRow> {
    let heading_path = split_heading_path(&row.get::<_, String>(7)?);
    let tree_score = row.get::<_, Option<f64>>(16)?;
    let mut field_values = Map::new();
    field_values.insert(
        "matchedBy".into(),
        json!(match retriever.kind {
            SqliteRetrieverKind::TreeVector => "TREE_VECTOR_V1",
            _ => "TREE_FULLTEXT_V1",
        }),
    );
    field_values.insert("treeNodeCode".into(), json!(row.get::<_, String>(14)?));
    field_values.insert("treeNodeTitle".into(), json!(row.get::<_, String>(15)?));
    if let Some(score) = tree_score {
        field_values.insert("treeScore".into(), json!(score));
    }
    Ok(RagSearchRow {
        corpus_id: plan.corpus_id.clone(),
        source_code: row.get(1)?,
        source_version: row.get(2)?,
        target_code: row.get(0)?,
        scope_type: row.get(3)?,
        scope_code: row.get(4)?,
        title: row.get(11)?,
        text: row.get(5)?,
        search_text: row.get(6)?,
        heading_path,
        ordinal: row.get(8)?,
        token_count: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
        embedding: embedding_from_row(row, 10)?,
        storage_score: tree_score,
        source_metadata: parse_metadata_json(
            row.get::<_, Option<String>>(12)?.as_deref().unwrap_or("{}"),
        ),
        text_metadata: parse_metadata_json(
            row.get::<_, Option<String>>(13)?.as_deref().unwrap_or("{}"),
        ),
        field_values,
    })
}

fn text_expr(column: &str) -> Result<String, RagError> {
    Ok(format!("tu.{}", quote_identifier(column)?))
}

fn search_text_expr(
    plan: &SqliteRagCorpusPlan,
    retriever: SqliteRetrieverStrategy,
) -> Result<String, RagError> {
    match retriever.kind {
        SqliteRetrieverKind::FulltextLike | SqliteRetrieverKind::FulltextFts5 => {
            let fts_table = quote_identifier(&fts_table_name(plan.fulltext_storage_table()))?;
            Ok(format!("{fts_table}.search_text"))
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => text_expr("text"),
        SqliteRetrieverKind::StructuredFilter
        | SqliteRetrieverKind::TreeFulltext
        | SqliteRetrieverKind::TreeVector => text_expr("text"),
    }
}

fn embedding_expr(
    plan: &SqliteRagCorpusPlan,
    retriever: SqliteRetrieverStrategy,
) -> Result<String, RagError> {
    if plan.text_unit_columns.contains("embedding") {
        return text_expr("embedding");
    }
    match retriever.kind {
        SqliteRetrieverKind::VectorCosine
        | SqliteRetrieverKind::VectorVec0
        | SqliteRetrieverKind::TreeVector => {
            Ok("vec.embedding".into())
        }
        _ => Ok("NULL".into()),
    }
}

fn source_expr(plan: &SqliteRagCorpusPlan, column: &str) -> Result<String, RagError> {
    if plan.source_table.is_some() {
        Ok(format!("src.{}", quote_identifier(column)?))
    } else {
        Ok(format!("tu.{}", quote_identifier(column)?))
    }
}

fn return_field_expr(
    plan: &SqliteRagCorpusPlan,
    column: &SqliteRagProjectionColumn,
) -> Result<String, RagError> {
    match column.alias {
        SqliteRagTableAlias::Source => source_expr(plan, &column.column),
        SqliteRagTableAlias::TextUnit => text_expr(&column.column),
    }
}

fn optional_source_expr(
    plan: &SqliteRagCorpusPlan,
    column: &str,
    fallback: &str,
) -> Result<String, RagError> {
    if plan.source_table.is_some() && plan.source_columns.contains(column) {
        source_expr(plan, column)
    } else if plan.source_table.is_none() && plan.text_unit_columns.contains(column) {
        text_expr(column)
    } else {
        Ok(fallback.into())
    }
}

fn source_metadata_expr(plan: &SqliteRagCorpusPlan) -> Result<String, RagError> {
    if let Some(column) = plan.source_metadata_column.as_deref() {
        source_expr(plan, column)
    } else {
        Ok("'{}'".into())
    }
}

fn text_metadata_expr(plan: &SqliteRagCorpusPlan) -> Result<String, RagError> {
    if let Some(column) = plan.text_unit_metadata_column.as_deref() {
        text_expr(column)
    } else {
        Ok("'{}'".into())
    }
}

fn json_from_row(
    row: &Row<'_>,
    index: usize,
    value_type: SqliteRagValueType,
) -> rusqlite::Result<Value> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(Value::Null),
        ValueRef::Integer(value) if value_type == SqliteRagValueType::Boolean => {
            Ok(json!(value != 0))
        }
        ValueRef::Integer(value) => Ok(json!(value)),
        ValueRef::Real(value) => Ok(json!(value)),
        ValueRef::Text(value) => Ok(json!(String::from_utf8_lossy(value).to_string())),
        ValueRef::Blob(_) => Ok(Value::Null),
    }
}

fn embedding_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Vec<f32>>> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(None),
        ValueRef::Blob(raw) => {
            Ok(decode_embedding(raw.to_vec())
                .or_else(|| serde_json::from_slice::<Vec<f32>>(raw).ok()))
        }
        ValueRef::Text(raw) => Ok(serde_json::from_slice::<Vec<f32>>(raw).ok()),
        _ => Ok(None),
    }
}

fn query_embedding_dimension(query: &RagRouteSearchQuery) -> Result<i64, RagError> {
    query
        .query_embedding
        .as_ref()
        .map(|embedding| embedding.len() as i64)
        .filter(|dimension| *dimension > 0)
        .ok_or(RagError::SearchRouteUnavailable)
}

fn query_embedding_json(query: &RagRouteSearchQuery) -> Result<String, RagError> {
    let embedding = query
        .query_embedding
        .as_ref()
        .filter(|embedding| !embedding.is_empty())
        .ok_or(RagError::SearchRouteUnavailable)?;
    serde_json::to_string(embedding).map_err(|_| RagError::StorageFailure)
}

fn split_heading_path(raw: &str) -> Vec<String> {
    raw.split('>')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn decode_embedding(raw: Vec<u8>) -> Option<Vec<f32>> {
    if !raw.len().is_multiple_of(4) {
        return None;
    }
    Some(
        raw.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

fn parse_metadata_json(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn json_string(value: &Value) -> Result<String, RagError> {
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}
