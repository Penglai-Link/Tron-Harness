use super::SqliteRagRepositorySql;
use rusqlite::{
    Row, params, params_from_iter,
    types::{Value as SqlValue, ValueRef},
};
use serde_json::{Map, Value, json};

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
        SqliteRagCorpusPlan, SqliteRagProjectionColumn, SqliteRagTableAlias, SqliteRagValueType,
        compile_sql_corpus_plan, quote_identifier,
    },
    storage_error,
};

use filters::append_filter_clauses;
use terms::{fts_query, search_terms};

impl RagSearchRepository for SqliteRagRepositorySql<'_> {
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

impl<'a> SqliteRagRepositorySql<'a> {
    pub(crate) fn hydrate_graph_rows_from_text_units(
        &self,
        corpus: &crate::domain::rag::RagCorpusRecord,
        rows: Vec<RagSearchRow>,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let plan = compile_sql_corpus_plan(self, corpus)?;
        rows.into_iter()
            .map(|row| hydrate_graph_row_from_text_units(self, &plan, row))
            .collect()
    }
}

fn search_route(
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
) -> Result<Vec<RagSearchRow>, RagError> {
    let retriever = sqlite_retriever_strategy(&query.retriever)?;
    if matches!(
        retriever.kind,
        SqliteRetrieverKind::TreeFulltext | SqliteRetrieverKind::TreeVector
    ) {
        return search_tree_route(repository, plan, query, retriever);
    }
    validate_retriever_available(repository, plan, query, retriever)?;
    let select = select_projection(plan, query, retriever)?;
    let from_sql = from_sql(plan, query, retriever)?;
    let (where_sql, where_params) = where_sql(plan, query, retriever)?;
    let limit_sql = " LIMIT ?";
    let order_sql = order_sql(plan, query, retriever)?;
    let sql = format!(
        "SELECT {} {from_sql} {where_sql} {order_sql}{limit_sql}",
        select.sql
    );
    let mut params = select.params.clone();
    params.extend(where_params);
    params.extend(order_params(query, retriever)?);
    params.push(SqlValue::Integer(query.limit.max(1) as i64));
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    let rows = statement
        .query_map(params_from_iter(params), |row| map_row(row, plan, &select))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(rows)
}

