use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use serde_json::json;
use tokio::{net::TcpListener, task::JoinHandle};
use tron_backend::{
    application::agent::{
        AgentRuntimeRegistry, ModelClient, RuntimeAdapterKey, RuntimeAdapterSelection,
        runtime_adapter_descriptors,
    },
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable,
        ModelRequest, ModelResponse, ToolRegistry,
    },
    shared::event_bus::EventBusHandle,
};

#[derive(Clone)]
struct NoopModelClient;

impl ModelClient for NoopModelClient {
    fn generate<'a>(
        &'a self,
        _config: &'a AgentModelConfig,
        _request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move { Err("noop model client".into()) })
    }
}

fn base_context() -> AgentRunContext {
    AgentRunContext {
        conversation_code: Some("CONV-1".into()),
        run_code: "RUN-1".into(),
        workspace_code: "TRON_CORE".into(),
        user_code: Some("USR-1".into()),
        locale: "en-US".into(),
        request_id: Some("REQ-1".into()),
        stream_key: Some("CONV-1".into()),
        agent_kind: AgentKind::SystemCli,
        system_agent_session_id: None,
        model: AgentModelConfig {
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "test-model".into(),
            thinking_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: Some(1.0),
            timeout_ms: Some(1000),
            max_retries: Some(0),
        },
        variables: vec![EffectiveVariable {
            key: "REGION".into(),
            value: "us".into(),
            source: "LOCAL".into(),
        }],
        skills: vec![EffectiveSkill {
            skill_name: "planner".into(),
            description: "plan".into(),
            instruction: Some("think".into()),
            tool_names: vec!["echo".into()],
        }],
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            domain_code: None,
            description: "echo".into(),
            input_schema: json!({ "type": "object" }),
            skill_name: Some("planner".into()),
            required_variable_keys: Vec::new(),
        }],
        tool_exposure: Default::default(),
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("hello")],
        observations: Vec::new(),
        started_at: "2026-05-30T00:00:00Z".into(),
    }
}

fn registry() -> AgentRuntimeRegistry {
    AgentRuntimeRegistry::new(
        Arc::new(NoopModelClient),
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
        reqwest::Client::new(),
    )
}

#[derive(Clone, Default)]
struct ServiceRuntimeMockState {
    cancelled: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

struct ServiceRuntimeMockServer {
    base_url: String,
    cancelled: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    _task: JoinHandle<()>,
}

async fn service_runtime_mock_server() -> ServiceRuntimeMockServer {
    let state = ServiceRuntimeMockState::default();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("service runtime mock server should bind");
    let address = listener
        .local_addr()
        .expect("service runtime mock server should expose address");
    let app = Router::new()
        .route("/runs/start", post(service_runtime_start))
        .route("/runs/resume", post(service_runtime_resume))
        .route("/runs/{run_code}/cancel", post(service_runtime_cancel))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("service runtime mock server should stay available");
    });
    ServiceRuntimeMockServer {
        base_url: format!("http://{address}"),
        cancelled: state.cancelled,
        requests: state.requests,
        _task: task,
    }
}

async fn service_runtime_start(
    State(state): State<ServiceRuntimeMockState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    state
        .requests
        .lock()
        .expect("service runtime requests mutex poisoned")
        .push(payload.clone());
    let context = payload.get("context").cloned().unwrap_or_else(|| json!({}));
    Json(json!({
        "runCode": context.get("runCode").and_then(|value| value.as_str()).unwrap_or("RUN-SVC-START"),
        "conversationCode": context.get("conversationCode"),
        "workspaceCode": context.get("workspaceCode").and_then(|value| value.as_str()).unwrap_or("TRON_CORE"),
        "finalOutput": "service start output",
        "stepCount": 1,
        "tokenUsage": {
            "inputTokens": 3,
            "outputTokens": 5,
            "totalTokens": 8
        },
        "systemAgentSessionId": "svc-start-session",
        "observations": []
    }))
}

async fn service_runtime_resume(
    State(state): State<ServiceRuntimeMockState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    state
        .requests
        .lock()
        .expect("service runtime requests mutex poisoned")
        .push(payload.clone());
    let context = payload.get("context").cloned().unwrap_or_else(|| json!({}));
    let requested_session = payload
        .get("requestedSessionId")
        .and_then(|value| value.as_str())
        .unwrap_or("svc-resume-session");
    Json(json!({
        "runCode": context.get("runCode").and_then(|value| value.as_str()).unwrap_or("RUN-SVC-RESUME"),
        "conversationCode": context.get("conversationCode"),
        "workspaceCode": context.get("workspaceCode").and_then(|value| value.as_str()).unwrap_or("TRON_CORE"),
        "finalOutput": "service resume output",
        "stepCount": 2,
        "tokenUsage": {
            "inputTokens": 4,
            "outputTokens": 6,
            "totalTokens": 10
        },
        "systemAgentSessionId": format!("{requested_session}-next"),
        "observations": []
    }))
}

