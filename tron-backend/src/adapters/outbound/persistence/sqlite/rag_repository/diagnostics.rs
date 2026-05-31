use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::{
    application::rag::{
        RagDebugSourceRecord, RagDebugSourceRequest, RagDebugSourceResult, RagDebugTextUnitRecord,
        RagDiagnosticWarningRecord, RagDiagnosticsRepository, RagSearchTraceWrite,
    },
    domain::rag::RagError,
};

use super::{SqliteRagRepository, storage_error};

impl RagDiagnosticsRepository for SqliteRagRepository {
    fn debug_source(
        &self,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError> {
        match request.corpus_id.as_str() {
            "memory-card" => debug_memory_source(self, request),
            "project-doc" => debug_project_doc_source(self, request),
            _ => Err(RagError::CorpusReserved),
        }
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        safe_payload(&trace.payload)?;
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

fn debug_memory_source(
    repository: &SqliteRagRepository,
    request: &RagDebugSourceRequest,
) -> Result<Option<RagDebugSourceResult>, RagError> {
    let source = repository
        .connection
        .query_row(
            "SELECT source_code, source_version, scope_type, scope_code, status, title, metadata
             FROM rag_memory_sources
             WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4",
            params![
                request.source_code,
                request.source_version,
                request.scope_type,
                request.scope_code
            ],
            |row| {
                Ok(RagDebugSourceRecord {
                    corpus_id: "memory-card".into(),
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
            debug_text_units(repository, "rag_memory_text_units", request)?
        } else {
            Vec::new()
        },
        warnings: if request.include_warnings {
            debug_warnings(repository, request)?
        } else {
            Vec::new()
        },
        source,
    }))
}

fn debug_project_doc_source(
    repository: &SqliteRagRepository,
    request: &RagDebugSourceRequest,
) -> Result<Option<RagDebugSourceResult>, RagError> {
    let source = repository
        .connection
        .query_row(
            "SELECT source_code, source_version, scope_type, scope_code, status, title, metadata
             FROM rag_project_doc_sources
             WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4",
            params![
                request.source_code,
                request.source_version,
                request.scope_type,
                request.scope_code
            ],
            |row| {
                Ok(RagDebugSourceRecord {
                    corpus_id: "project-doc".into(),
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
            debug_text_units(repository, "rag_project_doc_text_units", request)?
        } else {
            Vec::new()
        },
        warnings: if request.include_warnings {
            debug_warnings(repository, request)?
        } else {
            Vec::new()
        },
        source,
    }))
}

fn debug_text_units(
    repository: &SqliteRagRepository,
    table: &str,
    request: &RagDebugSourceRequest,
) -> Result<Vec<RagDebugTextUnitRecord>, RagError> {
    let sql = format!(
        "SELECT text_unit_code, ordinal, status, fulltext_status, embedding_status,
                embedding_model_code, heading_path, token_count
         FROM {table}
         WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4
         ORDER BY ordinal"
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
    repository: &SqliteRagRepository,
    request: &RagDebugSourceRequest,
) -> Result<Vec<RagDiagnosticWarningRecord>, RagError> {
    let mut statement = repository
        .connection
        .prepare(
            "SELECT record_code, stage_id, message, payload_json, created_at
             FROM rag_diagnostics
             WHERE record_type = 'INDEXING_WARNING'
               AND corpus_id = ?1
               AND source_code = ?2
               AND source_version = ?3
             ORDER BY created_at DESC",
        )
        .map_err(storage_error)?;
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