fn hydrate_graph_row_from_text_units(
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    mut row: RagSearchRow,
) -> Result<RagSearchRow, RagError> {
    let table = quote_identifier(&plan.text_unit_table)?;
    let status_column = quote_identifier(&plan.text_unit_status_column)?;
    let sql = format!(
        "SELECT text, heading_path, token_count, metadata
         FROM {table}
         WHERE source_code = ?1 AND source_version = ?2
           AND scope_type = ?3 AND scope_code = ?4
           AND {status_column} = 'ACTIVE'
         ORDER BY ordinal"
    );
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    let units = statement
        .query_map(
            params![
                row.source_code,
                row.source_version,
                row.scope_type,
                row.scope_code
            ],
            |unit| {
                Ok((
                    unit.get::<_, String>(0)?,
                    unit.get::<_, String>(1)?,
                    unit.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    unit.get::<_, Option<String>>(3)?
                        .unwrap_or_else(|| "{}".into()),
                ))
            },
        )
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    if units.is_empty() {
        return Ok(row);
    }
    row.text = units
        .iter()
        .map(|(text, _, _, _)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    row.heading_path = split_heading_path(&units[0].1);
    row.token_count = units
        .iter()
        .map(|(_, _, token_count, _)| *token_count)
        .sum();
    row.text_metadata = parse_metadata_json(&units[0].3);
    if let Some(object) = row.text_metadata.as_object_mut() {
        object.insert("hydratedTextUnitCount".into(), json!(units.len()));
    }
    Ok(row)
}

struct SelectProjection {
    sql: String,
    return_field_start: usize,
    return_field_columns: Vec<SqliteRagProjectionColumn>,
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
        embedding_expr(plan, retriever)?,
        optional_source_expr(plan, "title", "NULL")?,
        source_metadata_expr(plan)?,
        text_metadata_expr(plan)?,
    ];
    let return_field_start = columns.len();
    let mut return_field_columns = Vec::new();
    for field in &query.return_fields {
        if let Some(column) = plan.return_field_columns.get(field) {
            columns.push(return_field_expr(plan, column)?);
            return_field_columns.push(column.clone());
        }
    }
    let storage_score_index = columns.len();
    let (storage_score_expr, params) = storage_score_expr(plan, query, retriever)?;
    columns.push(storage_score_expr);
    Ok(SelectProjection {
        sql: columns.join(", "),
        return_field_start,
        return_field_columns,
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
            let fts_table = quote_identifier(&fts_table_name(plan.fulltext_storage_table()))?;
            if plan.has_distinct_fulltext_table() {
                let fulltext_table = quote_identifier(plan.fulltext_storage_table())?;
                format!(
                    "FROM {fts_table}
                     JOIN {fulltext_table} ft ON ft.id = {fts_table}.rowid
                     JOIN {text_table} tu ON tu.text_unit_code = ft.text_unit_code"
                )
            } else {
                format!(
                    "FROM {fts_table}
                     JOIN {text_table} tu ON tu.id = {fts_table}.rowid"
                )
            }
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            let dimension = query_embedding_dimension(query)?;
            let vector_table = quote_identifier(plan.vector_storage_table())?;
            let vec_table =
                quote_identifier(&vec_table_name(plan.vector_storage_table(), dimension))?;
            if plan.has_distinct_vector_table() {
                format!(
                    "FROM {vec_table} vec
                     JOIN {vector_table} vt ON vt.id = vec.rowid
                     JOIN {text_table} tu ON tu.text_unit_code = vt.text_unit_code"
                )
            } else {
                format!(
                    "FROM {vec_table} vec
                     JOIN {text_table} tu ON tu.id = vec.rowid"
                )
            }
        }
        SqliteRetrieverKind::StructuredFilter => format!("FROM {text_table} tu"),
        SqliteRetrieverKind::TreeFulltext | SqliteRetrieverKind::TreeVector => {
            return Err(RagError::SearchRouteUnavailable);
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
            let fts_table = quote_identifier(&fts_table_name(plan.fulltext_storage_table()))?;
            Ok((format!("-bm25({fts_table})"), Vec::new()))
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => Ok((
            "1.0 - vec_distance_cosine(vec.embedding, ?)".into(),
            vec![SqlValue::Text(query_embedding_json(query)?)],
        )),
        SqliteRetrieverKind::StructuredFilter => Ok(("1.0".into(), Vec::new())),
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
            let fts_table = quote_identifier(&fts_table_name(plan.fulltext_storage_table()))?;
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
        SqliteRetrieverKind::StructuredFilter => Ok("ORDER BY tu.source_code, tu.ordinal".into()),
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
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    query: &RagRouteSearchQuery,
    retriever: SqliteRetrieverStrategy,
) -> Result<(), RagError> {
    match retriever.kind {
        SqliteRetrieverKind::FulltextLike | SqliteRetrieverKind::FulltextFts5 => {
            if !plan.has_sqlite_fulltext() {
                return Err(RagError::SearchRouteUnavailable);
            }
            if sqlite_fts5_available(&repository.connection)
                && table_exists(
                    &repository.connection,
                    &fts_table_name(plan.fulltext_storage_table()),
                )?
            {
                Ok(())
            } else {
                Err(RagError::SearchRouteUnavailable)
            }
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            if !plan.has_sqlite_vector() {
                return Err(RagError::SearchRouteUnavailable);
            }
            let dimension = query_embedding_dimension(query)?;
            if !supported_vector_dimensions().contains(&dimension) {
                return Err(RagError::SearchRouteUnavailable);
            }
            if sqlite_vec_available(&repository.connection)
                && table_exists(
                    &repository.connection,
                    &vec_table_name(plan.vector_storage_table(), dimension),
                )?
            {
                Ok(())
            } else {
                Err(RagError::SearchRouteUnavailable)
            }
        }
        SqliteRetrieverKind::TreeFulltext => {
            if !plan.has_sqlite_tree() {
                return Err(RagError::SearchRouteUnavailable);
            }
            if sqlite_fts5_available(&repository.connection)
                && table_exists(
                    &repository.connection,
                    &fts_table_name(plan.tree_storage_table()?),
                )?
            {
                Ok(())
            } else {
                Err(RagError::SearchRouteUnavailable)
            }
        }
        SqliteRetrieverKind::TreeVector => {
            if !plan.has_sqlite_tree() {
                return Err(RagError::SearchRouteUnavailable);
            }
            let dimension = query_embedding_dimension(query)?;
            if !supported_vector_dimensions().contains(&dimension) {
                return Err(RagError::SearchRouteUnavailable);
            }
            if sqlite_vec_available(&repository.connection)
                && table_exists(
                    &repository.connection,
                    &vec_table_name(plan.tree_storage_table()?, dimension),
                )?
            {
                Ok(())
            } else {
                Err(RagError::SearchRouteUnavailable)
            }
        }
        SqliteRetrieverKind::StructuredFilter => Ok(()),
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
    if plan.has_distinct_fulltext_table()
        && matches!(
            retriever.kind,
            SqliteRetrieverKind::FulltextLike | SqliteRetrieverKind::FulltextFts5
        )
    {
        clauses.push("ft.status = 'ACTIVE'".to_string());
    }
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
                let fts_table = quote_identifier(&fts_table_name(plan.fulltext_storage_table()))?;
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
                let fts_table = quote_identifier(&fts_table_name(plan.fulltext_storage_table()))?;
                clauses.push(format!("{fts_table} MATCH ?"));
                params.push(SqlValue::Text(fts_query));
            }
        }
        SqliteRetrieverKind::VectorCosine | SqliteRetrieverKind::VectorVec0 => {
            query_embedding_dimension(query)?;
        }
        SqliteRetrieverKind::StructuredFilter
        | SqliteRetrieverKind::TreeFulltext
        | SqliteRetrieverKind::TreeVector => {}
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
    StructuredFilter,
    TreeFulltext,
    TreeVector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SqliteRetrieverStrategy {
    kind: SqliteRetrieverKind,
}

fn sqlite_retriever_strategy(strategy_id: &str) -> Result<SqliteRetrieverStrategy, RagError> {
    match strategy_id {
        "FULLTEXT" | "FULLTEXT_SQLITE_LIKE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::FulltextLike,
        }),
        "FULLTEXT_SQLITE_FTS5_SIMPLE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::FulltextFts5,
        }),
        "VECTOR" | "VECTOR_SQLITE_COSINE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::VectorCosine,
        }),
        "VECTOR_SQLITE_VEC0_COSINE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::VectorVec0,
        }),
        "STRUCTURED_FILTER_SQLITE_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::StructuredFilter,
        }),
        "TREE_FULLTEXT_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::TreeFulltext,
        }),
        "TREE_VECTOR_V1" => Ok(SqliteRetrieverStrategy {
            kind: SqliteRetrieverKind::TreeVector,
        }),
        _ => Err(RagError::SearchRouteUnavailable),
    }
}

impl RagSearchRepository for SqliteRagRepository {
    fn search_route(&self, query: RagRouteSearchQuery) -> Result<Vec<RagSearchRow>, RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.search_route(query),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.persist_search_trace(trace),
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
impl SqliteRagRepository {
    pub(crate) fn hydrate_graph_rows_from_text_units(
        &self,
        corpus: &crate::domain::rag::RagCorpusRecord,
        rows: Vec<RagSearchRow>,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let corpus = corpus.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .hydrate_graph_rows_from_text_units(&corpus, rows)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