async fn service_runtime_cancel(
    State(state): State<ServiceRuntimeMockState>,
    Path(run_code): Path<String>,
) -> Json<serde_json::Value> {
    state
        .cancelled
        .lock()
        .expect("service runtime cancelled mutex poisoned")
        .push(run_code);
    Json(json!({ "cancelled": true }))
}

#[test]
fn runtime_adapter_descriptors_cover_all_runtime_keys() {
    let descriptors = runtime_adapter_descriptors();
    assert_eq!(descriptors.len(), 6);
    assert!(
        descriptors
            .iter()
            .any(|item| item.key == RuntimeAdapterKey::React)
    );
    assert!(
        descriptors
            .iter()
            .any(|item| item.key == RuntimeAdapterKey::CodexExec)
    );
    assert!(
        descriptors
            .iter()
            .any(|item| item.key == RuntimeAdapterKey::CodexAppServer)
    );
    assert!(
        descriptors
            .iter()
            .any(|item| item.key == RuntimeAdapterKey::OpenCodeRun)
    );
    assert!(
        descriptors
            .iter()
            .any(|item| item.key == RuntimeAdapterKey::OpenCodeServe)
    );
    assert!(
        descriptors
            .iter()
            .any(|item| item.key == RuntimeAdapterKey::OpenCodeAcp)
    );
}

#[test]
fn runtime_adapter_key_from_code_parses_known_values() {
    assert_eq!(
        RuntimeAdapterKey::from_code("react"),
        Some(RuntimeAdapterKey::React)
    );
    assert_eq!(
        RuntimeAdapterKey::from_code("CODEX_APP_SERVER"),
        Some(RuntimeAdapterKey::CodexAppServer)
    );
    assert_eq!(
        RuntimeAdapterKey::from_code("opencode_acp"),
        Some(RuntimeAdapterKey::OpenCodeAcp)
    );
    assert_eq!(RuntimeAdapterKey::from_code("unknown"), None);
}

#[tokio::test]
async fn service_mode_runtime_factory_supports_start_resume_and_cancel() {
    let server = service_runtime_mock_server().await;
    let runtime = registry().resolve(RuntimeAdapterSelection::CodexAppServer {
        endpoint: server.base_url.clone(),
        working_directory: Some("/workspace".into()),
    });
    let start = runtime
        .start(base_context(), None)
        .await
        .expect("service runtime start should succeed");
    assert_eq!(start.final_output, "service start output");
    assert_eq!(
        start.system_agent_session_id.as_deref(),
        Some("svc-start-session")
    );

    let resume = runtime
        .resume("thr-1", base_context(), None)
        .await
        .expect("service runtime resume should succeed");
    assert_eq!(resume.final_output, "service resume output");
    assert_eq!(
        resume.system_agent_session_id.as_deref(),
        Some("thr-1-next")
    );

    runtime
        .cancel("RUN-1")
        .await
        .expect("service runtime cancel should succeed");
    let cancelled = server
        .cancelled
        .lock()
        .expect("service runtime cancelled mutex poisoned")
        .clone();
    assert_eq!(cancelled, vec!["RUN-1".to_string()]);

    let requests = server
        .requests
        .lock()
        .expect("service runtime requests mutex poisoned")
        .clone();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["action"], json!("start"));
    assert_eq!(requests[0]["workingDirectory"], json!("/workspace"));
    assert_eq!(requests[1]["action"], json!("resume"));
    assert_eq!(requests[1]["requestedSessionId"], json!("thr-1"));
    assert_eq!(
        requests[1]["context"]["systemAgentSessionId"],
        json!("thr-1")
    );
}

#[tokio::test]
async fn service_mode_runtime_factory_supports_all_service_adapters() {
    let server = service_runtime_mock_server().await;
    let selections = [
        (
            RuntimeAdapterKey::CodexAppServer,
            RuntimeAdapterSelection::CodexAppServer {
                endpoint: server.base_url.clone(),
                working_directory: None,
            },
        ),
        (
            RuntimeAdapterKey::OpenCodeServe,
            RuntimeAdapterSelection::OpenCodeServe {
                endpoint: server.base_url.clone(),
                working_directory: None,
            },
        ),
        (
            RuntimeAdapterKey::OpenCodeAcp,
            RuntimeAdapterSelection::OpenCodeAcp {
                endpoint: server.base_url.clone(),
                working_directory: None,
            },
        ),
    ];

    for (key, selection) in selections {
        let runtime = registry().resolve(selection);
        assert_eq!(runtime.key(), key);
        let run = runtime
            .start(base_context(), None)
            .await
            .expect("service-mode runtime should start successfully");
        assert_eq!(run.workspace_code, "TRON_CORE");
    }
}
