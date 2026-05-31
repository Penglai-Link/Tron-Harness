use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::params;
use serde_json::Value;

use crate::{
    application::rag::{
        RagRetryIndexRepository, RagRetryIndexRequest, RagRetryTextUnit, RagRetryVectorUpdate,
    },
    domain::rag::RagError,
};

use super::{SqliteRagRepository, storage_error};

impl RagRetryIndexRepository for SqliteRagRepository {
    fn retry_text_units(
        &self,
        request: &RagRetryIndexRequest,
    ) -> Result<Vec<RagRetryTextUnit>, RagError> {
        let table = text_unit_table(&request.corpus_id)?;
        let sql = format!(
            "SELECT text_unit_code, text
             FROM {table}
             WHERE source_code = ?1 AND source_version = ?2 AND scope_type = ?3 AND scope_code = ?4
               AND status = 'ACTIVE'
             ORDER BY ordinal"
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code
                ],
                |row| {
                    Ok(RagRetryTextUnit {
                        text_unit_code: row.get(0)?,
                        text: row.get(1)?,
                    })
                },
            )
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn rebuild_fulltext_index(&self, request: &RagRetryIndexRequest) -> Result<usize, RagError> {
        let table = text_unit_table(&request.corpus_id)?;
        let sql = format!(
            "UPDATE {table}
             SET search_text = text, fulltext_status = 'INDEXED', updated_at = ?1
             WHERE source_code = ?2 AND source_version = ?3 AND scope_type = ?4 AND scope_code = ?5
               AND status = 'ACTIVE'"
        );
        self.connection
            .execute(
                &sql,
                params![
                    now_millis(),
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code
                ],
            )
            .map_err(storage_error)
    }

    fn update_vector_index(&self, update: RagRetryVectorUpdate) -> Result<(), RagError> {
        let table = text_unit_table(&update.corpus_id)?;
        let sql = format!(
            "UPDATE {table}
             SET embedding_model_code = ?1,
                 embedding_version = ?2,
                 embedding_text_hash = ?3,
                 embedding_dimension = ?4,
                 embedding = ?5,
                 embedding_status = 'INDEXED',
                 updated_at = ?6
             WHERE text_unit_code = ?7"
        );
        self.connection
            .execute(
                &sql,
                params![
                    update.embedding_model_code,
                    update.embedding_version,
                    update.embedding_text_hash,
                    update.embedding_dimension,
                    update.embedding,
                    now_millis(),
                    update.text_unit_code
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn mark_vector_retry_failed(&self, request: &RagRetryIndexRequest) -> Result<(), RagError> {
        let table = text_unit_table(&request.corpus_id)?;
        let sql = format!(
            "UPDATE {table}
             SET embedding_status = 'FAILED', updated_at = ?1
             WHERE source_code = ?2 AND source_version = ?3 AND scope_type = ?4 AND scope_code = ?5"
        );
        self.connection
            .execute(
                &sql,
                params![
                    now_millis(),
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn persist_retry_warning(
        &self,
        request: &RagRetryIndexRequest,
        stage_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), RagError> {
        let now = now_millis();
        let record_code = format!(
            "{}:{}:{}:{}:{}",
            request.corpus_id,
            request.source_code,
            request.source_version,
            request.scope_code,
            stage_id
        );
        self.connection
            .execute(
                "INSERT INTO rag_diagnostics (
                    record_type, record_code, corpus_id, source_code, source_version,
                    scope_type, scope_code, workspace_code, user_code, agent_code, stage_id,
                    message, payload_json, created_at
                )
                VALUES ('INDEXING_WARNING', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT (record_code) DO UPDATE SET
                    message = excluded.message,
                    payload_json = excluded.payload_json,
                    created_at = excluded.created_at",
                params![
                    record_code,
                    request.corpus_id,
                    request.source_code,
                    request.source_version,
                    request.scope_type,
                    request.scope_code,
                    request.workspace_code,
                    request.user_code,
                    request.agent_code,
                    stage_id,
                    message,
                    json_string(&serde_json::json!({ "code": code, "stageId": stage_id }))?,
                    now,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn text_unit_table(corpus_id: &str) -> Result<&'static str, RagError> {
    match corpus_id {
        "memory-card" => Ok("rag_memory_text_units"),
        "project-doc" => Ok("rag_project_doc_text_units"),
        _ => Err(RagError::CorpusReserved),
    }
}

fn json_string(value: &Value) -> Result<String, RagError> {
    serde_json::to_string(value).map_err(|_| RagError::StorageFailure)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
