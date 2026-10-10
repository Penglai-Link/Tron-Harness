//! Workspace-scoped trace queries; profile middleware owns database identity.
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::observability::{
        CollectorConnection, MetricPage, ObservabilityError, RuntimeMetrics, TraceDetail,
        TraceMetrics, TracePage, TraceQuery,
    },
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryTracesRequest {
    pub workspace_code: String,
    pub session_code: Option<String>,
    pub trace_id: Option<String>,
    pub current: usize,
    pub size: usize,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTraceRequest {
    pub workspace_code: String,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMetricsRequest {
    pub workspace_code: String,
    pub session_code: Option<String>,
    pub trace_id: Option<String>,
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/observability/traces/query", post(query_traces))
        .route("/api/v1/observability/traces/{trace_id}", get(get_trace))
        .route("/api/v1/observability/metrics", get(get_metrics))
        .route("/api/v1/observability/runtime", get(get_runtime))
        .route(
            "/api/v1/observability/metric-points/query",
            post(query_metric_points),
        )
        .route(
            "/api/v1/observability/collector/export-config",
            post(get_export_config).layer(axum::middleware::map_response(
                |mut response: Response| async move {
                    response.headers_mut().insert(
                        axum::http::header::CACHE_CONTROL,
                        axum::http::HeaderValue::from_static("no-store"),
                    );
                    response
                },
            )),
        )
}
#[utoipa::path(post, path="/api/v1/observability/traces/query", tag="Observability",
    request_body=QueryTracesRequest, responses((status=200,body=ApiResponse<crate::application::observability::TracePage>),(status=400,body=ApiResponse<EmptyData>)))]
pub async fn query_traces(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<QueryTracesRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<TracePage>>, ObservabilityApiError> {
    let result = async {
        let Json(q) = request.map_err(|_| ObservabilityError::InvalidQuery)?;
        state
            .workspaces
            .get_workspace(&q.workspace_code)
            .map_err(|_| ObservabilityError::NotFound)?;
        state
            .agent_framework
            .observability()
            .await?
            .query(TraceQuery {
                workspace_code: q.workspace_code,
                session_code: q.session_code,
                trace_id: q.trace_id,
                current: q.current,
                size: q.size,
            })
    }
    .await;
    result
        .map(|value| Json(ApiResponse::protocol_success(value)))
        .map_err(|error| ObservabilityApiError(error, auth.locale))
}
#[utoipa::path(get, path="/api/v1/observability/traces/{trace_id}", tag="Observability",
    params(("trace_id"=String,Path),("workspaceCode"=String,Query)),
    responses((status=200,body=ApiResponse<crate::application::observability::TraceDetail>),(status=404,body=ApiResponse<EmptyData>)))]
pub async fn get_trace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    query: Result<Query<GetTraceRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<TraceDetail>>, ObservabilityApiError> {
    let result = async {
        let Query(q) = query.map_err(|_| ObservabilityError::InvalidQuery)?;
        state
            .workspaces
            .get_workspace(&q.workspace_code)
            .map_err(|_| ObservabilityError::NotFound)?;
        state
            .agent_framework
            .observability()
            .await?
            .detail(&q.workspace_code, &id)
    }
    .await;
    result
        .map(|value| Json(ApiResponse::protocol_success(value)))
        .map_err(|error| ObservabilityApiError(error, auth.locale))
}
pub struct ObservabilityApiError(ObservabilityError, String);
impl IntoResponse for ObservabilityApiError {
    fn into_response(self) -> Response {
        error_response(self.0, &self.1)
    }
}
fn error_response(error: ObservabilityError, locale: &str) -> Response {
    let descriptor = error.descriptor();
    let (status, zh, en) = match descriptor.message_key {
        "tron.observability.invalid_query" => (
            StatusCode::BAD_REQUEST,
            "查询参数无效，Trace ID 必须是 32 位十六进制字符",
            "Invalid query; Trace ID must contain 32 hexadecimal characters",
        ),
        "tron.observability.not_found" => (StatusCode::NOT_FOUND, "链路不存在", "Trace not found"),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "读取链路失败",
            "Could not read traces",
        ),
    };
    (
        status,
        Json(ApiResponse::<EmptyData>::error(
            descriptor.code,
            if locale.starts_with("en") { en } else { zh },
        )),
    )
        .into_response()
}

