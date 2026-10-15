//! Platform-wide, optionally workspace-scoped trace queries; profile middleware owns database identity.
use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::observability::{
        CollectorConnection, MetricPage, ObservabilityError, RuntimeMetrics, TraceDetail,
        TraceMetrics, TracePage, TraceQuery, TraceSummary, TraceSort, TraceSortField, TraceSortDirection,
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
    pub span_id: Option<String>,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub agent_code: Option<String>,
    #[serde(default)]
    pub keyword_aliases: Vec<String>,
    pub category: Option<crate::application::observability::TraceCategory>,
    #[serde(default)]
    pub sorts: Vec<TraceSort>,
    #[serde(default)]
    pub sort_by: TraceSortField,
    #[serde(default)]
    pub sort_direction: TraceSortDirection,
    pub workspace_code: Option<String>,
    pub session_code: Option<String>,
    pub trace_id: Option<String>,
    pub current: usize,
    pub size: usize,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTraceRequest {
    pub workspace_code: Option<String>,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMetricsRequest {
    pub workspace_code: Option<String>,
    pub session_code: Option<String>,
    pub trace_id: Option<String>,
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/observability/browser-spans", post(record_browser_span))
        .route("/api/v1/observability/logs/query", post(query_logs))
        .route("/api/v1/observability/traces/query", post(query_traces))
        .route("/api/v1/observability/traces/{trace_id}", get(get_trace))
        .route("/api/v1/observability/metrics", get(get_metrics))
        .route("/api/v1/observability/runtime", get(get_runtime))
        .route("/api/v1/observability/dashboard", get(get_dashboard))
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
        validate_workspace(&state, q.workspace_code.as_deref())?;
        let service = state.agent_framework.observability().await?;
        let workspace = q.workspace_code.clone().unwrap_or_default();
        let options_service = service.clone();
        let options_workspace=workspace.clone();
        let mut agent_options = if matches!(q.category,Some(crate::application::observability::TraceCategory::Agent)) {
            tokio::task::spawn_blocking(move || options_service.0.agent_options(&options_workspace)).await.map_err(|_| ObservabilityError::Storage)??
        } else {Vec::new()};
        let mut names=std::collections::BTreeMap::new();
        if !agent_options.is_empty() {
            let name_service=state.agent_management.clone();
            let name_workspace=workspace.clone();
            names=tokio::task::spawn_blocking(move || name_service.trace_agent_names(&[name_workspace])).await.map_err(|_| ObservabilityError::Storage)?;
            for option in &mut agent_options {
                option.name=names.get(&(workspace.clone(),option.code.clone())).cloned().unwrap_or_else(||option.code.clone());
            }
        }
        let keyword = q.keyword.unwrap_or_default().trim().to_lowercase();
        let mut aliases = q.keyword_aliases;
        if !keyword.is_empty() { aliases.extend(agent_options.iter().filter(|o| o.name.to_lowercase().contains(&keyword)).map(|o| o.code.clone())); }
        let query = TraceQuery { filters: crate::application::observability::TraceFilters {
            span_id: q.span_id, keyword, status:q.status.filter(|v| !v.is_empty()), agent_code:q.agent_code.filter(|v| !v.is_empty()), keyword_aliases:aliases,
        }, category: q.category,
            sorts: q.sorts,
            sort_by: q.sort_by,
            sort_direction: q.sort_direction,
            workspace_code: q.workspace_code.unwrap_or_default(),
            session_code: q.session_code,
            trace_id: q.trace_id,
            current: q.current,
            size: q.size,
        };
        let mut page = tokio::task::spawn_blocking(move || service.query(query))
            .await.map_err(|_| ObservabilityError::Storage)??;
        let extra_workspaces=page.records.iter().map(|row|row.workspace_code.clone()).filter(|code|code!=&workspace).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        if matches!(q.category,Some(crate::application::observability::TraceCategory::Agent)) && !extra_workspaces.is_empty() {
            let name_service=state.agent_management.clone();
            names.extend(tokio::task::spawn_blocking(move || name_service.trace_agent_names(&extra_workspaces)).await.map_err(|_| ObservabilityError::Storage)?);
        }
        let session_codes=page.records.iter().map(|row|row.session_code.clone()).collect::<Vec<_>>();
        let chats=state.chats.clone();
        let titles=tokio::task::spawn_blocking(move || chats.session_titles(&session_codes)).await.map_err(|_| ObservabilityError::Storage)?.unwrap_or_default().into_iter().map(|session|((session.workspace_code,session.session_code),session.title)).collect::<std::collections::BTreeMap<_,_>>();
        for trace in &mut page.records {
            if matches!(q.category,Some(crate::application::observability::TraceCategory::Agent)) {
                if let Some(name)=names.get(&(trace.workspace_code.clone(),trace.trigger.clone())) {trace.trigger=name.clone();}
            }
            trace.session_name=titles.get(&(trace.workspace_code.clone(),trace.session_code.clone())).cloned();
        }
        page.agent_options = agent_options;
        Ok(page)
    }
    .await;
    result
        .map(|value| Json(ApiResponse::protocol_success(value)))
        .map_err(|error| ObservabilityApiError(error, auth.locale))
}
#[utoipa::path(get, path="/api/v1/observability/traces/{trace_id}", tag="Observability",
    params(("trace_id"=String,Path),("workspaceCode"=Option<String>,Query)),
    responses((status=200,body=ApiResponse<crate::application::observability::TraceDetail>),(status=404,body=ApiResponse<EmptyData>)))]
