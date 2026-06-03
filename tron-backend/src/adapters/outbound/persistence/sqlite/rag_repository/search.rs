mod filters;
mod terms;

use rusqlite::{
    Row, params, params_from_iter,
    types::{Value as SqlValue, ValueRef},
};
use serde_json::{Value, json};

use crate::{
    application::rag::{
        RagConfigRepository, RagRouteSearchQuery, RagSearchRepository, RagSearchRow,
        RagSearchTraceWrite,
    },
    domain::rag::RagError,
    shared::sqlite_extensions::{sqlite_fts5_available, sqlite_vec_available},
};

use super::{
    SqliteRagRepository,
    indexes::{fts_table_name, supported_vector_dimensions, table_exists, vec_table_name},
    sql_plan::{
        SqliteRagCorpusPlan, SqliteRagValueType, compile_sql_corpus_plan, quote_identifier,
    },
    storage_error,
};

use filters::append_filter_clauses;
use terms::{fts_query, search_terms};

impl RagSearchRepository for SqliteRagRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError> {
        let corpus = self
            .get_corpus(&query.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        search_route(self, &plan, &query)
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        crate::application::rag::safe_trace_payload(&trace.payload)?;
        self.connection
            .execute(
                "INSERT INTO rag_diagnostics (
                    record_type, record_code, profile_id, workspace_code, user_code, agent_code,
                    query_text, payload_json, created_at, expires_at
                )
                VALUES ('SEARCH_TRACE', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT (record_code) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    created_at = excluded.created_at,
                    expires_at = excluded.expires_at",
                params![
                    trace.record_code,
                    trace.profile_id,
                    trace.workspace_code,
                    trace.user_code,
                    trace.agent_code,
                    trace.query_text,
                    json_string(&trace.payload)?,
                    trace.created_at,
                    trace.expires_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn search_route(
    repository: &SqliteRagRepository,
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
) -> Result<Vec<RagSearchRow>, RagError> {
    let retriever = sqlite_retriever_strategy(&query.retriever)?;
    validate_retriever_available(repository, plan, query, retriever)?;
    let select = select_projection(plan, query, retriever)?;
    let from_sql = from_sql(plan, query, retriever)?;
    let (where_sql, where_params) = where_sql(plan, query, retriever)?;
    let limit_sql = if retriever.limits_in_application {
        String::new()
    } else {
        " LIMIT ?".to_string()
    };
    let order_sql = order_sql(plan, query, retriever)?;
    let sql = format!(
        "SELECT {} {from_sql} {where_sql} {order_sql}{limit_sql}",
        select.sql
    );
    let mut params = select.params.clone();
    params.extend(where_params);
    params.extend(order_params(query, retriever)?);
    if !retriever.limits_in_application {
        params.push(SqlValue::Integer(query.limit.max(1) as i64));
    }
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    let rows = statement
        .query_map(params_from_iter(params), |row| map_row(row, plan, &select))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(rows)
}

struct SelectProjection {
    sql: String,
    metadata_start: usize,
    storage_score_index: usize,
    params: Vec<SqlValue>,
}

fn select_projection(
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<SelectProjection, RagError> {
    let mut columns = vec![
        text_expr("text_unit_code")?,
        text_expr("source_code")?,
        text_expr("source_version")?,
        text_expr("scope_type")?,
        text_expr("scope_code")?,
        text_expr("text")?,
        search_text_expr(plan, retriever)?,
        text_expr("heading_path")?,
        text_expr("ordinal")?,
        text_expr("token_count")?,
        "NULL".into(),
        optional_source_expr(plan, "title", "NULL")?,
        optional_source_expr(plan, "source_type", "NULL")?,
        optional_source_column_expr(plan, plan.source_content_hash_column.as_deref(), "NULL")?,
        text_expr(&plan.text_unit_status_column)?,
        source_metadata_expr(plan)?,
        text_metadata_expr(plan)?,
    ];
    let metadata_start = columns.len();
    for column in &plan.source_metadata_columns {
        columns.push(source_expr(plan, &column.column)?);
    }
    let storage_score_index = columns.len();
    let (storage_score_expr, params) = storage_score_expr(plan, query, retriever)?;
    columns.push(storage_score_expr);
    Ok(SelectProjection {
        sql: columns.join(", "),
        metadata_start,
        storage_score_index,
        params,
    })
}

fn from_sql(
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<String, RagError> {
    let text_table = quote_identifier(&plan.text_unit_table)?;
    let text_source = match retriever.kind {
        SqliteRetrieverKind::FulltextLike | SqliteRetrieverKind::FulltextFts5 => {
            let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
            format!(
                "FROM {fts_table}
                 JOIN {text_table} tu ON tu.id = {fts_table}.rowid"
            )
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            let dimension = query_embedding_dimension(query)?;
            let vec_table = quote_identifier(&vec_table_name(&plan.text_unit_table, dimension))?;
            format!(
                "FROM {vec_table} vec
                 JOIN {text_table} tu ON tu.id = vec.rowid"
            )
        }
    };
    if let Some(source_table) = plan.source_table.as_deref() {
        let source_table = quote_identifier(source_table)?;
        Ok(format!(
            "{text_source}
             JOIN {source_table} src
               ON src.source_code = tu.source_code
              AND src.source_version = tu.source_version
              AND src.scope_type = tu.scope_type
              AND src.scope_code = tu.scope_code"
        ))
    } else {
        Ok(text_source)
    }
}

fn storage_score_expr(
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<(String, Vec<SqlValue>), RagError> {
    match retriever.kind {
        SqliteRetrieverKind::FulltextFts5 => {
            let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
            Ok((format!("-bm25({fts_table})"), Vec::new()))
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => Ok((
            "1.0 - vec_distance_cosine(vec.embedding, ?)".into(),
            vec![SqlValue::Text(query_embedding_json(query)?)],
        )),
        _ => Ok(("NULL".into(), Vec::new())),
    }
}

fn order_sql(
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<String, RagError> {
    match retriever.kind {
        SqliteRetrieverKind::FulltextFts5 => {
            let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
            Ok(format!(
                "ORDER BY -bm25({fts_table}) DESC, tu.source_code, tu.ordinal"
            ))
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            query_embedding_json(query)?;
            Ok(
                "ORDER BY vec_distance_cosine(vec.embedding, ?) ASC, tu.source_code, tu.ordinal"
                    .into(),
            )
        }
        _ => Ok("ORDER BY tu.source_code, tu.ordinal".into()),
    }
}

fn order_params(
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<Vec<SqlValue>, RagError> {
    match retriever.kind {
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            Ok(vec![SqlValue::Text(query_embedding_json(query)?)])
        }
        _ => Ok(Vec::new()),
    }
}

fn validate_retriever_available(
    repository: &SqliteRagRepository,
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<(), RagError> {
    match retriever.kind {
        SqliteRetrieverKind::FulltextLike | SqliteRetrieverKind::FulltextFts5 => {
            if sqlite_fts5_available(&repository.connection)
                && table_exists(
                    &repository.connection,
                    &fts_table_name(&plan.text_unit_table),
                )?
            {
                Ok(())
            } else {
                Err(RagError::SearchRouteUnavailable)
            }
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            let dimension = query_embedding_dimension(query)?;
            if !supported_vector_dimensions().contains(&dimension) {
                return Err(RagError::SearchRouteUnavailable);
            }
            if sqlite_vec_available(&repository.connection)
                && table_exists(
                    &repository.connection,
                    &vec_table_name(&plan.text_unit_table, dimension),
                )?
            {
                Ok(())
            } else {
                Err(RagError::SearchRouteUnavailable)
            }
        }
    }
}

fn where_sql(
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<(String, Vec<SqlValue>), RagError> {
    let mut clauses = vec![
        "tu.source_version = ?".to_string(),
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
        SqliteRetrieverKind::FulltextLike => {
            let terms = search_terms(&query.query);
            if !terms.is_empty() {
                let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
                clauses.push(format!(
                    "({})",
                    terms
                        .iter()
                        .map(|_| format!("LOWER({fts_table}.search_text) LIKE ?"))
                        .collect::<Vec<_>>()
                        .join(" OR ")
                ));
                params.extend(
                    terms
                        .into_iter()
                        .map(|term| SqlValue::Text(format!("%{term}%"))),
                );
            }
        }
        SqliteRetrieverKind::FulltextFts5 => {
            let fts_query = fts_query(&query.query);
            if !fts_query.is_empty() {
                let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
                clauses.push(format!("{fts_table} MATCH ?"));
                params.push(SqlValue::Text(fts_query));
            }
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            query_embedding_dimension(query)?;
        }
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
    Ok((format!("WHERE {}", clauses.join(" AND ")), params))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqliteRetrieverKind {
    FulltextLike,
    FulltextFts5,
    VectorCosine,
    VectorVec0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SqliteRetrieverStrategy {
    kind: SqliteRetrieverKind,
    limits_in_application: bool,
}

fn sqlite_retriever_strategy(strategy_id: &str) -> Result<SqliteRetrieverStrategy, RagError> {
    match strategy_id {
        "FULLTEXT" | "FULLTEXT_SQLITE_LIKE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::FulltextLike,
            limits_in_application: false,
        }),
        "FULLTEXT_SQLITE_FTS5_SIMPLE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::FulltextFts5,
            limits_in_application: false,
        }),
        "VECTOR" | "VECTOR_SQLITE_COSINE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::VectorCosine,
            limits_in_application: false,
        }),
        "VECTOR_SQLITE_VEC0_COSINE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::VectorVec0,
            limits_in_application: false,
        }),
        _ => Err(RagError::SearchRouteUnavailable),
    }
}

fn map_row(
    row: &Row<'_>,
    plan: &SqliteRagCorpusPlan,
    select: &SelectProjection,
) -> rusqlite::Result<RagSearchRow> {
    let heading_path = split_heading_path(&row.get::<_, String>(7)?);
    let mut source_metadata = json!({
        "title": row.get::<_, Option<String>>(11)?,
        "sourceType": row.get::<_, Option<String>>(12)?,
        "sourceContentHash": row.get::<_, Option<String>>(13)?,
        "textUnitStatus": row.get::<_, Option<String>>(14)?,
        "extra": parse_metadata_json(row.get::<_, String>(15)?.as_str()),
    });
    if let Some(object) = source_metadata.as_object_mut() {
        for (offset, column) in plan.source_metadata_columns.iter().enumerate() {
            object.insert(
                column.logical_name.clone(),
                json_from_row(row, select.metadata_start + offset, column.value_type)?,
            );
        }
    }
    Ok(RagSearchRow {
        corpus_id: plan.corpus_id.clone(),
        source_code: row.get(1)?,
        source_version: row.get(2)?,
        target_code: row.get(0)?,
        scope_type: row.get(3)?,
        scope_code: row.get(4)?,
        text: row.get(5)?,
        search_text: row.get(6)?,
        heading_path,
        ordinal: row.get(8)?,
        token_count: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
        embedding: row
            .get::<_, Option<Vec<u8>>>(10)?
            .and_then(decode_embedding),
        storage_score: row.get::<_, Option<f64>>(select.storage_score_index)?,
        source_metadata,
        text_metadata: parse_metadata_json(row.get::<_, String>(16)?.as_str()),
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
            let fts_table = quote_identifier(&fts_table_name(&plan.text_unit_table))?;
            Ok(format!("{fts_table}.search_text"))
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => text_expr("text"),
    }
}

fn source_expr(plan: &SqliteRagCorpusPlan, column: &str) -> Result<String, RagError> {
    if plan.source_table.is_some() {
        Ok(format!("src.{}", quote_identifier(column)?))
    } else {
        Ok(format!("tu.{}", quote_identifier(column)?))
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

fn optional_source_column_expr(
    plan: &SqliteRagCorpusPlan,
    column: Option<&str>,
    fallback: &str,
) -> Result<String, RagError> {
    if let Some(column) = column {
        source_expr(plan, column)
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
