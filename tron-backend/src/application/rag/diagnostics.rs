use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;

use crate::domain::rag::RagError;

use super::RagConfigRepository;

pub trait RagDiagnosticsRepository: RagConfigRepository {
    fn debug_source(
        &self,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError>;
    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError>;
}

impl<T> RagDiagnosticsRepository for Arc<Mutex<T>>
where
    T: RagDiagnosticsRepository,
{
    fn debug_source(
        &self,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .debug_source(request)
    }

    fn persist_search_trace(&self, trace: RagSearchTraceWrite) -> Result<(), RagError> {
        self.lock()
            .expect("RAG repository mutex poisoned")
            .persist_search_trace(trace)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagDebugSourceRequest {
    pub workspace_code: String,
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub scope_type: String,
    pub scope_code: String,
    pub include_text_units: bool,
    pub include_warnings: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDebugSourceResult {
    pub source: RagDebugSourceRecord,
    pub text_units: Vec<RagDebugTextUnitRecord>,
    pub warnings: Vec<RagDiagnosticWarningRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDebugSourceRecord {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub scope_type: String,
    pub scope_code: String,
    pub status: String,
    pub title: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDebugTextUnitRecord {
    pub text_unit_code: String,
    pub ordinal: i64,
    pub status: String,
    pub fulltext_status: String,
    pub embedding_status: String,
    pub embedding_model_code: Option<String>,
    pub heading_path: String,
    pub token_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDiagnosticWarningRecord {
    pub record_code: String,
    pub stage_id: Option<String>,
    pub code: String,
    pub message: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagSearchTraceWrite {
    pub record_code: String,
    pub task_id: String,
    pub workspace_code: String,
    pub user_code: String,
    pub agent_code: String,
    pub query_text: String,
    pub payload: Value,
    pub created_at: i64,
    pub expires_at: i64,
}

pub fn safe_trace_payload(payload: &Value) -> Result<(), RagError> {
    let raw = payload.to_string().to_ascii_lowercase();
    for forbidden in [
        "contextpack",
        "embedding",
        "provider raw",
        "api_key",
        "sk-test",
    ] {
        if raw.contains(forbidden) {
            return Err(RagError::TracePayloadInvalid);
        }
    }
    Ok(())
}

/// Published source content, without embeddings or diagnostic internals.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSourceUnitsPage {
    pub items: Vec<RagSourceUnit>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSourceUnit {
    pub text_unit_code: String,
    pub ordinal: i64,
    pub text: String,
    pub heading_path: Vec<String>,
    pub metadata: Value,
}

/// Read-only inspection of a published embedded source generation.
pub trait RagSourceContentRepository {
    fn source_units_page(
        &self,
        request: RagDebugSourceRequest,
        page: usize,
        page_size: usize,
    ) -> Result<RagSourceUnitsPage, RagError>;
}
impl<T: RagSourceContentRepository> RagSourceContentRepository for Arc<Mutex<T>> {
    fn source_units_page(
        &self,
        request: RagDebugSourceRequest,
        page: usize,
        page_size: usize,
    ) -> Result<RagSourceUnitsPage, RagError> {
        self.lock()
            .map_err(|_| RagError::StorageFailure)?
            .source_units_page(request, page, page_size)
    }
}