pub async fn get_trace(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
    query: Result<Query<GetTraceRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<TraceDetail>>, ObservabilityApiError> {
    let result = async {
        let Query(q) = query.map_err(|_| ObservabilityError::InvalidQuery)?;
        validate_workspace(&state, q.workspace_code.as_deref())?;
        let mut detail = state
            .agent_framework
            .observability()
            .await?
            .detail(q.workspace_code.as_deref().unwrap_or_default(), &id)?;
        attach_session_name(&state, &mut detail.trace);
        let mut names = std::collections::BTreeMap::new();
        for span in &mut detail.spans {
            if let Some(code) = span.attributes.get("tron.agent.code").cloned() {
                let workspace = span.attributes.get("tron.workspace.code").cloned().unwrap_or_default();
                let name = names.entry((workspace.clone(), code.clone())).or_insert_with(|| state.agent_management.trace_agent_name(&workspace, &code).unwrap_or(code));
                span.attributes.insert("tron.agent.name".into(), name.clone());
            }
        }
        Ok(detail)
    }
    .await;
    result
        .map(|value| Json(ApiResponse::protocol_success(value)))
        .map_err(|error| ObservabilityApiError(error, auth.locale))
}
fn attach_session_name(state: &AppState, trace: &mut TraceSummary) {
    trace.session_name = state.chats.session_titles(std::slice::from_ref(&trace.session_code)).ok()
        .and_then(|sessions|sessions.into_iter().find(|session|session.workspace_code==trace.workspace_code))
        .map(|session|session.title);
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
        "tron.observability.invalid_log_query" => (
            StatusCode::BAD_REQUEST,
            "日志查询参数无效：Trace ID 为 32 位、Span ID 为 16 位非零十六进制字符",
            "Invalid log query: Trace ID must be 32 and Span ID 16 nonzero hexadecimal characters",
        ),
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
    params(("workspaceCode"=Option<String>,Query),("sessionCode"=Option<String>,Query),("traceId"=Option<String>,Query)),
    responses((status=200,body=ApiResponse<crate::application::observability::TraceMetrics>),(status=400,body=ApiResponse<EmptyData>)))]
