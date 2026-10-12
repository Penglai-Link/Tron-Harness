use super::*;
use eliteforge_agent_framework::*;
use eliteforge_agent_test_support::TestBuilder;
use serde_json::{Value, json};
struct Model;
#[async_trait::async_trait]
impl ChatModel for Model {
    async fn generate(
        &self,
        _: &str,
        request: ChatRequest,
        _: &dyn ChatStream,
    ) -> Result<ChatResponse, ModelError> {
        let mut response = if request.messages.iter().any(|m| m.role == Role::Tool) {
            ChatResponse::text("private response")
        } else {
            ChatResponse::tool_calls(vec![ToolCall::new(
                "call",
                "lookup",
                json!({"secret":"private args"}),
            )])
        };
        response.usage = Usage {
            input_tokens: 17,
            output_tokens: 3,
        };
        Ok(response.with_provider_metrics(ProviderMetrics {
            cached_input_tokens: Some(4),
            reasoning_tokens: None,
        }))
    }
}
struct Lookup;
#[async_trait::async_trait]
impl Tool for Lookup {
    async fn call(&self, _: Value, _: &RunContext) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput::error("private failure"))
    }
}
#[tokio::test]
async fn observability_real_plugin_exports_parentage_links_metrics_and_survives_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("telemetry.sqlite3");
    let telemetry = LocalTelemetry::open(&path).await.unwrap();
    let agent = TestBuilder::default()
        .plugin(telemetry.plugin().unwrap())
        .plugin(ChatModelPlugin::new(Model))
        .plugin(ToolProviderPlugin::new(
            "tools",
            StaticToolProvider::new(vec![ToolBinding::new(
                ToolDescriptor::new("lookup", "lookup", "lookup", json!({"type":"object"})),
                Lookup,
            )]),
        ))
        .build()
        .await
        .unwrap();
    use opentelemetry::trace::{FutureExt, SpanContext, SpanId, TraceFlags, TraceId, TraceState};
    let browser = Context::new().with_remote_span_context(SpanContext::new(
        TraceId::from_hex("11111111111111111111111111111111").unwrap(),
        SpanId::from_hex("2222222222222222").unwrap(), TraceFlags::SAMPLED, true, TraceState::default(),
    ));
    let http = telemetry.begin_http("POST", "/api/v1/chats/sessions/test/messages/stream", &browser);
    let provider = telemetry.runtime.clone();
    let root = async move {
        spawn_traced(async move {
            let parent = Context::current();
            parent.with_span(provider.traces.tracer("tron").start_with_context("tron.session.run", &parent))
        }).await.unwrap()
    }.with_context(http.clone()).await;
    assert_eq!(root.span().span_context().trace_id(), browser.span().span_context().trace_id());
    let id = root.span().span_context().trace_id().to_string();
    telemetry
        .register(&id, "workspace-a", "session", "run", "main-agent")
        .unwrap();
    let mut request = RunRequest::new("session", "private prompt");
    request.extensions.insert(TraceParent(root.clone()));
    let guard = TraceGuard::tracked(root, telemetry.store.clone(), BTreeMap::new());
    let result = agent.run(request).await;
    assert!(result.is_ok());
    guard.finish(&result);
    drop(guard);
    let service = telemetry.service();
    http.span().end();
    telemetry.flush().await;
    let exported = telemetry.store.spans(id.clone()).await.unwrap();
    let server = exported.iter().find(|s| s.name.starts_with("POST /api/v1/")).unwrap();
    assert_eq!(server.parent_span_id.as_deref(), Some("2222222222222222"));
    let session = exported.iter().find(|s| s.name == "tron.session.run").unwrap();
    assert_eq!(session.parent_span_id.as_deref(), Some(server.span_id.as_str()));
    let detail = service.detail("workspace-a", &id).unwrap();
    assert_eq!(detail.trace.model_count, 2);
    assert_eq!(detail.trace.tool_count, 1);
    assert_eq!(detail.trace.input_tokens, 34);
    assert_eq!(detail.trace.cached_input_tokens, 8);
    assert_eq!(detail.trace.output_tokens, 6);
    assert_eq!(detail.trace.status, "Completed");
    assert_eq!(detail.trace.error_count, 1);
    let run = detail.spans.iter().find(|s| s.name == "agent.run").unwrap();
    let models = detail
        .spans
        .iter()
        .filter(|s| s.name == "agent.model")
        .collect::<Vec<_>>();
    assert!(
        models
            .iter()
            .all(|s| s.parent_span_id.as_deref() == Some(run.span_id.as_str()))
    );
    let tool = detail
        .spans
        .iter()
        .find(|s| s.name == "agent.tool")
        .unwrap();
    assert_eq!(tool.links.len(), 1);
    assert!(models.iter().any(|s| s.span_id == tool.links[0].span_id));
    let json = serde_json::to_string(&detail).unwrap();
    assert!(!json.contains("private"));
    assert!(matches!(
        service.detail("workspace-b", &id),
        Err(ObservabilityError::NotFound)
    ));
    let query = |workspace: &str, session: Option<&str>, trace: Option<&str>, current| TraceQuery { sorts: Vec::new(), sort_by: Default::default(), sort_direction: Default::default(),
        workspace_code: workspace.into(),
        session_code: session.map(str::to_owned),
        trace_id: trace.map(str::to_owned),
        current,
        size: 1,
    };
    assert_eq!(
        service
            .query(query(
                "workspace-a",
                Some("session"),
                Some(&id.to_uppercase()),
                1
            ))
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        service
            .query(query("workspace-b", None, None, 1))
            .unwrap()
            .total,
        0
    );
    let metrics = service
        .metrics(query("workspace-a", Some("session"), None, 1))
        .unwrap();
    assert_eq!(metrics.trace_count, 1);
    assert_eq!(metrics.input_tokens, 34);
    assert_eq!(metrics.output_tokens, 6);
    assert_eq!(
        service
            .metrics(query("workspace-b", None, None, 1))
            .unwrap()
            .input_tokens,
        0
    );
    assert_eq!(
        service
            .query(query("workspace-a", Some("missing"), None, 1))
            .unwrap()
            .total,
        0
    );
    assert!(
        service
            .query(query("workspace-a", None, None, 2))
            .unwrap()
            .records
            .is_empty()
    );
    assert!(matches!(
        service.query(query("workspace-a", None, Some("invalid"), 1)),
        Err(ObservabilityError::InvalidQuery)
    ));
    assert!(agent.framework().close().await.is_success());
    telemetry.close().await.unwrap();
    drop(service);
    drop(telemetry);
    let reopened = LocalTelemetry::open(&path).await.unwrap();
    assert_eq!(
        reopened
            .service()
            .detail("workspace-a", &id)
            .unwrap()
            .trace
            .input_tokens,
        34
    );
    reopened.close().await.unwrap();
}
#[tokio::test]
async fn observability_abandoned_host_trace_is_unknown_and_scope_is_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("traces.sqlite3");
    let telemetry = LocalTelemetry::open(&path).await.unwrap();
    let root = Context::new().with_span(
        telemetry
            .runtime
            .traces
            .tracer("tron")
            .start("tron.session.run"),
    );
    root.span()
        .set_attribute(KeyValue::new("agent.operation.outcome", "unknown"));
    let id = root.span().span_context().trace_id().to_string();
    telemetry
        .register(&id, "a", "s", "r", "main-agent")
        .unwrap();
    drop(TraceGuard::tracked(root, telemetry.store.clone(), BTreeMap::new()));
    telemetry.flush().await;
    assert_eq!(
        telemetry.service().detail("a", &id).unwrap().trace.status,
        "Unknown"
    );
    assert!(matches!(
        telemetry.service().detail("b", &id),
        Err(ObservabilityError::NotFound)
    ));
    telemetry.close().await.unwrap();
}

