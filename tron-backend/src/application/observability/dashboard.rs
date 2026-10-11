use super::ObservabilityError;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestBucket {
    pub started_at: i64,
    pub requests: usize,
    pub errors: usize,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProcess {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub memory_bytes: u64,
    pub started_at: i64,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityDashboard {
    pub sampled_at: i64,
    pub trace_count: usize,
    pub failed_trace_count: usize,
    pub running_trace_count: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub average_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub recent_requests: Vec<RequestBucket>,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub resource_sampled_at: Option<i64>,
    pub metric_point_count: usize,
    pub processes: Vec<ServiceProcess>,
}
pub trait DashboardRepository: Send + Sync {
    fn dashboard(&self, workspace: &str) -> Result<ObservabilityDashboard, ObservabilityError>;
}