pub async fn get_metrics(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    query: Result<Query<GetMetricsRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<TraceMetrics>>, ObservabilityApiError> {
    let result = async {
        let Query(q) = query.map_err(|_| ObservabilityError::InvalidQuery)?;
        validate_workspace(&state, q.workspace_code.as_deref())?;
        state
            .agent_framework
            .observability()
            .await?
            .metrics(TraceQuery { filters: Default::default(), category: None,
                sorts: Vec::new(),
                sort_by: TraceSortField::default(),
                sort_direction: TraceSortDirection::default(),
                workspace_code: q.workspace_code.unwrap_or_default(),
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
    pub workspace_code: Option<String>,
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
    validate_workspace(&state, q.workspace_code.as_deref()).map_err(|e| ObservabilityApiError(e, auth.locale.clone()))?;
    let service = state
        .agent_framework
        .observability()
        .await
        .map_err(|e| ObservabilityApiError(e, auth.locale.clone()))?;
    let result =
        tokio::task::spawn_blocking(move || service.0.scoped_metric_points(q.workspace_code, q.name, q.current, q.size))
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

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDashboardRequest {
    pub workspace_code: Option<String>,
}
#[utoipa::path(get,path="/api/v1/observability/dashboard",tag="Observability",
params(("workspaceCode"=Option<String>,Query)),responses((status=200,body=ApiResponse<crate::application::observability::ObservabilityDashboard>),(status=404,body=ApiResponse<EmptyData>)))]
pub async fn get_dashboard(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    query: Result<Query<GetDashboardRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<
    Json<ApiResponse<crate::application::observability::ObservabilityDashboard>>,
    ObservabilityApiError,
> {
    let result = async {
        let Query(q) = query.map_err(|_| ObservabilityError::InvalidQuery)?;
        validate_workspace(&state, q.workspace_code.as_deref())?;
        let service = state.agent_framework.observability().await?;
        tokio::task::spawn_blocking(move || service.0.dashboard(q.workspace_code.as_deref().unwrap_or_default()))
            .await
            .map_err(|_| ObservabilityError::Storage)?
    }
    .await;
    result
        .map(|data| Json(ApiResponse::protocol_success(data)))
        .map_err(|e| ObservabilityApiError(e, auth.locale))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryLogsRequest {
    pub workspace_code: Option<String>,
    pub before_log_id: Option<String>,
    pub after_log_id: Option<String>,
    pub date: Option<String>,
    pub time_zone: Option<String>,
    pub keyword: Option<String>,
    pub level: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub current: usize,
    pub size: usize,
}
#[utoipa::path(post,path="/api/v1/observability/logs/query",tag="Observability",request_body=QueryLogsRequest,
responses((status=200,body=ApiResponse<crate::application::observability::ApplicationLogPage>),(status=400,body=ApiResponse<EmptyData>)))]
pub async fn query_logs(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<QueryLogsRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<
    Json<ApiResponse<crate::application::observability::ApplicationLogPage>>,
    ObservabilityApiError,
> {
    use crate::{
        adapters::outbound::observability::logs::{log_directory, query_log_files},
        application::observability::LogQuery,
    };
    let Json(q) = request.map_err(|_| {
        ObservabilityApiError(ObservabilityError::InvalidLogQuery, auth.locale.clone())
    })?;
    validate_workspace(&state, q.workspace_code.as_deref()).map_err(|e| ObservabilityApiError(e, auth.locale.clone()))?;
    let directory = log_directory(&state.config);
    tokio::task::spawn_blocking(move || {
        query_log_files(
            &directory,
            LogQuery {
                workspace_code: q.workspace_code,
                date: q.date,
                time_zone: q.time_zone,
                keyword: q.keyword,
                level: q.level,
                trace_id: q.trace_id,
                span_id: q.span_id,
                before_log_id: q.before_log_id,
                after_log_id: q.after_log_id,
                current: q.current,
                size: q.size,
            },
        )
    })
    .await
    .map_err(|_| ObservabilityApiError(ObservabilityError::Storage, auth.locale.clone()))?
    .map(|data| Json(ApiResponse::protocol_success(data)))
    .map_err(|e| ObservabilityApiError(e, auth.locale))
}

#[derive(Clone, Copy, Deserialize, utoipa::ToSchema)]
enum BrowserSpanKind { Page, Resource, Request }

fn ignored_trace_path(path: &str) -> bool {
    path.starts_with("/api/v1/observability/")
        || (path.starts_with("/api/v1/agents/agui/streams/")
            && (path.ends_with("/protocol-events") || path.ends_with("/events")))
        || (path.starts_with("/api/v1/chats/sessions/")
            && (path.ends_with("/subagents") || path.contains("/subagents/") || path.ends_with("/history/query")))
        || matches!(path, "/api/v1/health" | "/api/v1/ready" | "/api/v1/readiness" | "/health" | "/ready" | "/api/v1/profiles/events" | "/api/v1/system/skins/events")
}

/// Record completed application API requests without credentials, query strings or polling noise.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserSpanRequest {
    kind: BrowserSpanKind,
    phase: Option<String>,
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    started_at: i64,
    ended_at: i64,
    method: String,
    path: String,
    status: u16,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserSpansRequest {
    spans: Vec<BrowserSpanRequest>,
}

#[utoipa::path(post, path="/api/v1/observability/browser-spans", tag="Observability",
    request_body=BrowserSpansRequest, responses((status=200,body=ApiResponse<EmptyData>),(status=400,body=ApiResponse<EmptyData>)))]
pub async fn record_browser_span(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<BrowserSpansRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<EmptyData>>, ObservabilityApiError> {
    let invalid = || ObservabilityApiError(ObservabilityError::InvalidQuery, auth.locale.clone());
    let Json(batch) = request.map_err(|_| invalid())?;
    if batch.spans.is_empty() || batch.spans.len() > 100 { return Err(invalid()); }
    let mut spans = Vec::with_capacity(batch.spans.len());
    for q in batch.spans {
    let valid_id = |id: &str, len: usize| id.len() == len
        && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && id.bytes().any(|b| b != b'0');
    if !valid_id(&q.trace_id, 32) || !valid_id(&q.span_id, 16)
        || q.parent_span_id.as_deref().is_some_and(|id| !valid_id(id, 16))
        || q.started_at <= 0 || q.ended_at < q.started_at
        || q.ended_at - q.started_at > 86_400_000
        || q.path.len() > 2048 || !q.path.starts_with('/') || q.path.starts_with("//")
        || q.phase.as_ref().is_some_and(|phase| phase.len() > 64)
        || (matches!(q.kind, BrowserSpanKind::Request) && !q.path.starts_with("/api/v1/"))
        || q.path.contains(['?', '#']) || q.method.len() > 16
        || q.method.parse::<axum::http::Method>().is_err() || q.status > 599 {
        return Err(invalid());
    }
    if ignored_trace_path(&q.path) { continue; }
    let (scope, name, span_kind) = match q.kind {
        BrowserSpanKind::Page => ("tron.browser.page", format!("page {}", q.path), 1),
        BrowserSpanKind::Resource => ("tron.browser.resource", format!("resource {}", q.path), 3),
        BrowserSpanKind::Request => ("tron.browser.fetch", format!("{} {}", q.method, q.path), 3),
    };
    let mut attributes = std::collections::BTreeMap::from([
        ("tron.trace.page".into(), "true".into()),
        ("service.name".into(), "tron-frontend".into()),
        ("http.request.method".into(), q.method.clone()),
        ("url.path".into(), q.path.clone()),
        ("http.response.status_code".into(), q.status.to_string()),
    ]);
    if let Some(phase) = q.phase { attributes.insert("browser.phase".into(), phase); }
    spans.push(eliteforge_agent_core::TelemetrySpan {
        trace_id: q.trace_id, span_id: q.span_id, parent_span_id: q.parent_span_id,
        name, started_at: q.started_at, ended_at: q.ended_at,
        status_code: if (q.status == 0 && !matches!(q.kind, BrowserSpanKind::Resource)) || q.status >= 400 { 2 } else { 0 }, attributes,
        data: serde_json::json!({"durationMs":q.ended_at-q.started_at,"span":{"kind":span_kind},"scope":{"name":scope}}),
        ..Default::default()
    });
    }
    let telemetry = state.agent_framework.telemetry().await
        .map_err(|e| ObservabilityApiError(e, auth.locale.clone()))?;
    telemetry.store.write_spans(spans).await.map_err(|_| ObservabilityApiError(ObservabilityError::Storage, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}

/// Accept W3C version 00 context only; malformed or zero IDs start a new trace.
fn request_trace_context(headers: &axum::http::HeaderMap) -> opentelemetry::Context {
    use opentelemetry::trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState};
    let parse = || {
        let value = headers.get("traceparent")?.to_str().ok()?;
        let parts: Vec<_> = value.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" || parts[1].len() != 32
            || parts[2].len() != 16 || parts[3].len() != 2
            || !parts[1..].iter().all(|p| p.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))) {
            return None;
        }
        let trace = TraceId::from_hex(parts[1]).ok()?;
        let span = SpanId::from_hex(parts[2]).ok()?;
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        let state = headers.get("tracestate").and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<TraceState>().ok()).unwrap_or_default();
        let context = SpanContext::new(trace, span, TraceFlags::new(flags), true, state);
        context.is_valid().then_some(context)
    };
    match parse() {
        Some(context) => opentelemetry::Context::new().with_remote_span_context(context),
        None => opentelemetry::Context::new(),
    }
}

pub async fn log_application_request(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    use opentelemetry::{KeyValue, trace::{FutureExt, TraceContextExt, Status}};
    use tracing::Instrument;
    let path = request.uri().path().to_owned();
    if !path.starts_with("/api/v1/") || ignored_trace_path(&path) {
        return next.run(request).with_context(opentelemetry::Context::new()).await;
    }
    let method = request.method().to_string();
    let parent = request_trace_context(request.headers());
    let context = match state.agent_framework.telemetry().await {
        Ok(telemetry) => telemetry.begin_http(&method, &path, &parent),
        Err(error) => {
            tracing::error!(?error, "HTTP tracing unavailable");
            return next.run(request).await;
        }
    };
    let ids = context.span().span_context().clone();
    let log_span = tracing::info_span!("http.request", trace_id=%ids.trace_id(), span_id=%ids.span_id());
    let started = std::time::Instant::now();
    let business = {
        let _guard = context.clone().attach();
        crate::app::telemetry::child_operation("tron.business.request", vec![
            KeyValue::new("http.request.method", method.clone()),
            KeyValue::new("url.path", path.clone()),
        ])
    };
    let mut response = next.run(request).instrument(log_span).with_context(business.clone()).await;
    if response.status().is_server_error() { business.span().set_status(Status::error("Business request failed")); }
    business.span().end();
    let status = response.status().as_u16();
    context.span().set_attribute(KeyValue::new("http.response.status_code", i64::from(status)));
    if status >= 500 { context.span().set_status(Status::error("HTTP server error")); }
    let duration_ms = started.elapsed().as_millis() as u64;
    // Querying telemetry must not continuously generate more application logs.
    if !path.starts_with("/api/v1/observability/") {
        if status >= 500 {
            tracing::error!(trace_id=%ids.trace_id(), span_id=%ids.span_id(), method, path, status, duration_ms, "HTTP request completed");
        } else if status >= 400 {
            tracing::warn!(trace_id=%ids.trace_id(), span_id=%ids.span_id(), method, path, status, duration_ms, "HTTP request completed");
        } else {
            tracing::info!(trace_id=%ids.trace_id(), span_id=%ids.span_id(), method, path, status, duration_ms, "HTTP request completed");
        }
    }
    if let Ok(value) = format!("00-{}-{}-{:02x}", ids.trace_id(), ids.span_id(), ids.trace_flags().to_u8()).parse() {
        response.headers_mut().insert("traceparent", value);
    }
    context.span().end();
    response
}

// The selected Profile still owns storage; absence of a workspace only widens within it.
fn validate_workspace(state: &AppState, workspace: Option<&str>) -> Result<(), ObservabilityError> {
    if let Some(code) = workspace.filter(|code| !code.is_empty()) {
        state.workspaces.get_workspace(code).map_err(|_| ObservabilityError::NotFound)?;
    }
    Ok(())
}

#[cfg(test)]
mod http_trace_tests {
    use super::*;
    use opentelemetry::trace::TraceContextExt;
    #[test]
    fn ignores_chat_polling_but_keeps_agent_requests() {
        for path in [
            "/api/v1/agents/agui/streams/session/protocol-events",
            "/api/v1/chats/sessions/session/subagents",
            "/api/v1/chats/sessions/session/history/query",
        ] { assert!(ignored_trace_path(path)); }
        assert!(!ignored_trace_path("/api/v1/chats/sessions/session/messages/stream"));
    }
    #[test]
    fn accepts_valid_w3c_context_and_rejects_invalid_ids() {
        for value in [
            "00-11111111111111111111111111111111-2222222222222222-01",
            "00-11111111111111111111111111111111-2222222222222222-00",
        ] {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("traceparent", value.parse().unwrap());
            let context = request_trace_context(&headers);
            assert_eq!(context.span().span_context().trace_id().to_string(), "11111111111111111111111111111111");
            assert!(context.span().span_context().is_remote());
        }
        for value in [
            "00-00000000000000000000000000000000-2222222222222222-01",
            "00-11111111111111111111111111111111-0000000000000000-01",
            "ff-11111111111111111111111111111111-2222222222222222-01",
            "invalid",
        ] {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("traceparent", value.parse().unwrap());
            assert!(!request_trace_context(&headers).span().span_context().is_valid());
        }
    }
}
