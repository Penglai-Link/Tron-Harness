use std::{
    future::{Future, pending},
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::json;
use tokio::sync::Notify;
use tron_backend::{
    application::agent::{
        AgentInvocationError, AgentRunExecutor, AgentRuntime, AgentRuntimeRegistry, ModelClient,
        ModelStreamSink, RuntimeAdapterKey, RuntimeCapabilities,
    },
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, AgentRunResult, EffectiveSkill, EffectiveTool,
        EffectiveVariable, ModelRequest, ModelResponse, ToolRegistry,
    },
    shared::event_bus::EventBusHandle,
};

#[derive(Clone)]
struct BlockingFakeRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    calls: Arc<Mutex<Vec<String>>>,
    started: Arc<Notify>,
    cancel_error: Option<AgentInvocationError>,
}

impl BlockingFakeRuntime {
    fn new(key: RuntimeAdapterKey, supports_cancel: bool) -> Self {
        Self {
            key,
            capabilities: RuntimeCapabilities {
                supports_resume: true,
                supports_cancel,
                supports_steer: key == RuntimeAdapterKey::React,
                supports_streaming: true,
                supports_ephemeral: false,
                supports_workdir: true,
            },
            calls: Arc::new(Mutex::new(Vec::new())),
            started: Arc::new(Notify::new()),
            cancel_error: None,
        }
    }

    fn with_cancel_error(mut self, error: AgentInvocationError) -> Self {
        self.cancel_error = Some(error);
        self
    }

    fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("fake runtime calls mutex poisoned")
            .clone()
    }
}

impl AgentRuntime for BlockingFakeRuntime {
    fn key(&self) -> RuntimeAdapterKey {
        self.key
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        self.capabilities
    }

    fn start<'a>(
        &'a self,
        _context: AgentRunContext,
        _sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            self.calls
                .lock()
                .expect("fake runtime calls mutex poisoned")
                .push("start".into());
            self.started.notify_one();
            pending().await
        })
    }

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        _context: AgentRunContext,
        _sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            self.calls
                .lock()
                .expect("fake runtime calls mutex poisoned")
                .push(format!("resume:{session_id}"));
            self.started.notify_one();
            pending().await
        })
    }

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async move {
            self.calls
                .lock()
                .expect("fake runtime calls mutex poisoned")
                .push(format!("cancel:{run_code}"));
            match self.cancel_error.clone() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        })
    }
}

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

fn executor() -> (
    AgentRunExecutor,
    tron_backend::application::agent::ActiveRunRegistry,
) {
    let runtime_registry = AgentRuntimeRegistry::new(
        Arc::new(NoopModelClient),
        Arc::new(ToolRegistry::default()),
        EventBusHandle::disabled(),
        reqwest::Client::new(),
    );
    let active_runs = runtime_registry.active_runs();
    (AgentRunExecutor::new(runtime_registry), active_runs)
}

fn context() -> AgentRunContext {
    AgentRunContext {
        system_agent_fast_mode: false,
        collaboration_owner: None,
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
            supports_developer_role: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
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
            instruction: None,
            tool_names: Vec::new(),
        }],
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            domain_code: None,
            description: "echo".into(),
            input_schema: json!({ "type": "object" }),
            skill_name: None,
            required_variable_keys: Vec::new(),
        }],
        effective_tools: Vec::new(),
        tool_assignment: None,
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("hello")].into(),
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-07-17T00:00:00Z".into(),
        auto_model_route: None,
    }
}

async fn stop_runtime(
    runtime: Arc<BlockingFakeRuntime>,
    requested_session_id: Option<String>,
) -> AgentInvocationError {
    let started = runtime.started.notified();
    tokio::pin!(started);
    let (executor, active_runs) = executor();
    let execution_runtime = runtime.clone();
    let execution = tokio::spawn(async move {
        executor
            .execute_with_runtime(execution_runtime, context(), requested_session_id, None)
            .await
    });
    started.await;
    active_runs
        .request_stop("CONV-1", "RUN-1")
        .expect("matching fake runtime should stop");
    tokio::time::timeout(Duration::from_secs(1), execution)
        .await
        .expect("local execution should stop promptly")
        .expect("fake runtime task should join")
        .expect_err("stopped fake runtime should not complete")
}

#[tokio::test]
async fn stop_calls_supported_runtime_cancel_with_run_id_not_resume_session_id() {
    let runtime = Arc::new(BlockingFakeRuntime::new(
        RuntimeAdapterKey::OpenCodeServe,
        true,
    ));

    let error = stop_runtime(runtime.clone(), Some("svc-session-1".into())).await;

    assert_eq!(error, AgentInvocationError::RunStopped);
    assert_eq!(
        runtime.calls(),
        vec!["resume:svc-session-1", "cancel:RUN-1"]
    );
}

#[tokio::test]
async fn stop_does_not_call_cancel_for_react_runtime_without_cancel_capability() {
    let runtime = Arc::new(BlockingFakeRuntime::new(RuntimeAdapterKey::React, false));

    let error = stop_runtime(runtime.clone(), None).await;

    assert_eq!(error, AgentInvocationError::RunStopped);
    assert_eq!(runtime.calls(), vec!["start"]);
}

#[tokio::test]
async fn remote_cancel_failure_is_recorded_but_local_result_remains_stopped() {
    let runtime = Arc::new(
        BlockingFakeRuntime::new(RuntimeAdapterKey::CodexAppServer, true).with_cancel_error(
            AgentInvocationError::RuntimeFailed("remote cancel unavailable".into()),
        ),
    );

    let error = stop_runtime(runtime.clone(), None).await;

    assert_eq!(error, AgentInvocationError::RunStopped);
    assert_eq!(runtime.calls(), vec!["start", "cancel:RUN-1"]);
}
