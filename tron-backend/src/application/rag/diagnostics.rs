use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{Value, json};

use crate::domain::rag::{RagError, RagProfileType};

use super::{RagConfigRepository, RagStrategyRegistry};

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
pub struct RagDebugProfileResult {
    pub profile: Value,
    pub corpora: Vec<Value>,
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
    pub profile_id: String,
    pub workspace_code: String,
    pub user_code: String,
    pub agent_code: String,
    pub query_text: String,
    pub payload: Value,
    pub created_at: i64,
    pub expires_at: i64,
}

pub struct RagDiagnosticsService<R, S> {
    repository: R,
    _strategy_registry: S,
}

impl<R, S> RagDiagnosticsService<R, S>
where
    R: RagDiagnosticsRepository,
    S: RagStrategyRegistry,
{
    pub fn new(repository: R, strategy_registry: S) -> Self {
        Self {
            repository,
            _strategy_registry: strategy_registry,
        }
    }

    pub fn debug_profile(
        &self,
        workspace_code: &str,
        profile_id: &str,
    ) -> Result<RagDebugProfileResult, RagError> {
        if workspace_code.trim().is_empty() || profile_id.trim().is_empty() {
            return Err(RagError::DebugRequestInvalid);
        }
        let profile = self
            .repository
            .get_profile(profile_id)?
            .ok_or(RagError::ProfileNotFound)?;
        let corpora = self
            .repository
            .list_corpora()?
            .into_iter()
            .filter(|corpus| profile.corpus_ids.contains(&corpus.corpus_id))
            .map(project_corpus_for_debug)
            .collect::<Vec<_>>();
        Ok(RagDebugProfileResult {
            profile: json!({
                "profileId": profile.profile_id,
                "profileType": profile.profile_type.as_str(),
                "status": profile.status.as_str(),
                "primaryCorpusId": profile.primary_corpus_id,
                "corpusIds": profile.corpus_ids,
                "requiredSourceVersion": profile.required_source_version,
                "degradePolicy": profile.degrade_policy,
                "filters": profile.filters,
                "configSummary": summarize_profile_config(&profile.config, &profile.profile_type),
            }),
            corpora,
        })
    }

    pub fn debug_source(
        &self,
        request: RagDebugSourceRequest,
    ) -> Result<RagDebugSourceResult, RagError> {
        validate_debug_source_request(&request)?;
        self.repository
            .debug_source(&request)?
            .ok_or(RagError::SourceNotFound)
    }
}

fn validate_debug_source_request(request: &RagDebugSourceRequest) -> Result<(), RagError> {
    for value in [
        request.workspace_code.as_str(),
        request.corpus_id.as_str(),
        request.source_code.as_str(),
        request.source_version.as_str(),
        request.scope_type.as_str(),
        request.scope_code.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(RagError::DebugRequestInvalid);
        }
    }
    Ok(())
}

fn summarize_profile_config(config: &Value, profile_type: &RagProfileType) -> Value {
    let routes = config
        .get("routes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("routeId").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "profileType": profile_type.as_str(),
        "routes": routes,
        "contextStrategy": config
            .get("contextPolicy")
            .and_then(|value| value.get("strategy"))
            .and_then(Value::as_str),
    })
}

pub fn project_corpus_for_debug(corpus: crate::domain::rag::RagCorpusRecord) -> Value {
    json!({
        "corpusId": corpus.corpus_id,
        "status": corpus.status.as_str(),
        "diagnosticsTable": corpus.storage_binding.diagnostics_table,
    })
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
