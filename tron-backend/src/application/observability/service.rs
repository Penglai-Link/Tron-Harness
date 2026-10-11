use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TraceSummary {
    pub workspace_code: String,
    pub trace_id: String,
    pub session_code: String,
    pub session_name: Option<String>,
    pub run_code: String,
    pub agent_code: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub duration_ms: f64,
    pub status: String,
    pub span_count: usize,
    pub error_count: usize,
    pub model_count: usize,
    pub tool_count: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TraceSpan {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub started_at: i64,
    pub duration_ms: f64,
    pub status: String,
    pub attributes: BTreeMap<String, String>,
    pub events: Vec<SpanEvent>,
    pub links: Vec<SpanLink>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: i64,
    pub attributes: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpanLink {
    pub trace_id: String,
    pub span_id: String,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TracePage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<TraceSummary>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TraceDetail {
    pub trace: TraceSummary,
    pub spans: Vec<TraceSpan>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, utoipa::ToSchema)]
pub enum TraceSortField {
    #[default]
    StartedAt,
    Duration,
    Spans,
    Tokens,
    Errors,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, utoipa::ToSchema)]
pub enum TraceSortDirection {
    Asc,
    #[default]
    Desc,
}
#[derive(Clone, Copy, Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct TraceSort {
    pub field: TraceSortField,
    pub direction: TraceSortDirection,
}
#[derive(Clone, Debug)]
pub struct TraceQuery {
    pub sorts: Vec<TraceSort>,
    pub sort_by: TraceSortField,
    pub sort_direction: TraceSortDirection,
    pub workspace_code: String,
    pub session_code: Option<String>,
    pub trace_id: Option<String>,
    pub current: usize,
    pub size: usize,
}
#[derive(Clone, Copy, Debug)]
pub enum ObservabilityError {
    InvalidQuery,
    InvalidLogQuery,
    NotFound,
    Storage,
}
pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
impl ObservabilityError {
    pub fn descriptor(self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::InvalidLogQuery => ("101040004", "tron.observability.invalid_log_query"),
            Self::InvalidQuery => ("101040001", "tron.observability.invalid_query"),
            Self::NotFound => ("101040002", "tron.observability.not_found"),
            Self::Storage => ("101040003", "tron.observability.storage_failure"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TraceMetrics {
    pub trace_count: usize,
    pub failed_trace_count: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub average_duration_ms: f64,
    pub p95_duration_ms: f64,
}
pub trait TraceRepository: Send + Sync {
    fn metrics(&self, query: &TraceQuery) -> Result<TraceMetrics, ObservabilityError>;
    fn query(&self, query: &TraceQuery) -> Result<TracePage, ObservabilityError>;
    fn detail(&self, workspace: &str, trace: &str) -> Result<TraceDetail, ObservabilityError>;
}
#[derive(Clone)]
pub struct ObservabilityService(pub Arc<dyn super::ObservabilityRepository>);
impl ObservabilityService {
    pub fn query(&self, mut query: TraceQuery) -> Result<TracePage, ObservabilityError> {
        query.trace_id = query
            .trace_id
            .map(|id| id.trim().to_ascii_lowercase())
            .filter(|id| !id.is_empty());
        query.session_code = query
            .session_code
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty());
        if query.current == 0
            || !(1..=100).contains(&query.size)
            || query.current > 1_000_000
            || query.session_code.as_ref().is_some_and(|s| s.len() > 256)
            || query
                .trace_id
                .as_ref()
                .is_some_and(|id| !valid_trace_id(id))
        {
            return Err(ObservabilityError::InvalidQuery);
        }
        self.0.query(&query)
    }
    pub fn metrics(&self, mut query: TraceQuery) -> Result<TraceMetrics, ObservabilityError> {
        query.trace_id = query
            .trace_id
            .map(|id| id.trim().to_ascii_lowercase())
            .filter(|id| !id.is_empty());
        query.session_code = query
            .session_code
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty());
        if query.session_code.as_ref().is_some_and(|s| s.len() > 256)
            || query
                .trace_id
                .as_ref()
                .is_some_and(|id| !valid_trace_id(id))
        {
            return Err(ObservabilityError::InvalidQuery);
        }
        self.0.metrics(&query)
    }
    pub fn detail(&self, workspace: &str, trace: &str) -> Result<TraceDetail, ObservabilityError> {
        if !valid_trace_id(trace) {
            return Err(ObservabilityError::InvalidQuery);
        }
        self.0.detail(workspace, &trace.to_ascii_lowercase())
    }
}
fn valid_trace_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit()) && id.bytes().any(|c| c != b'0')
}
