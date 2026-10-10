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
        Ok(response)
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
    let root = Context::new().with_span(
        telemetry
            .runtime
            .traces
            .tracer("tron")
            .start("tron.session.run"),
    );
    let id = root.span().span_context().trace_id().to_string();
    telemetry
        .register(&id, "workspace-a", "session", "run", "main-agent")
        .unwrap();
    let mut request = RunRequest::new("session", "private prompt");
    request.extensions.insert(TraceParent(root.clone()));
    let guard = TraceGuard(root);
    let result = agent.run(request).await;
    assert!(result.is_ok());
    guard.finish(&result);
    drop(guard);
    let service = telemetry.service();
    telemetry.flush().await;
    let detail = service.detail("workspace-a", &id).unwrap();
    assert_eq!(detail.trace.model_count, 2);
    assert_eq!(detail.trace.tool_count, 1);
    assert_eq!(detail.trace.input_tokens, 34);
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
    let query = |workspace: &str, session: Option<&str>, trace: Option<&str>, current| TraceQuery {
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
    drop(TraceGuard(root));
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