#[tokio::test]
async fn observability_collector_persists_sdk_metrics_and_real_process_samples() {
    use opentelemetry::metrics::MeterProvider;
    let temp = tempfile::tempdir().unwrap();
    let telemetry = LocalTelemetry::open(&temp.path().join("metrics.sqlite3"))
        .await
        .unwrap();
    let meter = telemetry.runtime.metrics.meter("collector-test");
    let counter = meter.u64_counter("test.requests").build();
    let histogram = meter.f64_histogram("test.duration").build();
    counter.add(23, &[]);
    histogram.record(42., &[]);
    tokio::time::sleep(Duration::from_millis(1200)).await;
    telemetry.flush().await;
    let store = telemetry.store.clone();
    let points = store
        .metric_points(Some("test.requests".into()), 0, 100)
        .await
        .unwrap();
    assert_eq!(points.total, 1);
    assert_eq!(points.records[0].kind, "Sum");
    assert_eq!(points.records[0].temporality, 2);
    assert!(points.records[0].monotonic);
    assert_eq!(points.records[0].data["asInt"], "23");
    let hist = store
        .metric_points(Some("test.duration".into()), 0, 100)
        .await
        .unwrap();
    assert_eq!(hist.records[0].kind, "Histogram");
    assert!(
        hist.records[0].data["bucketCounts"]
            .as_array()
            .unwrap()
            .len()
            > 1
    );
    assert_eq!(hist.records[0].data["count"], "1");
    let runtime = telemetry.service().0.runtime_metrics().unwrap();
    assert!(runtime.memory_bytes.unwrap() > 0);
    assert!(runtime.cpu_percent.unwrap() >= 0.);
    assert!(runtime.sampled_at.is_some());
    telemetry.close().await.unwrap();
}

