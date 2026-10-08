use super::SqliteRagRepositorySql;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::{
    application::rag::{
        RagConfigRepository, RagDebugSourceRecord, RagDebugSourceRequest, RagDebugSourceResult,
        RagDebugTextUnitRecord, RagDiagnosticWarningRecord, RagDiagnosticsRepository,
        RagSearchTraceWrite,
    },
    domain::rag::RagError,
};

use super::{
    SqliteRagRepository,
    indexes::{fts_table_name, supported_vector_dimensions, table_exists, vec_table_name},
    sql_plan::{SqliteRagCorpusPlan, compile_sql_corpus_plan, quote_identifier},
    storage_error,
};

impl RagDiagnosticsRepository for SqliteRagRepositorySql<'_> {
    fn debug_source(
        &self,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError> {
        let corpus = self
            .get_corpus(&request.corpus_id)?
            .ok_or(RagError::CorpusNotFound)?;
        let plan = compile_sql_corpus_plan(self, &corpus)?;
        debug_source(self, &plan, request)
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        safe_payload(&trace.payload)?;
        self.connection
            .execute(
                "INSERT INTO rag_diagnostics (
                    record_type, record_code, task_id, workspace_code, user_code, agent_code,
                    query_text, payload_json, created_at, expires_at
                )
                VALUES ('SEARCH_TRACE', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT (record_code) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    created_at = excluded.created_at,
                    expires_at = excluded.expires_at",
                params![
                    trace.record_code,
                    trace.task_id,
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

fn debug_source(
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    request: &RagDebugSourceRequest,
) -> Result<Option<RagDebugSourceResult>, RagError> {
    let source_table = plan.source_storage_table();
    let title_expr = if plan.source_columns.contains("title") {
        quote_identifier("title")?
    } else {
        "NULL".to_string()
    };
    let metadata_expr = plan
        .source_metadata_column
        .as_deref()
        .map(quote_identifier)
        .transpose()?
        .unwrap_or_else(|| "'{}'".to_string());
    let order_sql = if plan.has_distinct_source_table() {
        String::new()
    } else {
        " ORDER BY ordinal LIMIT 1".to_string()
    };
    let sql = format!(
        "SELECT source_code, source_version, scope_type, scope_code, {}, {title_expr}, {metadata_expr}
         FROM {}
         WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4{order_sql}",
        quote_identifier(&plan.source_status_column)?,
        quote_identifier(source_table)?,
    );
    let source = repository
        .connection
        .query_row(
            &sql,
            params![
                request.source_code,
                request.source_version,
                request.scope_type,
                request.scope_code
            ],
            |row| {
                Ok(RagDebugSourceRecord {
                    corpus_id: request.corpus_id.clone(),
                    source_code: row.get(0)?,
                    source_version: row.get(1)?,
                    scope_type: row.get(2)?,
                    scope_code: row.get(3)?,
                    status: row.get(4)?,
                    title: row.get(5)?,
                    metadata: parse_json(row.get::<_, String>(6)?.as_str()),
                })
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some(source) = source else {
        return Ok(None);
    };
    Ok(Some(RagDebugSourceResult {
        text_units: if request.include_text_units {
            debug_text_units(repository, plan, request)?
        } else {
            Vec::new()
        },
        warnings: if request.include_warnings {
            debug_warnings(repository, plan, request)?
        } else {
            Vec::new()
        },
        source,
    }))
}

fn debug_text_units(
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    request: &RagDebugSourceRequest,
) -> Result<Vec<RagDebugTextUnitRecord>, RagError> {
    let text_table = quote_identifier(&plan.text_unit_table)?;
    let fts_table_name = fts_table_name(plan.fulltext_storage_table());
    let (fulltext_status_expr, fts_join) = if table_exists(&repository.connection, &fts_table_name)?
    {
        let fts_table = quote_identifier(&fts_table_name)?;
        if plan.has_distinct_fulltext_table() {
            let fulltext_table = quote_identifier(plan.fulltext_storage_table())?;
            (
                format!("CASE WHEN {fts_table}.rowid IS NULL THEN 'PENDING' ELSE 'INDEXED' END"),
                format!(
                    "LEFT JOIN {fulltext_table} ft ON ft.text_unit_code = tu.text_unit_code
                     LEFT JOIN {fts_table} ON {fts_table}.rowid = ft.id"
                ),
            )
        } else {
            (
                format!("CASE WHEN {fts_table}.rowid IS NULL THEN 'PENDING' ELSE 'INDEXED' END"),
                format!("LEFT JOIN {fts_table} ON {fts_table}.rowid = tu.id"),
            )
        }
    } else {
        ("'PENDING'".to_string(), String::new())
    };
    let vector_dimension = supported_vector_dimensions()
        .first()
        .copied()
        .ok_or(RagError::StorageBindingInvalid)?;
    let vec_table_name = vec_table_name(plan.vector_storage_table(), vector_dimension);
    let (embedding_status_expr, vec_join) =
        if table_exists(&repository.connection, &vec_table_name)? {
            let vec_table = quote_identifier(&vec_table_name)?;
            if plan.has_distinct_vector_table() {
                let vector_table = quote_identifier(plan.vector_storage_table())?;
                (
                    format!(
                        "CASE WHEN {vec_table}.rowid IS NULL THEN 'PENDING' ELSE 'INDEXED' END"
                    ),
                    format!(
                        "LEFT JOIN {vector_table} vt ON vt.text_unit_code = tu.text_unit_code
                         LEFT JOIN {vec_table} ON {vec_table}.rowid = vt.id"
                    ),
                )
            } else {
                (
                    format!(
                        "CASE WHEN {vec_table}.rowid IS NULL THEN 'PENDING' ELSE 'INDEXED' END"
                    ),
                    format!("LEFT JOIN {vec_table} ON {vec_table}.rowid = tu.id"),
                )
            }
        } else {
            ("'PENDING'".to_string(), String::new())
        };
    let sql = format!(
        "SELECT tu.text_unit_code, tu.ordinal, tu.{}, {fulltext_status_expr}, {embedding_status_expr},
                NULL, tu.heading_path, tu.token_count
         FROM {text_table} tu
         {fts_join}
         {vec_join}
         WHERE tu.source_code = ?1 AND tu.source_version = ?2
           AND tu.scope_type = ?3 AND tu.scope_code = ?4
         ORDER BY tu.ordinal",
        quote_identifier(&plan.text_unit_status_column)?,
    );
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                request.source_code,
                request.source_version,
                request.scope_type,
                request.scope_code
            ],
            |row| {
                Ok(RagDebugTextUnitRecord {
                    text_unit_code: row.get(0)?,
                    ordinal: row.get(1)?,
                    status: row.get(2)?,
                    fulltext_status: row.get(3)?,
                    embedding_status: row.get(4)?,
                    embedding_model_code: row.get(5)?,
                    heading_path: row.get(6)?,
                    token_count: row.get(7)?,
                })
            },
        )
        .map_err(storage_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

fn debug_warnings(
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    request: &RagDebugSourceRequest,
) -> Result<Vec<RagDiagnosticWarningRecord>, RagError> {
    let diagnostics_table = plan
        .diagnostics_table
        .as_deref()
        .ok_or(RagError::StorageBindingInvalid)?;
    let sql = format!(
        "SELECT record_code, stage_id, message, payload_json, created_at
         FROM {}
         WHERE record_type = 'INDEXING_WARNING'
           AND corpus_id = ?1
           AND source_code = ?2
           AND source_version = ?3
         ORDER BY created_at DESC",
        quote_identifier(diagnostics_table)?,
    );
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                request.corpus_id,
                request.source_code,
                request.source_version
            ],
            |row| {
                let payload = parse_json(row.get::<_, String>(3)?.as_str());
                Ok(RagDiagnosticWarningRecord {
                    record_code: row.get(0)?,
                    stage_id: row.get(1)?,
                    code: payload
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or("RAG_INDEXING_WARNING")
                        .to_string(),
                    message: row.get(2)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .map_err(storage_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
}

fn safe_payload(payload: &Value) -> Result<(), RagError> {
    crate::application::rag::safe_trace_payload(payload)
}

fn json_string(value: &Value) -> Result<String, RagError> {
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}

fn parse_json(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
}

impl RagDiagnosticsRepository for SqliteRagRepository {
    fn debug_source(
        &self,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError> {
        let request = request.to_owned();
        self.connection.call(
            move |connection| SqliteRagRepositorySql { connection }.debug_source(&request),
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

impl crate::application::rag::RagSourceContentRepository for SqliteRagRepository {
    fn source_units_page(
        &self,
        request: RagDebugSourceRequest,
        page: usize,
        page_size: usize,
    ) -> Result<crate::application::rag::RagSourceUnitsPage, RagError> {
        if page == 0 || page_size == 0 || page_size > 100 || page > 1_000_000 {
            return Err(RagError::DebugRequestInvalid);
        }
        self.connection.call(move |connection| {
            let repository = SqliteRagRepositorySql { connection };
            let corpus = repository.get_corpus(&request.corpus_id)?.ok_or(RagError::CorpusNotFound)?;
            let plan = compile_sql_corpus_plan(&repository, &corpus)?;
            let table = quote_identifier(&plan.text_unit_table)?;
            let status = quote_identifier(&plan.text_unit_status_column)?;
            let source_status = quote_identifier(&plan.source_status_column)?;
            // KB uses an embedded source/text layout. A retired or staging generation is never visible.
            if plan.has_distinct_source_table() { return Err(RagError::StorageBindingInvalid); }
            let filter = format!("source_code=?1 AND source_version=?2 AND scope_type=?3 AND scope_code=?4 AND {status}='ACTIVE' AND {source_status}='ACTIVE'");
            let total: usize = repository.connection.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {filter}"), params![request.source_code, request.source_version, request.scope_type, request.scope_code], |row| row.get(0)).map_err(storage_error)?;
            let mut statement = repository.connection.prepare(&format!("SELECT text_unit_code,ordinal,text,heading_path,metadata FROM {table} WHERE {filter} ORDER BY ordinal,text_unit_code LIMIT ?5 OFFSET ?6")).map_err(storage_error)?;
            let items = statement.query_map(params![request.source_code, request.source_version, request.scope_type, request.scope_code, page_size, (page-1)*page_size], |row| {
                Ok(crate::application::rag::RagSourceUnit {
                    text_unit_code: row.get(0)?, ordinal: row.get(1)?, text: row.get(2)?,
                    heading_path: serde_json::from_str(&row.get::<_,String>(3)?).unwrap_or_default(),
                    metadata: parse_json(&row.get::<_,String>(4)?),
                })
            }).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
            Ok(crate::application::rag::RagSourceUnitsPage { items, total, page, page_size })
        }, || storage_error(rusqlite::Error::InvalidQuery))
    }
}
