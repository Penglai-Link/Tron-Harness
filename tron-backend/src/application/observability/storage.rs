//! Tron retention policy and read-model contracts; persistence uses Framework telemetry ports.
use super::{ObservabilityError, TraceRepository};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TRACE_RETENTION_DAYS: i64 = 15;
pub const METRIC_RETENTION_DAYS: i64 = 5;
pub const MAX_TRACE_RECORDS: usize = 50_000;
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MetricPoint {
    pub point_id: String,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub kind: String,
    pub temporality: i32,
    pub monotonic: bool,
    pub timestamp: i64,
    pub resource: BTreeMap<String, String>,
    pub scope: String,
    pub data: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MetricPage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<MetricPoint>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMetrics {
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub sampled_at: Option<i64>,
    pub metric_point_count: usize,
}
pub trait ObservabilityRepository: TraceRepository {
    fn metric_points(
        &self,
        name: Option<String>,
        current: usize,
        size: usize,
    ) -> Result<MetricPage, ObservabilityError>;
    fn runtime_metrics(&self) -> Result<RuntimeMetrics, ObservabilityError>;
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectorConnection {
    pub endpoint: String,
    pub authorization: String,
    pub trace_retention_days: i64,
    pub metric_retention_days: i64,
    pub max_trace_records: usize,
}