#[tokio::test]
async fn observability_otlp_auth_validation_retries_and_profile_isolation() {
    use opentelemetry_proto::tonic::{
        collector::metrics::v1::ExportMetricsServiceRequest,
        metrics::v1::{
            Gauge, Metric, NumberDataPoint, ResourceMetrics, ScopeMetrics, metric::Data,
            number_data_point,
        },
    };
    use prost::Message;
    let temp = tempfile::tempdir().unwrap();
    let a = LocalTelemetry::open(&temp.path().join("a.sqlite3"))
        .await
        .unwrap();
    let b = LocalTelemetry::open(&temp.path().join("b.sqlite3"))
        .await
        .unwrap();
    let point = NumberDataPoint {
        time_unix_nano: now() as u64 * 1_000_000,
        value: Some(number_data_point::Value::AsDouble(3.5)),
        ..Default::default()
    };
    let request = ExportMetricsServiceRequest {
        resource_metrics: vec![ResourceMetrics {
            scope_metrics: vec![ScopeMetrics {
                metrics: vec![Metric {
                    name: "external.gauge".into(),
                    data: Some(Data::Gauge(Gauge {
                        data_points: vec![point],
                    })),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
    };
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("{}/v1/metrics", a.endpoint);
    let send = |authorization: String, body: Vec<u8>| {
        client
            .post(&url)
            .header("authorization", authorization)
            .header("content-type", "application/x-protobuf")
            .body(body)
            .send()
    };
    assert_eq!(
        send(b.authorization.clone(), request.encode_to_vec())
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        send(a.authorization.clone(), vec![255])
            .await
            .unwrap()
            .status(),
        400
    );
    for _ in 0..2 {
        assert_eq!(
            send(a.authorization.clone(), request.encode_to_vec())
                .await
                .unwrap()
                .status(),
            200
        );
    }
    assert_eq!(
        a.store
            .metric_points(Some("external.gauge".into()), 0, 15)
            .await
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        b.store
            .metric_points(Some("external.gauge".into()), 0, 15)
            .await
            .unwrap()
            .total,
        0
    );
    a.close().await.unwrap();
    b.close().await.unwrap();
}

#[tokio::test]
async fn agent_completion_log_uses_the_real_otel_span_identifiers() {
    use crate::{
        adapters::outbound::observability::logs::{daily_appender, query_log_files},
        application::observability::LogQuery,
    };
    use tracing_subscriber::prelude::*;
    let temp = tempfile::tempdir().unwrap();
    let telemetry = LocalTelemetry::open(&temp.path().join("logs.sqlite3"))
        .await
        .unwrap();
    let root = Context::new().with_span(
        telemetry
            .runtime
            .traces
            .tracer("tron")
            .start("tron.session.run"),
    );
    let trace_id = root.span().span_context().trace_id().to_string();
    let span_id = root.span().span_context().span_id().to_string();

    tracing::subscriber::with_default(
        tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_writer(daily_appender(temp.path(), 30).unwrap()),
        ),
        || {
            let guard = TraceGuard(root, None, std::sync::atomic::AtomicBool::new(false));
            guard.finish(&Err(
                eliteforge_agent_core::AgentError::InvalidConfiguration(
                    "private test failure".into(),
                ),
            ));
        },
    );
    let page = query_log_files(
        temp.path(),
        LogQuery {
            current: 1,
            size: 15,
            trace_id: Some(trace_id.clone()),
            span_id: Some(span_id.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.records[0].level, "Error");
    assert_eq!(page.records[0].trace_id, Some(trace_id));
    assert_eq!(page.records[0].span_id, Some(span_id));
    assert_eq!(page.records[0].message, "Agent run failed");
    assert!(!page.records[0].message.contains("private test failure"));
    telemetry.close().await.unwrap();
}

#[tokio::test]
async fn observability_platform_queries_include_all_workspaces_and_optional_filters() {
    use opentelemetry::metrics::MeterProvider;
    let temp = tempfile::tempdir().unwrap();
    let telemetry = LocalTelemetry::open(&temp.path().join("platform.sqlite3")).await.unwrap();
    let mut ids = Vec::new();
    for workspace in ["alpha", "beta"] {
        let root = Context::new().with_span(telemetry.runtime.traces.tracer("tron").start("tron.session.run"));
        let id = root.span().span_context().trace_id().to_string();
        telemetry.register(&id, workspace, "session", "run", "main-agent").unwrap();
        ids.push(id);
        drop(TraceGuard::tracked(root, telemetry.store.clone(), BTreeMap::new()));
    }
    let gauge = telemetry.runtime.metrics.meter("test").f64_gauge("workspace.load").build();
    for workspace in ["alpha", "beta"] { gauge.record(1., &[KeyValue::new("tron.workspace.code", workspace)]); }
    telemetry.flush().await;
    let service = telemetry.service();
    let query = |workspace: &str| TraceQuery { sorts: Vec::new(), sort_by: Default::default(), sort_direction: Default::default(), workspace_code: workspace.into(), session_code: None, trace_id: None, current: 1, size: 15 };
    assert_eq!(service.query(query("")).unwrap().total, 2);
    assert_eq!(service.metrics(query("")).unwrap().trace_count, 2);
    // Sorting runs before pagination across the full platform result set.
    let mut sort_query = query("");
    sort_query.sort_by = crate::application::observability::TraceSortField::StartedAt;
    sort_query.sort_direction = crate::application::observability::TraceSortDirection::Asc;
    sort_query.size = 1;
    let first = service.query(sort_query.clone()).unwrap();
    sort_query.current = 2;
    let second = service.query(sort_query.clone()).unwrap();
    assert_eq!(first.total, 2);
    assert_ne!(first.records[0].trace_id, second.records[0].trace_id);
    assert!(first.records[0].started_at <= second.records[0].started_at);
    sort_query.workspace_code = "alpha".into();
    sort_query.current = 1;
    assert_eq!(service.query(sort_query).unwrap().total, 1);

    assert_eq!(service.0.dashboard("").unwrap().trace_count, 2);
    let alpha_dashboard = service.0.dashboard("alpha").unwrap();
    assert_eq!(alpha_dashboard.trace_count, 1);
    assert_eq!(alpha_dashboard.running_trace_count, 0);
    assert_eq!(
        alpha_dashboard
            .recent_requests
            .iter()
            .map(|bucket| bucket.requests)
            .sum::<usize>(),
        1
    );
    for (workspace, id) in ["alpha", "beta"].iter().zip(ids.iter()) {
        let page = service.query(query(workspace)).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.records[0].workspace_code, *workspace);
        assert_eq!(service.detail("", id).unwrap().trace.workspace_code, *workspace);
        assert_eq!(service.metrics(query(workspace)).unwrap().trace_count, 1);
    }
    assert_eq!(service.0.scoped_metric_points(None, Some("workspace.load".into()), 1, 15).unwrap().total, 2);
    let points = service.0.scoped_metric_points(Some("alpha".into()), Some("workspace.load".into()), 1, 1).unwrap();
    assert_eq!(points.total, 1);
    assert_eq!(points.records.len(), 1);
    assert_eq!(service.0.scoped_metric_points(Some("beta".into()), Some("workspace.load".into()), 2, 1).unwrap().records.len(), 0);
    telemetry.close().await.unwrap();
}

#[tokio::test]
async fn background_operations_generate_fresh_server_roots_without_browser_context() {
    let temp = tempfile::tempdir().unwrap();
    let telemetry = LocalTelemetry::open(&temp.path().join("background.sqlite3")).await.unwrap();
    let first = telemetry.begin_operation("tron.automation.execute", &Context::new(), vec![]);
    let second = telemetry.begin_operation("tron.automation.execute", &Context::new(), vec![]);
    let first_id = first.0.span().span_context().trace_id().to_string();
    let second_id = second.0.span().span_context().trace_id().to_string();
    assert_ne!(first_id, second_id);
    first.complete("completed");
    second.complete("error");
    drop(first);
    drop(second);
    telemetry.flush().await;
    for id in [first_id, second_id] {
        let spans = telemetry.store.spans(id).await.unwrap();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].parent_span_id.is_none());
    }
    telemetry.close().await.unwrap();
}

struct ChildModel;
#[async_trait::async_trait]
impl ChatModel for ChildModel {
    async fn generate(&self, _: &str, _: ChatRequest, _: &dyn ChatStream) -> Result<ChatResponse, ModelError> {
        Ok(ChatResponse::text("child completed"))
    }
}
struct Delegate(Agent);
#[async_trait::async_trait]
impl Tool for Delegate {
    async fn call(&self, _: Value, parent: &RunContext) -> Result<ToolOutput, ToolError> {
        let caller = Context::current();
        let mut child_request = RunRequest::new("nested-child", "child");
        child_request.extensions.insert(TraceParent(caller.clone()));
        self.0.start_child_owned(child_request, parent).await
            .map_err(|e| ToolError::Failed(e.to_string()))?.wait().await
            .map_err(|e| ToolError::Failed(e.to_string()))?;
        let mut thread_request = RunRequest::new("nested-thread", "thread");
        thread_request.extensions.insert(TraceParent(caller));
        let agent = self.0.clone();
        spawn_traced(async move { agent.run(thread_request).await }).await
            .map_err(|e| ToolError::Failed(e.to_string()))?
            .map_err(|e| ToolError::Failed(e.to_string()))?;
        Ok(ToolOutput::text("delegated"))
    }
}
#[tokio::test]
async fn observability_child_and_independent_thread_runs_share_the_calling_tool_trace() {
    let temp = tempfile::tempdir().unwrap();
    let telemetry = LocalTelemetry::open(&temp.path().join("delegation.sqlite3")).await.unwrap();
    let child = TestBuilder::default().plugin(telemetry.plugin().unwrap())
        .plugin(ChatModelPlugin::new(ChildModel)).build().await.unwrap();
    let parent = TestBuilder::default().plugin(telemetry.plugin().unwrap())
        .plugin(ChatModelPlugin::new(Model))
        .plugin(ToolProviderPlugin::new("delegate", StaticToolProvider::new(vec![ToolBinding::new(
            ToolDescriptor::new("lookup", "lookup", "lookup", json!({"type":"object"})), Delegate(child.clone()))])))
        .build().await.unwrap();
    let guard = telemetry.begin_operation("test.root", &Context::new(), vec![]);
    let id = guard.0.span().span_context().trace_id().to_string();
    let mut request = RunRequest::new("parent", "delegate");
    request.extensions.insert(TraceParent(guard.0.clone()));
    parent.run(request).await.unwrap();
    guard.complete("completed");
    drop(guard);
    telemetry.flush().await;
    let spans = telemetry.store.spans(id).await.unwrap();
    let tool = spans.iter().find(|s| s.name == "agent.tool").unwrap();
    let delegated: Vec<_> = spans.iter().filter(|s| s.name == "agent.run" && s.parent_span_id.as_deref() == Some(tool.span_id.as_str())).collect();
    assert_eq!(delegated.len(), 2, "both native child and independent thread must inherit the caller tool span");
    assert_eq!(spans.iter().filter(|s| s.name == "agent.model").count(), 4);
    assert!(parent.framework().close().await.is_success());
    assert!(child.framework().close().await.is_success());
    telemetry.close().await.unwrap();
}
