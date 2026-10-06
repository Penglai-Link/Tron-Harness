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
use tokio::{
    net::TcpListener,
    task::JoinHandle,
    time::{Duration, sleep},
};
use tron_backend::{
    application::agent::{
        AgentInvocationError, AgentRunExecutor, AgentRuntimeRegistry, ModelClient,
        RuntimeAdapterKey, RuntimeAdapterSelection, runtime_adapter_descriptors,
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

    // Test adapters explicitly declare one transport attempt per mock invocation.
    fn generate_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        self.generate(config, request)
    }

    fn close_observed<'a>(
        &'a self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }

    fn close<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}

fn base_context() -> AgentRunContext {
    AgentRunContext {
        agent_code: None,
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
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: Some(1.0),
            max_retries: Some(0),
        },
        variables: vec![EffectiveVariable {
            key: "REGION".into(),
            value: "us".into(),
            source: "LOCAL".into(),
            sensitive: false,
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
        effective_tools: Vec::new(),
        tool_exposure: Default::default(),
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("hello")],
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-05-30T00:00:00Z".into(),
        auto_model_route: None,
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
    cancel_requests: Arc<Mutex<Vec<serde_json::Value>>>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    start_delay_ms: u64,
}

struct ServiceRuntimeMockServer {
    base_url: String,
    cancelled: Arc<Mutex<Vec<String>>>,
    cancel_requests: Arc<Mutex<Vec<serde_json::Value>>>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    _task: JoinHandle<()>,
}

async fn service_runtime_mock_server() -> ServiceRuntimeMockServer {
    service_runtime_mock_server_with_start_delay(0).await
}

async fn service_runtime_mock_server_with_start_delay(
    start_delay_ms: u64,
) -> ServiceRuntimeMockServer {
    let state = ServiceRuntimeMockState {
        start_delay_ms,
        ..ServiceRuntimeMockState::default()
    };
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
        cancel_requests: state.cancel_requests,
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
    if state.start_delay_ms > 0 {
        sleep(Duration::from_millis(state.start_delay_ms)).await;
    }
    let context = payload.get("context").cloned().unwrap_or_else(|| json!({}));
    Json(json!({
        "runCode": context.get("runCode").and_then(|value| value.as_str()).unwrap_or("RUN-SVC-START"),
        "conversationCode": context.get("conversationCode"),
        "workspaceCode": context.get("workspaceCode").and_then(|value| value.as_str()).unwrap_or("TRON_CORE"),
        "finalOutput": if context["runCode"] == "RUN-FLOOD" { "x".repeat(72 * 2_000) } else { "service start output".into() },
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
        "systemAgentSessionId": if context["runCode"] == "RUN-STABLE" { requested_session.to_owned() } else { format!("{requested_session}-next") },
        "observations": []
    }))
}

async fn service_runtime_cancel(
    State(state): State<ServiceRuntimeMockState>,
    Path(run_code): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    state
        .cancelled
        .lock()
        .expect("service runtime cancelled mutex poisoned")
        .push(run_code);
    state
        .cancel_requests
        .lock()
        .expect("service runtime cancel requests mutex poisoned")
        .push(payload);
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
    let cancel_requests = server
        .cancel_requests
        .lock()
        .expect("service runtime cancel requests mutex poisoned")
        .clone();
    assert_eq!(cancel_requests, vec![json!({ "runCode": "RUN-1" })]);

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
async fn executor_stop_cancels_in_flight_service_runtime_by_run_code() {
    let server = service_runtime_mock_server_with_start_delay(2_000).await;
    let runtime_registry = registry();
    let active_runs = runtime_registry.active_runs();
    let executor = AgentRunExecutor::new(runtime_registry);
    let runtime = executor.resolve_runtime(RuntimeAdapterSelection::OpenCodeServe {
        endpoint: server.base_url.clone(),
        working_directory: None,
    });
    let execution = tokio::spawn(async move {
        executor
            .execute_with_runtime(runtime, base_context(), None, None)
            .await
    });

    for _ in 0..100 {
        if !server
            .requests
            .lock()
            .expect("service runtime requests mutex poisoned")
            .is_empty()
        {
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !server
            .requests
            .lock()
            .expect("service runtime requests mutex poisoned")
            .is_empty(),
        "service runtime start request should be in flight before stop"
    );

    active_runs
        .request_stop("CONV-1", "RUN-1")
        .expect("matching active service run should stop");
    let error = tokio::time::timeout(Duration::from_secs(1), execution)
        .await
        .expect("local execution should stop after remote cancel")
        .expect("service execution task should join")
        .expect_err("stopped service execution should not complete");
    assert_eq!(error, AgentInvocationError::RunStopped);
    assert_eq!(
        server
            .cancelled
            .lock()
            .expect("service runtime cancelled mutex poisoned")
            .as_slice(),
        ["RUN-1"]
    );
    assert_eq!(
        server
            .cancel_requests
            .lock()
            .expect("service runtime cancel requests mutex poisoned")
            .as_slice(),
        [json!({ "runCode": "RUN-1" })]
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

#[tokio::test]
async fn service_mode_runtime_rejects_sensitive_variables_before_posting() {
    let server = service_runtime_mock_server().await;
    let runtime = registry().resolve(RuntimeAdapterSelection::CodexAppServer {
        endpoint: server.base_url.clone(),
        working_directory: Some("/workspace".into()),
    });
    let mut context = base_context();
    context.variables = vec![EffectiveVariable {
        key: "VSD_RUNTIME_TOKEN".into(),
        value: "runtime-sensitive-value".into(),
        source: "PLATFORM".into(),
        sensitive: true,
    }];

    let error = runtime
        .start(context, None)
        .await
        .expect_err("AC-VSD-06 RED: untrusted service mode must reject sensitive variables");
    assert_eq!(error.code(), "VARIABLE_RUNTIME_SECRET_DELIVERY_UNTRUSTED");

    let requests = server
        .requests
        .lock()
        .expect("service runtime requests mutex poisoned")
        .clone();
    assert!(
        requests.is_empty(),
        "AC-VSD-06 RED: sensitive context must fail before external POST"
    );
}

#[tokio::test]
async fn service_adapters_restore_sessions_from_framework_after_host_restart() {
    use tron_backend::application::agent::FrameworkHost;
    let server = service_runtime_mock_server().await;
    let directory = tempfile::tempdir().unwrap();
    for (index, selection) in [
        RuntimeAdapterSelection::CodexAppServer {
            endpoint: server.base_url.clone(),
            working_directory: None,
        },
        RuntimeAdapterSelection::OpenCodeServe {
            endpoint: server.base_url.clone(),
            working_directory: None,
        },
        RuntimeAdapterSelection::OpenCodeAcp {
            endpoint: server.base_url.clone(),
            working_directory: None,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let path = directory.path().join(format!("runtime-{index}.sqlite3"));
        let host = FrameworkHost::persistent(&path, None);
        let runtime = registry()
            .with_framework_host(Some(host.clone()))
            .resolve(selection.clone());
        runtime.start(base_context(), None).await.unwrap();
        host.close().await;
        drop(runtime);
        drop(host);
        let host = FrameworkHost::persistent(&path, None);
        let runtime = registry()
            .with_framework_host(Some(host.clone()))
            .resolve(selection);
        let mut context = base_context();
        context.run_code = "RUN-RESTARTED".into();
        context.messages = vec![AgentMessage::user("continue after restart")];
        let result = runtime.start(context.clone(), None).await.unwrap();
        assert_eq!(result.final_output, "service resume output");
        assert_eq!(
            server.requests.lock().unwrap().last().unwrap()["requestedSessionId"],
            "svc-start-session"
        );
        assert!(!runtime.capabilities().supports_streaming);
        // Product runtime mappings supply explicit IDs; they cannot bypass known ownership.
        context.system_agent_session_id = Some("svc-start-session".into());
        context.model.api_key = "different-account".into();
        runtime.start(context.clone(), None).await.unwrap();
        assert_eq!(
            server.requests.lock().unwrap().last().unwrap()["action"],
            "start"
        );
        // The same native string may be allocated by both accounts; the original binding
        // still cannot be resumed for a third account.
        context.model.api_key = "third-account".into();
        assert!(
            runtime
                .resume("svc-start-session", context.clone(), None)
                .await
                .is_err()
        );
        // Reusing a chat ID under another owner cannot reuse that remote session.
        context.user_code = Some("ANOTHER-USER".into());
        context.run_code = "RUN-OTHER-USER".into();
        runtime.start(context, None).await.unwrap();
        assert_eq!(
            server.requests.lock().unwrap().last().unwrap()["action"],
            "start"
        );
        host.close().await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn cli_adapters_restore_native_sessions_after_host_restart() {
    use std::os::unix::fs::PermissionsExt;
    use tron_backend::application::agent::FrameworkHost;
    for codex in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("agent");
        let log = directory.path().join("argv.txt");
        let event = if codex {
            r#"echo '{"type":"thread.started","thread_id":"cli-session"}'
echo '{"type":"item.completed","item":{"type":"agent_message","text":"cli reply"}}'"#
        } else {
            r#"echo '{"type":"step_start","sessionID":"cli-session"}'
echo '{"type":"text","part":{"text":"cli reply"}}'"#
        };
        std::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{event}\n",
                log.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let command_name = executable.to_string_lossy().into_owned();
        let selection = if codex {
            RuntimeAdapterSelection::CodexExec {
                command_name,
                working_directory: Some(directory.path().to_string_lossy().into_owned()),
            }
        } else {
            RuntimeAdapterSelection::OpenCodeRun {
                command_name,
                working_directory: Some(directory.path().to_string_lossy().into_owned()),
            }
        };
        let path = directory.path().join("framework.sqlite3");
        for turn in 0..2 {
            let host = FrameworkHost::persistent(&path, None);
            let runtime = registry()
                .with_framework_host(Some(host.clone()))
                .resolve(selection.clone());
            let mut context = base_context();
            context.run_code = format!("CLI-{turn}");
            let result = runtime.start(context, None).await.unwrap();
            assert_eq!(
                result.system_agent_session_id.as_deref(),
                Some("cli-session")
            );
            assert_eq!(result.final_output, "cli reply");
            host.close().await;
        }
        let argv = std::fs::read_to_string(log).unwrap();
        let lines: Vec<_> = argv.lines().collect();
        assert!(!lines[0].contains("cli-session"));
        assert!(lines[1].contains("cli-session"), "{argv}");
        if codex {
            assert!(lines[1].contains("resume"));
        } else {
            assert!(lines[1].contains("--session"));
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn dropping_cli_run_terminates_its_child_process_tree() {
    use std::os::unix::fs::PermissionsExt;
    for codex in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("agent");
        let pid_file = directory.path().join("child.pid");
        std::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nsleep 60 &\necho $! > '{}'\nwait\n",
                pid_file.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let command_name = executable.to_string_lossy().into_owned();
        let selection = if codex {
            RuntimeAdapterSelection::CodexExec {
                command_name,
                working_directory: Some(directory.path().to_string_lossy().into_owned()),
            }
        } else {
            RuntimeAdapterSelection::OpenCodeRun {
                command_name,
                working_directory: Some(directory.path().to_string_lossy().into_owned()),
            }
        };
        let runtime = registry().resolve(selection);
        let task = tokio::spawn(async move { runtime.start(base_context(), None).await });
        let pid = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if let Ok(pid) = std::fs::read_to_string(&pid_file)
                    && !pid.trim().is_empty()
                {
                    break pid.trim().to_owned();
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("CLI child did not start");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let output = std::process::Command::new("ps")
                    .args(["-o", "stat=", "-p", &pid])
                    .output()
                    .unwrap();
                let state = String::from_utf8_lossy(&output.stdout);
                if state.trim().is_empty() || state.trim().starts_with('Z') {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("CLI cancellation left a running descendant");
    }
}

#[tokio::test]
async fn legacy_external_reference_is_imported_without_rewriting_history() {
    use eliteforge_agent_core as fw;
    use eliteforge_agent_core::ConversationStore;
    use eliteforge_agent_official_plugin_storage_local::SqliteConversationStore;
    use sha2::{Digest, Sha256};
    use tron_backend::application::agent::FrameworkHost;
    let server = service_runtime_mock_server().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.sqlite3");
    let selection = RuntimeAdapterSelection::CodexAppServer {
        endpoint: server.base_url.clone(),
        working_directory: None,
    };
    let raw = registry()
        .with_framework_host(None)
        .resolve(selection.clone());
    let context = base_context();
    let identity = serde_json::to_vec(&(
        raw.session_scope().unwrap(),
        &context.user_code,
        &context.workspace_code,
        &context.execution_scope,
        &context.model.base_url,
        &context.model.api_key,
        &context.model.model_name,
    ))
    .unwrap();
    let old_scope = format!("{:x}", Sha256::digest(identity));
    let store = SqliteConversationStore::open(&path, Default::default())
        .await
        .unwrap();
    let mut old = store
        .open(context.conversation_code.as_deref().unwrap())
        .await
        .unwrap();
    let mut message = fw::Message::assistant("old external answer remains readable");
    message.model_context = Some(fw::ModelMessageContext {
        model: fw::ModelRef::new("framework.external", "agent"),
        protocol: "external-agent".into(),
        format: "framework.external-session.v1".into(),
        data: json!({"scope":old_scope,"sessionId":"legacy-native"}),
    });
    old.append(&[message.clone()]).await.unwrap();
    old.finish(fw::ConversationOutcome::Completed)
        .await
        .unwrap();
    drop(old);
    store.close().await.unwrap();
    let host = FrameworkHost::persistent(&path, None);
    let runtime = registry()
        .with_framework_host(Some(host.clone()))
        .resolve(selection.clone());
    let result = runtime.start(context.clone(), None).await.unwrap();
    assert_eq!(result.final_output, "service resume output");
    assert_eq!(
        server.requests.lock().unwrap().last().unwrap()["requestedSessionId"],
        "legacy-native"
    );
    let mut stable = context.clone();
    stable.run_code = "RUN-STABLE".into();
    for _ in 0..3 {
        runtime.start(stable.clone(), None).await.unwrap();
    }
    let mut wrong_account = context.clone();
    wrong_account.model.api_key = "other-account".into();
    let before = server.requests.lock().unwrap().len();
    assert!(
        runtime
            .resume("legacy-native", wrong_account, None)
            .await
            .is_err()
    );
    assert_eq!(server.requests.lock().unwrap().len(), before);
    host.close().await;
    drop(runtime);
    drop(host);
    let store = SqliteConversationStore::open(&path, Default::default())
        .await
        .unwrap();
    let mut binding = store
        .open_branch(
            &fw::ConversationKey::new(context.conversation_code.as_deref().unwrap())
                .branch("tron.external/CODEX_APP_SERVER"),
        )
        .await
        .unwrap();
    assert_eq!(
        binding.snapshot().await.unwrap().revision,
        1,
        "repeated same native references must not consume history capacity"
    );
    binding
        .finish(fw::ConversationOutcome::Completed)
        .await
        .unwrap();
    drop(binding);
    let mut old = store
        .open(context.conversation_code.as_deref().unwrap())
        .await
        .unwrap();
    assert_eq!(old.snapshot().await.unwrap().messages, vec![message]);
    old.finish(fw::ConversationOutcome::Completed)
        .await
        .unwrap();
    drop(old);
    store.close().await.unwrap();
    let host = FrameworkHost::persistent(&path, None);
    let runtime = registry()
        .with_framework_host(Some(host.clone()))
        .resolve(selection);
    // Switching back to an older known ID must become the next automatic choice.
    runtime
        .resume("alternate-native", stable.clone(), None)
        .await
        .unwrap();
    runtime
        .resume("legacy-native-next", stable.clone(), None)
        .await
        .unwrap();
    runtime.start(stable, None).await.unwrap();
    assert_eq!(
        server.requests.lock().unwrap().last().unwrap()["requestedSessionId"],
        "legacy-native-next"
    );
    let mut changed = context;
    changed.model.api_path = Some("/different-protocol-path".into());
    changed.system_agent_session_id = Some("legacy-native-next".into());
    runtime.start(changed, None).await.unwrap();
    assert_eq!(
        server.requests.lock().unwrap().last().unwrap()["action"],
        "start"
    );
    host.set_pack_enabled("framework.external-agent", false)
        .await
        .unwrap();
    let before = server.requests.lock().unwrap().len();
    assert!(runtime.start(base_context(), None).await.is_err());
    assert_eq!(server.requests.lock().unwrap().len(), before);
    host.close().await;
}

#[tokio::test]
async fn external_run_respects_busy_configuration_and_releases_binding_on_drop() {
    use tron_backend::application::agent::FrameworkHost;
    let server = service_runtime_mock_server_with_start_delay(30_000).await;
    let host = FrameworkHost::temporary(None).unwrap();
    let runtime = registry().with_framework_host(Some(host.clone())).resolve(
        RuntimeAdapterSelection::CodexAppServer {
            endpoint: server.base_url.clone(),
            working_directory: None,
        },
    );
    let task = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.start(base_context(), None).await }
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while server.requests.lock().unwrap().is_empty() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(host.set_extension_enabled("runtime", false).await.is_err());
    assert!(
        runtime.start(base_context(), None).await.is_err(),
        "same native binding must remain exclusive"
    );
    task.abort();
    let _ = task.await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if host.set_extension_enabled("runtime", false).await.is_ok() {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(runtime.start(base_context(), None).await.is_err());
    host.close().await;
}

#[tokio::test]
async fn external_bridge_observation_overflow_fails_and_releases_the_binding() {
    use tron_backend::application::agent::FrameworkHost;
    let server = service_runtime_mock_server().await;
    let host = FrameworkHost::temporary(None).unwrap();
    let runtime = registry().with_framework_host(Some(host.clone())).resolve(
        RuntimeAdapterSelection::CodexAppServer {
            endpoint: server.base_url.clone(),
            working_directory: None,
        },
    );
    let mut flood = base_context();
    flood.run_code = "RUN-FLOOD".into();
    let error = runtime.start(flood, None).await.unwrap_err();
    assert!(format!("{error:?}").contains("bounded event capacity"));
    assert_eq!(
        runtime
            .start(base_context(), None)
            .await
            .unwrap()
            .final_output,
        "service start output"
    );
    host.close().await;
}

#[tokio::test]
async fn closing_host_releases_native_bindings_even_when_outer_future_is_not_polled() {
    use tron_backend::application::agent::FrameworkHost;
    let slow = service_runtime_mock_server_with_start_delay(30_000).await;
    let fast = service_runtime_mock_server().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("closing.sqlite3");
    let host = FrameworkHost::persistent(&path, None);
    let runtime = registry().with_framework_host(Some(host.clone())).resolve(
        RuntimeAdapterSelection::CodexAppServer {
            endpoint: slow.base_url.clone(),
            working_directory: None,
        },
    );
    let mut execution = Box::pin(runtime.start(base_context(), None));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            tokio::select! {
                result = &mut execution => panic!("expected in-flight run: {result:?}"),
                _ = sleep(Duration::from_millis(10)) => {
                    if !slow.requests.lock().unwrap().is_empty() {
                        break;
                    }
                }
            }
        }
    })
    .await
    .unwrap();
    // A different store's owner must remain fenced independently of this host.
    use eliteforge_agent_core::{ConversationKey, ConversationStore};
    use eliteforge_agent_official_plugin_storage_local::SqliteConversationStore;
    let foreign = SqliteConversationStore::open(&path, Default::default())
        .await
        .unwrap();
    let foreign_key =
        ConversationKey::new("foreign-owner").branch("tron.external/CODEX_APP_SERVER");
    let mut foreign_session = foreign.open_branch(&foreign_key).await.unwrap();
    // The outer future retains the native lease while the inner run is drained.
    tokio::time::timeout(Duration::from_secs(5), host.close())
        .await
        .unwrap();
    drop(execution);
    assert!(foreign.open_branch(&foreign_key).await.is_err());
    foreign_session
        .append(&[eliteforge_agent_core::Message::assistant(
            "foreign owner still valid",
        )])
        .await
        .unwrap();
    foreign_session
        .finish(eliteforge_agent_core::ConversationOutcome::Completed)
        .await
        .unwrap();
    drop(foreign_session);
    foreign.close().await.unwrap();
    let reopened = FrameworkHost::persistent(&path, None);
    let runtime = registry()
        .with_framework_host(Some(reopened.clone()))
        .resolve(RuntimeAdapterSelection::CodexAppServer {
            endpoint: fast.base_url.clone(),
            working_directory: None,
        });
    assert_eq!(
        runtime
            .start(base_context(), None)
            .await
            .unwrap()
            .final_output,
        "service start output"
    );
    reopened.close().await;
    let unopened = FrameworkHost::persistent(directory.path().join("unused.sqlite3"), None);
    unopened.close().await;
    let runtime = registry()
        .with_framework_host(Some(unopened.clone()))
        .resolve(RuntimeAdapterSelection::CodexAppServer {
            endpoint: fast.base_url.clone(),
            working_directory: None,
        });
    let before = fast.requests.lock().unwrap().len();
    let error = runtime.start(base_context(), None).await.unwrap_err();
    assert!(format!("{error:?}").contains("host is closed"));
    assert_eq!(fast.requests.lock().unwrap().len(), before);
    let react = registry()
        .with_framework_host(Some(unopened))
        .resolve(RuntimeAdapterSelection::React);
    let error = react.start(base_context(), None).await.unwrap_err();
    assert!(format!("{error:?}").contains("host is closed"));
    assert!(!directory.path().join("unused.sqlite3").exists());
}