#[utoipa::path(get, path="/api/v1/observability/metrics", tag="Observability",
    params(("workspaceCode"=String,Query),("sessionCode"=Option<String>,Query),("traceId"=Option<String>,Query)),
    responses((status=200,body=ApiResponse<crate::application::observability::TraceMetrics>),(status=400,body=ApiResponse<EmptyData>)))]
pub async fn get_metrics(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    query: Result<Query<GetMetricsRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<TraceMetrics>>, ObservabilityApiError> {
    let result = async {
        let Query(q) = query.map_err(|_| ObservabilityError::InvalidQuery)?;
        state
            .workspaces
            .get_workspace(&q.workspace_code)
            .map_err(|_| ObservabilityError::NotFound)?;
        state
            .agent_framework
            .observability()
            .await?
            .metrics(TraceQuery {
                workspace_code: q.workspace_code,
                session_code: q.session_code,
                trace_id: q.trace_id,
                current: 1,
                size: 1,
            })
    }
    .await;
    result
        .map(|value| Json(ApiResponse::protocol_success(value)))
        .map_err(|error| ObservabilityApiError(error, auth.locale))
}

#[utoipa::path(get,path="/api/v1/observability/runtime",tag="Observability",responses((status=200,body=ApiResponse<RuntimeMetrics>)))]
pub async fn get_runtime(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<RuntimeMetrics>>, ObservabilityApiError> {
    let service = state
        .agent_framework
        .observability()
        .await
        .map_err(|e| ObservabilityApiError(e, auth.locale.clone()))?;
    let result = tokio::task::spawn_blocking(move || service.0.runtime_metrics())
        .await
        .map_err(|_| ObservabilityApiError(ObservabilityError::Storage, auth.locale.clone()))?;
    result
        .map(|data| Json(ApiResponse::protocol_success(data)))
        .map_err(|e| ObservabilityApiError(e, auth.locale))
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryMetricPointsRequest {
    pub name: Option<String>,
    pub current: usize,
    pub size: usize,
}
#[utoipa::path(post,path="/api/v1/observability/metric-points/query",tag="Observability",request_body=QueryMetricPointsRequest,responses((status=200,body=ApiResponse<MetricPage>),(status=400,body=ApiResponse<EmptyData>)))]
pub async fn query_metric_points(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<QueryMetricPointsRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<MetricPage>>, ObservabilityApiError> {
    let Json(q) = request.map_err(|_| {
        ObservabilityApiError(ObservabilityError::InvalidQuery, auth.locale.clone())
    })?;
    let service = state
        .agent_framework
        .observability()
        .await
        .map_err(|e| ObservabilityApiError(e, auth.locale.clone()))?;
    let result =
        tokio::task::spawn_blocking(move || service.0.metric_points(q.name, q.current, q.size))
            .await
            .map_err(|_| ObservabilityApiError(ObservabilityError::Storage, auth.locale.clone()))?;
    result
        .map(|data| Json(ApiResponse::protocol_success(data)))
        .map_err(|e| ObservabilityApiError(e, auth.locale))
}
#[utoipa::path(post,path="/api/v1/observability/collector/export-config",tag="Observability",responses((status=200,body=ApiResponse<CollectorConnection>)))]
pub async fn get_export_config(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<CollectorConnection>>, ObservabilityApiError> {
    state
        .agent_framework
        .collector_connection()
        .await
        .map(|data| Json(ApiResponse::protocol_success(data)))
        .map_err(|e| ObservabilityApiError(e, auth.locale))
}
