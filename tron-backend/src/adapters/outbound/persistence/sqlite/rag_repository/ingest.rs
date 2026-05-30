use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Transaction, params};
use serde_json::Value;

use crate::{
    application::rag::{RagIngestRepository, RagPersistIngestCommand},
    domain::rag::RagError,
};

use super::{SqliteRagRepository, storage_error};

impl RagIngestRepository for SqliteRagRepository {
    fn persist_ingest(&self, command: RagPersistIngestCommand) -> Result<(), RagError> {
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(storage_error)?;
        match command.corpus_id.as_str() {
            "memory-card" => persist_memory_source(&tx, &command)?,
            "project-doc" => persist_project_doc_source(&tx, &command)?,
            _ => return Err(RagError::CorpusReserved),
        }
        persist_text_units(&tx, &command)?;
        persist_warnings(&tx, &command)?;
        tx.commit().map_err(storage_error)
    }
}

fn persist_memory_source(
    tx: &Transaction<'_>,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let memory = command
        .memory
        .as_ref()
        .ok_or(RagError::IngestRequestInvalid)?;
    let now = now_millis();
    tx.execute(
        "INSERT INTO rag_memory_sources (
            source_code, source_version, source_type, scope_type, scope_code, status,
            title, content_hash, metadata, created_at, updated_at,
            memory_type, memory_status, review_status, version, importance_score
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT (source_code, source_version, scope_type, scope_code) DO UPDATE SET
            source_type = excluded.source_type,
            status = excluded.status,
            title = excluded.title,
            content_hash = excluded.content_hash,
            metadata = excluded.metadata,
            updated_at = excluded.updated_at,
            memory_type = excluded.memory_type,
            memory_status = excluded.memory_status,
            review_status = excluded.review_status,
            version = excluded.version,
            importance_score = excluded.importance_score",
        params![
            command.source_code,
            command.source_version,
            command.source_type,
            command.scope.scope_type,
            command.scope.scope_code,
            command.source_status,
            command.title,
            command.content_hash,
            json_string(&command.metadata)?,
            now,
            memory.memory_type,
            memory.memory_status,
            memory.review_status,
            memory.version,
            memory.importance_score,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn persist_project_doc_source(
    tx: &Transaction<'_>,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let project_doc = command
        .project_doc
        .as_ref()
        .ok_or(RagError::IngestRequestInvalid)?;
    let now = now_millis();
    tx.execute(
        "INSERT INTO rag_project_doc_sources (
            source_code, source_version, source_type, scope_type, scope_code, status,
            title, content_hash, document_code, document_type, path, body_hash, metadata,
            created_at, updated_at, repo_code, module_code, module_name
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15, ?16, ?17)
        ON CONFLICT (source_code, source_version, scope_type, scope_code) DO UPDATE SET
            source_type = excluded.source_type,
            status = excluded.status,
            title = excluded.title,
            content_hash = excluded.content_hash,
            document_code = excluded.document_code,
            document_type = excluded.document_type,
            path = excluded.path,
            body_hash = excluded.body_hash,
            metadata = excluded.metadata,
            updated_at = excluded.updated_at,
            repo_code = excluded.repo_code,
            module_code = excluded.module_code,
            module_name = excluded.module_name",
        params![
            command.source_code,
            command.source_version,
            command.source_type,
            command.scope.scope_type,
            command.scope.scope_code,
            command.source_status,
            command.title,
            command.content_hash,
            project_doc.document_code,
            project_doc.document_type,
            project_doc.path,
            project_doc.body_hash,
            json_string(&command.metadata)?,
            now,
            project_doc.repo_code,
            project_doc.module_code,
            project_doc.module_name,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn persist_text_units(
    tx: &Transaction<'_>,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let table = match command.corpus_id.as_str() {
        "memory-card" => "rag_memory_text_units",
        "project-doc" => "rag_project_doc_text_units",
        _ => return Err(RagError::CorpusReserved),
    };
    let now = now_millis();
    for unit in &command.text_units {
        if command.corpus_id == "memory-card" {
            persist_memory_text_unit(tx, command, unit, now)?;
        } else {
            persist_project_doc_text_unit(tx, command, unit, now)?;
        }
    }
    let delete_sql = format!(
        "DELETE FROM {table}
         WHERE source_code = ?1
           AND source_version = ?2
           AND scope_type = ?3
           AND scope_code = ?4
           AND ordinal >= ?5"
    );
    tx.execute(
        &delete_sql,
        params![
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            command.scope.scope_code,
            command.text_units.len() as i64,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn persist_memory_text_unit(
    tx: &Transaction<'_>,
    command: &RagPersistIngestCommand,
    unit: &crate::application::rag::RagTextUnitWrite,
    now: i64,
) -> Result<(), RagError> {
    tx.execute(
        "INSERT INTO rag_memory_text_units (
            text_unit_code, source_code, source_version, scope_type, scope_code,
            unit_type, text, search_text, heading_path, ordinal, token_count,
            content_hash, status, fulltext_status, embedding_model_code,
            embedding_version, embedding_text_hash, embedding_dimension, embedding,
            embedding_status, metadata, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?22)
        ON CONFLICT (source_code, source_version, ordinal) DO UPDATE SET
            text_unit_code = excluded.text_unit_code,
            scope_type = excluded.scope_type,
            scope_code = excluded.scope_code,
            unit_type = excluded.unit_type,
            text = excluded.text,
            search_text = excluded.search_text,
            heading_path = excluded.heading_path,
            token_count = excluded.token_count,
            content_hash = excluded.content_hash,
            status = excluded.status,
            fulltext_status = excluded.fulltext_status,
            embedding_model_code = excluded.embedding_model_code,
            embedding_version = excluded.embedding_version,
            embedding_text_hash = excluded.embedding_text_hash,
            embedding_dimension = excluded.embedding_dimension,
            embedding = excluded.embedding,
            embedding_status = excluded.embedding_status,
            metadata = excluded.metadata,
            updated_at = excluded.updated_at",
        params![
            unit.text_unit_code,
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            command.scope.scope_code,
            unit.unit_type,
            unit.text,
            unit.search_text,
            unit.heading_path,
            unit.ordinal as i64,
            unit.token_count,
            unit.content_hash,
            unit.status,
            unit.fulltext_status,
            unit.embedding_model_code,
            unit.embedding_version,
            unit.embedding_text_hash,
            unit.embedding_dimension,
            unit.embedding,
            unit.embedding_status,
            json_string(&unit.metadata)?,
            now,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn persist_project_doc_text_unit(
    tx: &Transaction<'_>,
    command: &RagPersistIngestCommand,
    unit: &crate::application::rag::RagTextUnitWrite,
    now: i64,
) -> Result<(), RagError> {
    tx.execute(
        "INSERT INTO rag_project_doc_text_units (
            text_unit_code, source_code, source_version, document_code, scope_type, scope_code,
            unit_type, text, search_text, heading_path, ordinal, token_count,
            content_hash, status, fulltext_status, embedding_model_code,
            embedding_version, embedding_text_hash, embedding_dimension, embedding,
            embedding_status, metadata, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?23)
        ON CONFLICT (source_code, source_version, ordinal) DO UPDATE SET
            text_unit_code = excluded.text_unit_code,
            document_code = excluded.document_code,
            scope_type = excluded.scope_type,
            scope_code = excluded.scope_code,
            unit_type = excluded.unit_type,
            text = excluded.text,
            search_text = excluded.search_text,
            heading_path = excluded.heading_path,
            token_count = excluded.token_count,
            content_hash = excluded.content_hash,
            status = excluded.status,
            fulltext_status = excluded.fulltext_status,
            embedding_model_code = excluded.embedding_model_code,
            embedding_version = excluded.embedding_version,
            embedding_text_hash = excluded.embedding_text_hash,
            embedding_dimension = excluded.embedding_dimension,
            embedding = excluded.embedding,
            embedding_status = excluded.embedding_status,
            metadata = excluded.metadata,
            updated_at = excluded.updated_at",
        params![
            unit.text_unit_code,
            command.source_code,
            command.source_version,
            unit.document_code,
            command.scope.scope_type,
            command.scope.scope_code,
            unit.unit_type,
            unit.text,
            unit.search_text,
            unit.heading_path,
            unit.ordinal as i64,
            unit.token_count,
            unit.content_hash,
            unit.status,
            unit.fulltext_status,
            unit.embedding_model_code,
            unit.embedding_version,
            unit.embedding_text_hash,
            unit.embedding_dimension,
            unit.embedding,
            unit.embedding_status,
            json_string(&unit.metadata)?,
            now,
        ],
    )
    .map_err(storage_error)?;
    Ok(())
}

fn persist_warnings(
    tx: &Transaction<'_>,
    command: &RagPersistIngestCommand,
) -> Result<(), RagError> {
    let now = now_millis();
    for warning in &command.warnings {
        let record_code = format!(
            "{}:{}:{}:{}:{}:{}",
            command.corpus_id,
            command.profile_id,
            command.source_code,
            command.source_version,
            command.scope.scope_type,
            warning.stage_id
        );
        tx.execute(
            "INSERT INTO rag_diagnostics (
                record_type, record_code, corpus_id, profile_id, source_code, source_version,
                scope_type, scope_code, workspace_code, user_code, agent_code, stage_id,
                message, payload_json, created_at
            )
            VALUES ('INDEXING_WARNING', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT (record_code) DO UPDATE SET
                message = excluded.message,
                payload_json = excluded.payload_json,
                created_at = excluded.created_at",
            params![
                record_code,
                command.corpus_id,
                command.profile_id,
                command.source_code,
                command.source_version,
                command.scope.scope_type,
                command.scope.scope_code,
                command.workspace_code,
                command.user_code,
                command.agent_code,
                warning.stage_id,
                warning.message,
                json_string(&serde_json::json!({ "code": warning.code }))?,
                now,
            ],
        )
        .map_err(storage_error)?;
    }
    Ok(())
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
