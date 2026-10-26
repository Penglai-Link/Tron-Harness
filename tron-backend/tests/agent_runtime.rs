mod common;
use std::{
    sync::{
        Arc, Barrier, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use tokio::sync::Notify;
use tron_backend::{
    adapters::outbound::model::{chat_completions_request, embeddings_request, responses_request},
    adapters::outbound::persistence::SqliteChatRepository,
    application::agent::{ActiveRunRegistry, AgentSteerInput, ModelClient, ReactAgentBuilder},
    domain::agent::{
        AgentError, AgentExecutionScope, AgentKind, AgentMessage, AgentMessagePart,
        AgentModelConfig,
        AgentModelProtocol, AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool,
        EffectiveVariable, InProcessTool, ModelRequest, ModelResponse, Tool, ToolCall, ToolError,
        ToolExecutionResult, ToolObservation, ToolRegistry, ToolRequest,
    },
    shared::{
        event_bus::{BusRouteClass, EventBusConfig, EventBusRuntimeBuilder, RecordingEventSink},
        id::FixedSnowflakeIdProvider,
    },
};

#[derive(Clone)]
struct ScriptedModelClient {
    responses: Arc<std::sync::Mutex<Vec<ModelResponse>>>,
}

impl ScriptedModelClient {
    fn new(responses: Vec<ModelResponse>) -> Self {
        Self {
            responses: Arc::new(std::sync::Mutex::new(responses)),
        }
    }
}


/// Rewrite every scripted direct tool call into a `tool_execute` envelope,
/// keeping the original call id so observations stay addressable. Gateway
/// mode exposes only the stable pair to the model.
fn to_gateway_response(response: ModelResponse) -> ModelResponse {
    let mut assistant = response.assistant;
    for part in &mut assistant.parts {
        if let AgentMessagePart::ToolCall {
            call_id: _,
            tool_name,
            input,
            input_error,
        } = part
        {
            if input_error.is_some() { continue; }
            if tool_name == "tool_search" {
                *input = common::gateway_arguments(tool_name, input.clone());
                continue;
            }
            if tool_name == "tool_execute" {
                if let Ok(envelope) = eliteforge_agent_core::GatewayEnvelope::parse(input) {
                    *input = json!({"tool":envelope.tool,"arguments":common::gateway_arguments(&envelope.tool,envelope.arguments)});
                }
                continue;
            }
            let inner_name = tool_name.clone();
            let inner_input = input.clone();
            *tool_name = "tool_execute".into();
            *input = json!({"tool": inner_name, "arguments": common::gateway_arguments(&inner_name, inner_input)});
        }
    }
    ModelResponse {
        assistant,
        ..response
    }
}

impl ModelClient for ScriptedModelClient {
    fn generate<'a>(
        &'a self,
        _config: &'a AgentModelConfig,
        _request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut guard = self
                .responses
                .lock()
                .expect("scripted model mutex poisoned");
            if guard.is_empty() {
                return Err("no scripted response".to_string());
            }
            Ok(to_gateway_response(guard.remove(0)))
        })
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

#[derive(Clone)]
struct ScriptedResultModelClient {
    results: Arc<std::sync::Mutex<Vec<Result<ModelResponse, String>>>>,
    requests: Arc<std::sync::Mutex<Vec<(AgentModelConfig, ModelRequest)>>>,
}

#[derive(Clone)]
struct BlockingFirstResponseModelClient {
    requests: Arc<Mutex<Vec<ModelRequest>>>,
    calls: Arc<AtomicUsize>,
    started: Arc<Notify>,
    release: Arc<Notify>,
}

impl BlockingFirstResponseModelClient {
    fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(AtomicUsize::new(0)),
            started: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
        }
    }

    fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .expect("blocking model requests mutex poisoned")
            .clone()
    }
}

impl ModelClient for BlockingFirstResponseModelClient {
    fn generate<'a>(
        &'a self,
        _config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.requests
                .lock()
                .expect("blocking model requests mutex poisoned")
                .push(request.clone());
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                self.started.notify_one();
                self.release.notified().await;
                Ok(ModelResponse::final_output("first boundary output"))
            } else {
                Ok(ModelResponse::final_output("steered final output"))
            }
        })
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

impl ScriptedResultModelClient {
    fn new(results: Vec<Result<ModelResponse, String>>) -> Self {
        Self {
            results: Arc::new(std::sync::Mutex::new(results)),
            requests: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<(AgentModelConfig, ModelRequest)> {
        self.requests
            .lock()
            .expect("scripted result requests mutex poisoned")
            .clone()
    }
}

impl ModelClient for ScriptedResultModelClient {
    fn generate<'a>(
        &'a self,
        _config: &'a AgentModelConfig,
        _request: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.requests
                .lock()
                .expect("scripted result requests mutex poisoned")
                .push((_config.clone(), _request.clone()));
            let mut guard = self
                .results
                .lock()
                .expect("scripted result model mutex poisoned");
            if guard.is_empty() {
                return Err("no scripted result".to_string());
            }
            guard.remove(0).map(to_gateway_response)
        })
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

struct ProgressTool;

impl Tool for ProgressTool {
    fn name(&self) -> &str {
        "progress"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let mut progress = |_line: String| {};
        self.exec_with_progress(request, context, &mut progress)
    }

    fn exec_with_progress(
        &self,
        request: ToolRequest,
        _context: &AgentRunContext,
        progress: &mut dyn FnMut(String),
    ) -> Result<ToolExecutionResult, ToolError> {
        progress("first progress line".into());
        progress("second progress line".into());
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: request.tool_name,
            output: "progress complete".into(),
        }))
    }
}

fn base_context(run_code: &str) -> AgentRunContext {
    AgentRunContext {
        system_agent_fast_mode: false,
        collaboration_owner: None,
        agent_code: None,
        conversation_code: Some("CONV-1".into()),
        run_code: run_code.into(),
        workspace_code: "TRON_CORE".into(),
        user_code: Some("USR-1".into()),
        locale: "en-US".into(),
        request_id: Some("REQ-1".into()),
        stream_key: Some("CONV-1".into()),
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: AgentModelConfig {
            supports_developer_role: false,
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "gpt-test".into(),
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
            description: "Plan work".into(),
            instruction: Some("Think briefly.".into()),
            tool_names: vec!["echo".into()],
        }],
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            domain_code: None,
            description: "Echo input".into(),
            input_schema: json!({"type": "object"}),
            skill_name: Some("planner".into()),
            required_variable_keys: vec![],
        }],
        effective_tools: Vec::new(),
        tool_assignment: None,
        execution_scope: AgentExecutionScope {
            applications: Vec::new(),
            project_code: None,
            app_code: Some("APP-1".into()),
            repository_url: Some("https://example.test/repo.git".into()),
            local_directory: Some("/workspace/repo".into()),
            branch_name: Some("main".into()),
            task_code: Some("TASK-1".into()),
            allowed_local_directories: Vec::new(),
        },
        limits: AgentRunLimits {
            max_steps: 4,
            max_tool_calls: 2,
            max_parallel_tool_calls: 2,
        },
        messages: vec![AgentMessage::user("hello")].into(),
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-05-27T00:00:00Z".into(),
        auto_model_route: None,
    }
}

fn temp_database_url(label: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    std::env::temp_dir()
        .join(format!("{label}-{}-{millis}.sqlite3", std::process::id()))
        .display()
        .to_string()
}

fn temp_artifact_root(label: &str) -> std::path::PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    std::env::temp_dir().join(format!("{label}-{}-{millis}", std::process::id()))
}

// Observe official SDK output through the production owned consumer and real SQLite.
fn native_presentation_fixture() -> (
    tron_backend::application::agent::FrameworkHost,
    tron_backend::application::agent::AgentAguiStreamService,
) {
    use tron_backend::{
        adapters::outbound::persistence::SqliteAgentEventReplayStore,
        application::agent::AgentAguiStreamService, shared::event_bus::AguiEventSink,
    };
    let host =
        fixture_host(tron_backend::application::agent::FrameworkHost::temporary(None).unwrap());
    let replay = SqliteAgentEventReplayStore::new().unwrap();
    replay.bind_framework(&host).unwrap();
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        Arc::new(replay),
    );
    host.bind_presentation(service.clone()).unwrap();
    (host, service)
}
fn assert_native_source_identity(
    events: &[tron_backend::domain::agent::AgentAguiProtocolEvent],
    product_run: &str,
) {
    assert!(!events.is_empty());
    let logical = events[0].data["metadata"]["framework"]["runId"]
        .as_str()
        .unwrap();
    assert!(eliteforge_agent_core::RunId::parse_str(logical).is_ok());
    let execution = events[0].data["metadata"]["framework"]["executionId"]
        .as_str()
        .unwrap();
    assert!(eliteforge_agent_core::RunId::parse_str(execution).is_ok());
    let mut identities = std::collections::BTreeSet::new();
    let mut previous = 0;
    for event in events {
        assert_eq!(event.data["runId"], product_run);
        assert_eq!(event.data["metadata"]["framework"]["runId"], logical);
        assert_eq!(
            event.data["metadata"]["framework"]["executionId"],
            execution
        );
        let sequence = event.data["metadata"]["framework"]["sourceSequence"]
            .as_u64()
            .unwrap();
        assert!(sequence >= previous);
        previous = sequence;
        assert!(
            identities
                .insert(tron_backend::application::agent::protocol_event_identity(event).unwrap())
        );
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| event.sse_event == "RUN_STARTED")
            .count(),
        1
    );
}
fn native_text(events: &[tron_backend::domain::agent::AgentAguiProtocolEvent]) -> String {
    events
        .iter()
        .filter(|event| event.sse_event == "TEXT_MESSAGE_CONTENT")
        .map(|event| event.data["delta"].as_str().unwrap())
        .collect()
}
async fn wait_for_business_event(
    sink: &RecordingEventSink,
    name: &str,
) -> Vec<tron_backend::shared::event_bus::BusEvent> {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let events = sink.events();
            if events.iter().any(|event| event.event_name == name) {
                return events;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("existing business audit must still be delivered")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_completes_final_answer_and_publishes_metadata_only_events() {
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(100)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "done",
        )])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("dddf4313-5ee6-5bdd-b5a5-a5befdf49783"))
        .await
        .expect("final answer run should pass");

    assert_eq!(result.final_output, "done");
    assert_eq!(result.step_count, 1);
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "dddf4313-5ee6-5bdd-b5a5-a5befdf49783");
    assert_eq!(
        protocol[0].data["metadata"]["framework"]["runId"],
        result.native.as_ref().unwrap().run_id
    );
    assert_eq!(native_text(&protocol), "done");
    assert_eq!(
        protocol
            .iter()
            .filter(|event| event.sse_event == "RUN_FINISHED"
                && event.data["outcome"]["type"] == "success")
            .count(),
        1
    );
    let canonical = presentation.events("CONV-1").unwrap();
    let serialized = serde_json::to_string(&canonical).unwrap();
    for private in ["hello", "done", "sk-test", "prompt", "completion"] {
        assert!(!serialized.contains(private));
    }
    let events = wait_for_business_event(&sink, "agent.model.responded").await;
    let names: Vec<_> = events
        .iter()
        .filter(|event| event.event_name.starts_with("agent.model."))
        .map(|event| event.event_name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["agent.model.requested", "agent.model.responded"]
    );
    assert!(!events.iter().any(|event| matches!(
        event.event_name.as_str(),
        "agent.run.started" | "agent.run.completed" | "agent.output.created"
    )));
    assert!(events.iter().all(|event| {
        event
            .payload_fields
            .iter()
            .any(|field| field.key == "model_name" && field.value == "gpt-test")
    }));
    // Metadata events may reference budget block names (promptPrefix...) but
    // must never carry message content, tool results, or credentials.
    for (index, event) in events.iter().enumerate() {
        let serialized = format!("{:?}", event);
        if serialized.contains("hello")
            || serialized.contains("done")
            || serialized.contains("sk-test")
            || serialized.contains("completion")
        {
            panic!("event {index} leaked content: {serialized}");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_supports_auto_model_without_model_name() {
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-auto",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(200)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "done",
        )])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("5e9b27eb-966a-57a0-bc1d-da06e1b79954");
    context.model.model_code = "PERSON_AI:ROUTE:ChatAuto".into();
    context.model.model_name.clear();

    let result = agent
        .run_stream(context, Arc::new(|_event| {}))
        .await
        .expect("Auto run should succeed without a physical model name");

    assert_eq!(result.final_output, "done");
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "5e9b27eb-966a-57a0-bc1d-da06e1b79954");
    assert_eq!(native_text(&protocol), "done");
    assert!(
        protocol
            .iter()
            .any(|event| event.sse_event == "TEXT_MESSAGE_START")
    );
    assert!(
        protocol
            .iter()
            .any(|event| event.sse_event == "TEXT_MESSAGE_END")
    );
    assert_eq!(
        protocol
            .iter()
            .filter(|event| event.sse_event == "RUN_FINISHED")
            .count(),
        1
    );
    let events = wait_for_business_event(&sink, "agent.model.responded").await;
    assert!(events.iter().all(|event| {
        event
            .payload_fields
            .iter()
            .all(|field| field.key != "model_name")
    }));
    assert!(events.iter().all(|event| {
        event
            .attrs
            .iter()
            .any(|attr| attr.key == "model_code" && attr.value == "PERSON_AI:ROUTE:ChatAuto")
    }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_executes_allowed_tool_and_keeps_runs_isolated() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            },
        )))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"value": "a"})),
            ModelResponse::final_output("after-tool"),
            ModelResponse::final_output("second-run"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let first = agent
        .run(base_context("a5385a4f-12ce-5135-9cc5-2656c0887ba8"))
        .await
        .expect("tool run should pass");
    let second = agent
        .run(base_context("215720fb-93ff-50a6-8fa7-1538db6db213"))
        .await
        .expect("second run should pass");

    assert_eq!(first.final_output, "after-tool");
    assert_eq!(first.observations.len(), 1);
    assert_eq!(second.final_output, "second-run");
    assert!(second.observations.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_continues_after_tool_execution_failure() {
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-failure",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(101)),
    )
    .register_sink(sink.clone())
    .build();
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "flaky",
            |_request: ToolRequest| Err(ToolError::ExecutionFailed("boom".into())),
        )))
        .expect("tool should register");
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("flaky", json!({"value": "x"})),
            ModelResponse::final_output("after-failure"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run({
            let mut context = base_context("be4aff72-5bd2-50bc-a030-810b44e29bb5");
            context.tools.push(EffectiveTool {
                tool_name: "flaky".into(),
                domain_code: None,
                description: "Flaky".into(),
                input_schema: json!({"type": "object"}),
                skill_name: Some("planner".into()),
                required_variable_keys: vec![],
            });
            context
        })
        .await
        .expect("tool execution failure should not abort the run");

    assert_eq!(result.final_output, "after-failure");
    assert_eq!(result.observations.len(), 1);
    assert_eq!(result.observations[0].tool_name, "flaky");
    assert_eq!(result.observations[0].output, "boom");
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "be4aff72-5bd2-50bc-a030-810b44e29bb5");
    let result = protocol
        .iter()
        .find(|event| event.sse_event == "TOOL_CALL_RESULT")
        .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(result.data["content"].as_str().unwrap()).unwrap();
    assert_eq!(payload["isError"], true);
    assert!(payload["content"].to_string().contains("boom"));
    assert_eq!(native_text(&protocol), "after-failure");
    assert!(protocol.iter().any(
        |event| event.sse_event == "RUN_FINISHED" && event.data["outcome"]["type"] == "success"
    ));
    assert!(!protocol.iter().any(|event| event.sse_event == "RUN_ERROR"));
    let events = wait_for_business_event(&sink, "agent.observation.created").await;
    assert!(
        events
            .iter()
            .any(|event| event.event_name == "agent.tool.failed")
    );
    assert!(
        events
            .iter()
            .any(|event| event.event_name == "agent.observation.created")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_fails_when_tool_recovery_ends_without_an_assistant_response() {
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-empty-final-output",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(102)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![ToolCall {
                call_id: Some("call-write".into()),
                tool_name: "fs_write_text".into(),
                input: serde_json::Value::Null,
                input_error: Some("function arguments are missing".into()),
            }]),
            ModelResponse::final_output(""),
        ])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(base_context("2d8d5f7c-920d-5588-87fa-ddaf84b6c7e7"))
        .await
        .expect_err("an empty final response must fail the run");

    assert!(
        matches!(
            &error,
            AgentError::EmptyFinalOutput(message)
                if message.contains("fs_write_text: tool input is invalid: function arguments are missing")
        ),
        "{error:?}"
    );
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "2d8d5f7c-920d-5588-87fa-ddaf84b6c7e7");
    assert!(
        protocol
            .iter()
            .any(|event| event.sse_event == "TOOL_CALL_RESULT")
    );
    assert_eq!(
        protocol
            .iter()
            .filter(|event| event.sse_event == "RUN_ERROR")
            .count(),
        1
    );
    assert!(
        !protocol
            .iter()
            .any(|event| event.sse_event == "RUN_FINISHED")
    );
    let events = wait_for_business_event(&sink, "agent.tool.failed").await;
    assert!(
        events
            .iter()
            .any(|event| event.event_name == "agent.tool.failed")
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_name == "agent.run.completed")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_allows_empty_successful_tool_observations() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "query",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: String::new(),
                })
            },
        )))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("query", json!({"keyword": "missing"})),
            ModelResponse::final_output("No records were found."),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run({
            let mut context = base_context("5ee7e2fe-4ba1-5825-a8bb-bcc75405cadf");
            context.tools.push(EffectiveTool {
                tool_name: "query".into(),
                domain_code: None,
                description: "Query records".into(),
                input_schema: json!({"type": "object"}),
                skill_name: Some("planner".into()),
                required_variable_keys: vec![],
            });
            context
        })
        .await
        .expect("an empty successful query result must not fail the run");

    assert_eq!(result.final_output, "No records were found.");
    assert_eq!(result.observations.len(), 1);
    assert!(result.observations[0].output.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_executes_multiple_tool_calls_in_one_model_turn() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            },
        )))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                    input_error: None,
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("5b5cfe24-85e7-5b9c-a4f0-ee8dbe9319ca"))
        .await
        .expect("multi tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 2);
    let mut outputs = result.observations.iter().map(|o| o.output.as_str()).collect::<Vec<_>>();
    outputs.sort();
    assert_eq!(outputs, ["observed:\"a\"", "observed:\"b\""]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_executes_multiple_tool_calls_concurrently_by_default() {
    let registry = ToolRegistry::default();
    let barrier = Arc::new(Barrier::new(2));
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let barrier = Arc::clone(&barrier);
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            move |request: ToolRequest| {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                let mut observed = max_running.load(Ordering::SeqCst);
                while current > observed {
                    match max_running.compare_exchange(
                        observed,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
                barrier.wait();
                thread::sleep(Duration::from_millis(50));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                    input_error: None,
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = tokio::time::timeout(
        Duration::from_secs(2),
        agent.run(base_context("18a5d0db-8ec5-56a3-bada-398ff899f3cc")),
    )
    .await
    .expect("parallel tool calls should complete within timeout")
    .expect("parallel tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 2);
    let mut outputs = result.observations.iter().map(|o| o.output.as_str()).collect::<Vec<_>>();
    outputs.sort();
    assert_eq!(outputs, ["observed:\"a\"", "observed:\"b\""]);
    assert_eq!(max_running.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_continues_after_unauthorized_parallel_tool_call() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "echo completed".into(),
                })
            },
        )))
        .expect("allowed tool should register");
    let hidden_executions = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("hidden_tool", {
            let hidden_executions = Arc::clone(&hidden_executions);
            move |request: ToolRequest| {
                hidden_executions.fetch_add(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "hidden tool must not execute".into(),
                })
            }
        })))
        .expect("hidden tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-allowed".into()),
                    tool_name: "echo".into(),
                    input: json!({}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-unauthorized".into()),
                    tool_name: "hidden_tool".into(),
                    input: json!({}),
                    input_error: None,
                },
            ]),
            ModelResponse::final_output("recovered after parallel rejection"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("7dadf35b-5c3d-50e8-af16-d42a6ba05753"))
        .await
        .expect("an unauthorized parallel tool should not abort the run");

    assert_eq!(result.final_output, "recovered after parallel rejection");
    assert_eq!(result.observations.len(), 2, "{:?}", result.observations);
    // Completion order within a parallel batch is not guaranteed; assert by
    // tool identity instead of positional order.
    let echo = result
        .observations
        .iter()
        .find(|observation| observation.tool_name == "echo")
        .expect("echo observation");
    let hidden = result
        .observations
        .iter()
        .find(|observation| observation.tool_name == "hidden_tool")
        .expect("hidden tool rejection");
    assert_eq!(echo.output, "echo completed");
    assert!(hidden.output.contains("is not allowed"));
    assert_eq!(hidden_executions.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_executes_single_tool_call_without_parallel_batching() {
    let registry = ToolRegistry::default();
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            move |request: ToolRequest| {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                let mut observed = max_running.load(Ordering::SeqCst);
                while current > observed {
                    match max_running.compare_exchange(
                        observed,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
                thread::sleep(Duration::from_millis(50));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![ToolCall {
                call_id: Some("call-1".into()),
                tool_name: "echo".into(),
                input: json!({"value": "a"}),
                input_error: None,
            }]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("22a4551c-2169-5c0c-8496-f6e8428fc69d"))
        .await
        .expect("single tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 1);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(max_running.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_batches_parallel_tool_calls_by_parallel_limit() {
    let registry = ToolRegistry::default();
    let barrier = Arc::new(Barrier::new(2));
    let running = Arc::new(AtomicUsize::new(0));
    let max_running = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let barrier = Arc::clone(&barrier);
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            move |request: ToolRequest| {
                let current = running.fetch_add(1, Ordering::SeqCst) + 1;
                let mut observed = max_running.load(Ordering::SeqCst);
                while current > observed {
                    match max_running.compare_exchange(
                        observed,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => observed = actual,
                    }
                }
                barrier.wait();
                thread::sleep(Duration::from_millis(20));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-3".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "c"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-4".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "d"}),
                    input_error: None,
                },
            ]),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("a42314dc-fa1f-5334-bcdd-ed154442792b");
    context.limits.max_tool_calls = 6;
    context.limits.max_parallel_tool_calls = 2;
    let result = tokio::time::timeout(Duration::from_secs(2), agent.run(context))
        .await
        .expect("batched parallel tool calls should complete within timeout")
        .expect("batched parallel tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 4);
    assert_eq!(max_running.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_executes_remaining_parallel_budget_before_failing() {
    let registry = ToolRegistry::default();
    let executed = Arc::new(AtomicUsize::new(0));
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let executed = Arc::clone(&executed);
            move |request: ToolRequest| {
                executed.fetch_add(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["value"]),
                })
            }
        })))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "a"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "b"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-3".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "c"}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-4".into()),
                    tool_name: "echo".into(),
                    input: json!({"value": "d"}),
                    input_error: None,
                },
            ]),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("efeb50dc-3e9c-5f47-93e7-fb27054c4018");
    context.limits.max_tool_calls = 3;
    context.limits.max_parallel_tool_calls = 2;
    let result = agent
        .run(context)
        .await
        .expect("native tool-call limit retains the complete outcome");
    let native = result.native.as_ref().unwrap();
    assert_eq!(
        native.status,
        tron_backend::domain::agent::AgentNativeRunStatus::ToolCallLimit
    );
    assert!(!native.run_id.is_empty());
    assert_eq!(result.step_count, 1);
    assert_eq!(result.observations.len(), 4, "{:?}", result.observations);
    // The fourth call over the budget must report exhaustion; completion
    // order within the batch is not guaranteed, so match by content.
    assert!(
        result
            .observations
            .iter()
            .any(|observation| observation.output.contains("tool-call budget exhausted")),
        "{:?}",
        result.observations
    );
    assert!(result.final_output.is_empty());
    let snapshot = result.sdk_execution.as_ref().unwrap();
    assert_eq!(snapshot.model_calls, 1);
    assert_eq!(snapshot.steps, 1);
    assert_eq!(snapshot.tool_calls, 3);
    assert_eq!(executed.load(Ordering::SeqCst), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_publishes_tool_agui_correlation_payloads() {
    let long_command = "cargo test --package tron-backend --test agent_runtime ".repeat(3);
    let expected_args = serde_json::to_string(&json!({ "command": long_command.clone() }))
        .expect("tool args should serialize");
    let expected_result = format!("observed:\"{long_command}\"");
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!("observed:{}", request.input["command"]),
                })
            },
        )))
        .expect("tool should register");
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-tool",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(200)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"command": long_command})),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    agent
        .run(base_context("4420c784-3ccb-5756-9ab2-6cda552745c1"))
        .await
        .expect("tool run should pass");

    let events = wait_for_business_event(&sink, "agent.observation.created").await;
    let tool_started = events
        .iter()
        .find(|event| event.event_name == "agent.tool.started")
        .expect("tool start event should be published");
    let tool_completed = events
        .iter()
        .find(|event| event.event_name == "agent.tool.completed")
        .expect("tool completion event should be published");
    let observation = events
        .iter()
        .find(|event| event.event_name == "agent.observation.created")
        .expect("observation event should be published");

    let payload =
        |key: &str, event: &tron_backend::shared::event_bus::BusEvent| -> Option<String> {
            event
                .payload_fields
                .iter()
                .find(|attr| attr.key == key)
                .map(|attr| attr.value.clone())
        };

    let invocation_id =
        payload("tool_call_id", tool_started).expect("Framework invocation identity");
    assert!(eliteforge_agent_core::RunId::parse_str(&invocation_id).is_ok());
    assert_eq!(payload("tool_name", tool_started).as_deref(), Some("echo"));
    assert_eq!(
        payload("tool_args_json", tool_started).as_deref(),
        Some(expected_args.as_str())
    );
    assert_eq!(
        payload("tool_call_id", tool_completed).as_deref(),
        Some(invocation_id.as_str())
    );
    assert_eq!(
        payload("tool_result_json", tool_completed).as_deref(),
        Some(expected_result.as_str())
    );
    assert_eq!(
        payload("tool_call_id", observation).as_deref(),
        Some(invocation_id.as_str())
    );
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "4420c784-3ccb-5756-9ab2-6cda552745c1");
    let start = protocol
        .iter()
        .find(|event| event.sse_event == "TOOL_CALL_START")
        .unwrap();
    let args = protocol
        .iter()
        .find(|event| event.sse_event == "TOOL_CALL_ARGS")
        .unwrap();
    let result = protocol
        .iter()
        .find(|event| event.sse_event == "TOOL_CALL_RESULT")
        .unwrap();
    let batch: eliteforge_agent_core::ToolBatchItem = serde_json::from_str(
        &payload("tool_batch_json",tool_started).expect("Framework batch identity")
    ).unwrap();
    let parent_id = batch.parent_invocation_id.to_string();
    assert_ne!(parent_id,invocation_id);
    assert_eq!(batch.invocation_id.to_string(),invocation_id);
    assert_eq!(batch.index,0);
    for event in [tool_completed,observation] {
        let recorded: eliteforge_agent_core::ToolBatchItem = serde_json::from_str(
            &payload("tool_batch_json",event).unwrap()).unwrap();
        assert_eq!(recorded,batch);
    }
    assert_eq!(start.data["toolCallId"], parent_id);
    assert_eq!(start.data["toolCallName"], "echo");
    assert_eq!(args.data["toolCallId"], parent_id);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(args.data["delta"].as_str().unwrap()).unwrap(),
        json!({"items":[serde_json::from_str::<serde_json::Value>(&expected_args).unwrap()]})
    );
    assert_eq!(result.data["toolCallId"], parent_id);
    let output: serde_json::Value = serde_json::from_str(result.data["content"].as_str().unwrap()).unwrap();
    assert_eq!(output["isError"], false);
    let public_result: eliteforge_agent_core::ToolBatchResult =
        serde_json::from_value(output["structured"].clone()).unwrap();
    assert_eq!(public_result.results[0].identity,batch);
    assert_eq!(public_result.results[0].status,eliteforge_agent_core::ToolBatchStatus::Completed);
    assert_eq!(public_result.results[0].result,Some(json!(expected_result)));
    assert_eq!(public_result.results[0].content.text(),expected_result);
    let end_index = protocol
        .iter()
        .position(|event| event.sse_event == "TOOL_CALL_END")
        .unwrap();
    let result_index = protocol
        .iter()
        .position(|event| event.sse_event == "TOOL_CALL_RESULT")
        .unwrap();
    assert!(
        end_index < result_index,
        "native argument stream closes before execution result"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_publishes_tool_progress_activity_events() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(ProgressTool))
        .expect("progress tool should register");
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-tool-progress",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(240)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("progress", json!({})),
            ModelResponse::final_output("after-progress"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("39cfe5c2-cf4e-5faf-adff-79e48a427562");
    context.tools = vec![EffectiveTool {
        tool_name: "progress".into(),
        domain_code: None,
        description: "Progress".into(),
        input_schema: json!({"type": "object"}),
        skill_name: None,
        required_variable_keys: vec![],
    }];

    let result = agent
        .run(context)
        .await
        .expect("progress tool run should pass");

    assert_eq!(result.final_output, "after-progress");
    let events = wait_for_business_event(&sink, "agent.observation.created").await;
    let activity_payloads = events
        .iter()
        .filter(|event| event.event_name == "agent.tool.activity")
        .map(|event| {
            event
                .payload_fields
                .iter()
                .find(|attr| attr.key == "tool_result_json")
                .map(|attr| attr.value.clone())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        activity_payloads,
        vec!["first progress line", "second progress line"]
    );
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "39cfe5c2-cf4e-5faf-adff-79e48a427562");
    let activities: Vec<_> = protocol
        .iter()
        .filter(|event| {
            event.sse_event == "ACTIVITY_SNAPSHOT"
                && event.data["activityType"] == "tron.tool.progress"
        })
        .collect();
    assert_eq!(activities.len(), 2);
    let starts: Vec<_> = protocol
        .iter()
        .filter(|event| event.sse_event == "TOOL_CALL_START")
        .collect();
    assert_eq!(starts.len(), 1);
    let first_batch: eliteforge_agent_core::ToolBatchItem =
        serde_json::from_value(activities[0].data["content"]["batch"].clone()).unwrap();
    assert_eq!(first_batch.parent_invocation_id.to_string(), starts[0].data["toolCallId"]);
    assert_ne!(first_batch.parent_invocation_id, first_batch.invocation_id);
    assert_eq!(first_batch.index, 0);
    for activity in &activities {
        let identity: eliteforge_agent_core::ToolBatchItem =
            serde_json::from_value(activity.data["content"]["batch"].clone()).unwrap();
        assert_eq!(identity, first_batch);
    }
    for start in starts {
        let lines: Vec<_> = activities
            .iter()
            .filter(|event| event.data["content"]["toolCallId"] == start.data["toolCallId"])
            .map(|event| event.data["content"]["line"].as_str().unwrap())
            .collect();
        assert_eq!(lines, vec!["first progress line", "second progress line"]);
        assert!(
            activities
                .iter()
                .all(|event| event.data["content"]["toolName"] == "progress")
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_publishes_parallel_tool_progress_activity_events() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(ProgressTool))
        .expect("progress tool should register");
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-parallel-tool-progress",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(245)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-1".into()),
                    tool_name: "progress".into(),
                    input: json!({"index": 1}),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-2".into()),
                    tool_name: "progress".into(),
                    input: json!({"index": 2}),
                    input_error: None,
                },
            ]),
            ModelResponse::final_output("after-progress"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("38b0fcb8-fc0d-58ca-96ee-e8c814f38020");
    context.tools = vec![EffectiveTool {
        tool_name: "progress".into(),
        domain_code: None,
        description: "Progress".into(),
        input_schema: json!({"type": "object"}),
        skill_name: None,
        required_variable_keys: vec![],
    }];

    let result = agent
        .run(context)
        .await
        .expect("parallel progress tool run should pass");

    assert_eq!(result.final_output, "after-progress");
    let events = {
        let mut current = Vec::new();
        for _ in 0..100 {
            current = sink.events();
            if current
                .iter()
                .filter(|event| event.event_name == "agent.tool.activity")
                .count()
                >= 4
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        current
    };
    let payload =
        |key: &str, event: &tron_backend::shared::event_bus::BusEvent| -> Option<String> {
            event
                .payload_fields
                .iter()
                .find(|attr| attr.key == key)
                .map(|attr| attr.value.clone())
        };
    let activity_events = events
        .iter()
        .filter(|event| event.event_name == "agent.tool.activity")
        .collect::<Vec<_>>();

    assert_eq!(activity_events.len(), 4);
    let invocation_ids = events
        .iter()
        .filter(|event| event.event_name == "agent.tool.started")
        .map(|event| payload("tool_call_id", event).expect("invocation identity"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(invocation_ids.len(), 2);
    for tool_call_id in invocation_ids {
        let lines = activity_events
            .iter()
            .filter(|event| {
                payload("tool_call_id", event).as_deref() == Some(tool_call_id.as_str())
            })
            .map(|event| payload("tool_result_json", event).unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines.contains(&"first progress line".to_string()));
        assert!(lines.contains(&"second progress line".to_string()));
    }
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "38b0fcb8-fc0d-58ca-96ee-e8c814f38020");
    let activities: Vec<_> = protocol
        .iter()
        .filter(|event| {
            event.sse_event == "ACTIVITY_SNAPSHOT"
                && event.data["activityType"] == "tron.tool.progress"
        })
        .collect();
    assert_eq!(activities.len(), 4);
    let starts: Vec<_> = protocol
        .iter()
        .filter(|event| event.sse_event == "TOOL_CALL_START")
        .collect();
    assert_eq!(starts.len(), 2);
    for start in starts {
        let lines: Vec<_> = activities
            .iter()
            .filter(|event| event.data["content"]["toolCallId"] == start.data["toolCallId"])
            .map(|event| event.data["content"]["line"].as_str().unwrap())
            .collect();
        assert_eq!(lines, vec!["first progress line", "second progress line"]);
        assert!(
            activities
                .iter()
                .all(|event| event.data["content"]["toolName"] == "progress")
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_truncates_oversize_tool_event_payloads_without_failing_run() {
    let oversized_output = "x".repeat(73_854);
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("echo", {
            let oversized_output = oversized_output.clone();
            move |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: oversized_output.clone(),
                })
            }
        })))
        .expect("tool should register");
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "agent-runtime-oversize-tool",
        BusRouteClass::Agui,
    ));
    let event_bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(250)),
    )
    .register_sink(sink.clone())
    .build();
    let (native_host, presentation) = native_presentation_fixture();
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"command": "large"})),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    // Exercise event truncation without also overflowing the model input budget.
    let mut context = base_context("24e4c1d9-abfd-5b3f-aff6-d1da60e55e72");
    // Native protected history costs 148358 tokens, beyond the old 127845-token allocation.
    context.model.context_window_tokens = Some(256_000);
    let result = agent
        .run(context)
        .await
        .expect("oversize tool event payload should not fail the run");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations[0].output, oversized_output);
    let events = wait_for_business_event(&sink, "agent.observation.created").await;
    let tool_completed = events
        .iter()
        .find(|event| event.event_name == "agent.tool.completed")
        .expect("tool completion event should be published");
    let truncated_result = tool_completed
        .payload_fields
        .iter()
        .find(|attr| attr.key == "tool_result_json")
        .map(|attr| attr.value.clone())
        .expect("tool result payload should be present");

    assert!(truncated_result.len() <= 4096);
    assert!(truncated_result.contains("[truncated: original_bytes=73854]"));
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "24e4c1d9-abfd-5b3f-aff6-d1da60e55e72");
    let start = protocol
        .iter()
        .find(|event| event.sse_event == "TOOL_CALL_START")
        .unwrap();
    let native_result = protocol
        .iter()
        .find(|event| event.sse_event == "TOOL_CALL_RESULT")
        .unwrap();
    assert_eq!(native_result.data["toolCallId"], start.data["toolCallId"]);
    let output: serde_json::Value =
        serde_json::from_str(native_result.data["content"].as_str().unwrap()).unwrap();
    assert_eq!(output["isError"], false);
    assert!(protocol.iter().any(
        |event| event.sse_event == "RUN_FINISHED" && event.data["outcome"]["type"] == "success"
    ));
    assert!(!protocol.iter().any(|event| event.sse_event == "RUN_ERROR"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tool_registry_rejects_missing_or_unauthorized_tools() {
    let registry = ToolRegistry::default();
    let context = base_context("81af840c-9358-508b-9a4f-e7806e8f2092");

    let missing = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "echo".into(),
                input: json!({}),
                invocation_id: None,
            },
            &context,
        )
        .expect_err("registered executor should be required");
    assert!(matches!(missing, ToolError::MissingTool(_)));

    registry
        .register(Arc::new(InProcessTool::new("hidden", |_request| {
            Ok(ToolObservation {
                tool_name: "hidden".into(),
                output: "hidden".into(),
            })
        })))
        .expect("hidden tool should register");
    let unauthorized = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "hidden".into(),
                input: json!({}),
                invocation_id: None,
            },
            &context,
        )
        .expect_err("tool must be in allowed snapshot");
    assert!(matches!(unauthorized, ToolError::UnauthorizedTool(_)));
}

#[test]
fn tool_registry_authorizes_effective_tools_that_are_not_model_exposed() {
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("fs_write_text", |request| {
            Ok(ToolObservation {
                tool_name: request.tool_name,
                output: "written".into(),
            })
        })))
        .expect("effective tool should register");

    let discovery = EffectiveTool {
        tool_name: "discovery_expand_tool_domain".into(),
        domain_code: Some("discovery".into()),
        description: "Expand one tool domain".into(),
        input_schema: json!({"type": "object"}),
        skill_name: None,
        required_variable_keys: Vec::new(),
    };
    let write_text = EffectiveTool {
        tool_name: "fs_write_text".into(),
        domain_code: Some("fs".into()),
        description: "Write text".into(),
        input_schema: json!({"type": "object"}),
        skill_name: None,
        required_variable_keys: Vec::new(),
    };
    let mut context = base_context("280b193d-cc75-5027-8197-45ece3c9fc8b");
    context.tools = vec![discovery.clone()];
    context.effective_tools = vec![discovery, write_text];

    let execution = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "fs_write_text".into(),
                input: json!({}),
                invocation_id: None,
            },
            &context,
        )
        .expect("an effective tool should remain authorized before model exposure");

    assert_eq!(execution.observation.output, "written");
    assert_eq!(
        context.tools.len(),
        1,
        "model exposure should stay progressive"
    );
    let serialized = serde_json::to_value(&context).expect("context should serialize");
    assert_eq!(serialized["tools"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        serialized["effectiveTools"].as_array().map(Vec::len),
        Some(2)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn model_request_builders_use_frozen_model_config() {
    let config = base_context("00036f81-8c18-565a-bb47-0f37cb5c7e4b").model;
    let request = ModelRequest {
        messages: vec![AgentMessage::user("hi")].into(),
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            domain_code: None,
            description: "Echo".into(),
            input_schema: json!({"type": "object"}),
            skill_name: None,
            required_variable_keys: vec![],
        }],
        system_agent_session_id: None,
        prepared_tool_schema: None,
        response_input_items: None,
        previous_response_id: None,
    };

    let chat = chat_completions_request(&config, &request);
    assert_eq!(chat["model"], json!("gpt-test"));
    assert_eq!(chat["tools"][0]["function"]["name"], json!("echo"));

    let responses = responses_request(&config, &request);
    assert_eq!(responses["model"], json!("gpt-test"));
    assert_eq!(responses["input"], json!("hi"));

    let embeddings = embeddings_request(&config, "embed me");
    assert_eq!(embeddings["model"], json!("gpt-test"));
    assert_eq!(embeddings["input"], json!("embed me"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn max_step_exhaustion_fails_without_output_success() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
            max_parallel_tool_calls: 1,
        },
        ..base_context("96663e53-8e55-55fb-8731-7d7e4ff52f3c")
    };
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("echo", |_request| {
            Ok(ToolObservation {
                tool_name: "echo".into(),
                output: "again".into(),
            })
        })))
        .expect("tool should register");
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::tool_call(
            "echo",
            json!({}),
        )])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(context)
        .await
        .expect("native step limit retains the complete outcome");
    let native = result.native.as_ref().unwrap();
    assert_eq!(
        native.status,
        tron_backend::domain::agent::AgentNativeRunStatus::StepLimit
    );
    assert!(!native.run_id.is_empty());
    assert_eq!(result.step_count, 1);
    assert_eq!(result.observations.len(), 1);
    assert!(result.final_output.is_empty());
    let snapshot = result.sdk_execution.as_ref().unwrap();
    assert_eq!(snapshot.model_calls, 1);
    assert_eq!(snapshot.steps, 1);
    assert_eq!(snapshot.tool_calls, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_tool_call_limit_is_rejected_before_model_execution() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 0,
            max_parallel_tool_calls: 1,
        },
        ..base_context("50bbd0a5-9537-5a2b-ad2d-08d22c560987")
    };
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![])),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(context)
        .await
        .expect_err("zero tool-call limit should be rejected");

    assert_eq!(error.to_string(), "maxToolCalls must be greater than zero");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_parallel_tool_call_limit_is_rejected_before_model_execution() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
            max_parallel_tool_calls: 0,
        },
        ..base_context("d2b55ed7-6f46-559e-abbe-8573171fb04f")
    };
    let agent = native_agent_builder(
        Arc::new(ScriptedModelClient::new(vec![])),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(context)
        .await
        .expect_err("zero parallel tool-call limit should be rejected");

    assert_eq!(
        error.to_string(),
        "maxParallelToolCalls must be greater than zero"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_consumes_steer_at_final_model_boundary_without_starting_a_second_run() {
    let active_runs = ActiveRunRegistry::new();
    let guard = active_runs
        .register(
            "SESSION-STEER",
            "5e71bc66-db6e-5975-8008-ee147ee84f5e",
            "REACT",
            true,
        )
        .expect("active run should register");
    let control = active_runs
        .control("SESSION-STEER", "5e71bc66-db6e-5975-8008-ee147ee84f5e")
        .expect("react run control should resolve");
    let model_client = BlockingFirstResponseModelClient::new();
    let started = model_client.started.clone();
    let release = model_client.release.clone();
    let agent = native_agent_builder(
        Arc::new(model_client.clone()),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("5e71bc66-db6e-5975-8008-ee147ee84f5e");
    context.conversation_code = Some("SESSION-STEER".into());
    let started_signal = started.notified();
    tokio::pin!(started_signal);
    let run = tokio::spawn(async move { agent.run_with_control(context, Some(control)).await });

    started_signal.await;
    active_runs
        .accept_steer(
            "SESSION-STEER",
            "5e71bc66-db6e-5975-8008-ee147ee84f5e",
            || {
                Ok::<_, ()>((
                    (),
                    AgentSteerInput {
                        page_context: None,
                        annotations: Vec::new(),
                        requested_skills: Vec::new(),
                        steer_code: "MSG-STEER-1".into(),
                        content: "change direction now".into(),
                        attachments: Vec::new(),
                    },
                ))
            },
        )
        .expect("steer should route to the active run")
        .expect("steer acceptance should succeed");
    release.notify_one();

    let result = run
        .await
        .expect("react run task should join")
        .expect("react run should complete");
    assert_eq!(result.run_code, "5e71bc66-db6e-5975-8008-ee147ee84f5e");
    assert_eq!(result.step_count, 2);
    // After a steer at the final boundary, the run re-enters the model once
    // and the completed output is the post-steer answer (the pre-steer text
    // was already streamed and journaled, not concatenated into the result).
    assert_eq!(result.final_output, "steered final output");
    let requests = model_client.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert!(
            request.messages.iter().any(|message| {
                message.role == "user" && message.plain_text_content() == "hello"
            })
        );
    }
    assert!(requests[1].messages.iter().any(|message| {
        message.role == "assistant" && message.plain_text_content() == "first boundary output"
    }));
    assert_eq!(
        requests[1]
            .messages
            .iter().find(|message| message.role == "user" && message.plain_text_content() == "change direction now")
            .expect("second request should contain steer input")
            .plain_text_content(),
        "change direction now"
    );
    drop(guard);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn react_agent_waits_for_current_tool_step_before_injecting_steer() {
    let active_runs = ActiveRunRegistry::new();
    let _guard = active_runs
        .register(
            "SESSION-TOOL-STEER",
            "f13d61b4-5c6e-5c62-8f48-100cba1df259",
            "REACT",
            true,
        )
        .expect("active run should register");
    let control = active_runs
        .control("SESSION-TOOL-STEER", "f13d61b4-5c6e-5c62-8f48-100cba1df259")
        .expect("react run control should resolve");
    let tool_started = Arc::new(Notify::new());
    let tool_gate = Arc::new((Mutex::new(false), Condvar::new()));
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("blocking-tool", {
            let tool_started = tool_started.clone();
            let tool_gate = tool_gate.clone();
            move |request: ToolRequest| {
                tool_started.notify_one();
                let (released, condition) = &*tool_gate;
                let mut released = released.lock().expect("tool gate mutex poisoned");
                while !*released {
                    released = condition
                        .wait(released)
                        .expect("tool gate condvar should wait");
                }
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "tool completed".into(),
                })
            }
        })))
        .expect("blocking tool should register");
    let model_client = ScriptedResultModelClient::new(vec![
        Ok(ModelResponse::tool_call("blocking-tool", json!({}))),
        Ok(ModelResponse::final_output("after tool steer")),
    ]);
    let agent = native_agent_builder(
        Arc::new(model_client.clone()),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("f13d61b4-5c6e-5c62-8f48-100cba1df259");
    context.conversation_code = Some("SESSION-TOOL-STEER".into());
    context.tools.push(EffectiveTool {
        tool_name: "blocking-tool".into(),
        domain_code: None,
        description: "blocks until the test releases it".into(),
        input_schema: json!({"type": "object"}),
        skill_name: None,
        required_variable_keys: Vec::new(),
    });
    let tool_started_signal = tool_started.notified();
    tokio::pin!(tool_started_signal);
    let run = tokio::spawn(async move { agent.run_with_control(context, Some(control)).await });

    tool_started_signal.await;
    active_runs
        .accept_steer(
            "SESSION-TOOL-STEER",
            "f13d61b4-5c6e-5c62-8f48-100cba1df259",
            || {
                Ok::<_, ()>((
                    (),
                    AgentSteerInput {
                        page_context: None,
                        annotations: Vec::new(),
                        requested_skills: Vec::new(),
                        steer_code: "MSG-STEER-TOOL".into(),
                        content: "use the completed tool result differently".into(),
                        attachments: Vec::new(),
                    },
                ))
            },
        )
        .expect("steer should route while the tool is in flight")
        .expect("steer acceptance should succeed");
    {
        let (released, condition) = &*tool_gate;
        *released.lock().expect("tool gate mutex poisoned") = true;
        condition.notify_all();
    }

    let result = run
        .await
        .expect("react run task should join")
        .expect("react run should complete");
    assert_eq!(result.run_code, "f13d61b4-5c6e-5c62-8f48-100cba1df259");
    assert_eq!(result.final_output, "after tool steer");
    let requests = model_client.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].1.messages.iter().any(|message| {
        message.role == "tool" && message.text_content().contains("tool completed")
    }));
    assert_eq!(
        requests[1]
            .1
            .messages
            .iter().find(|message| message.role == "user" && message.plain_text_content() == "use the completed tool result differently")
            .expect("second request should contain steer input")
            .plain_text_content(),
        "use the completed tool result differently"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn framework_restores_responses_history_and_replays_changed_layout_after_restart() {
    use tron_backend::application::agent::FrameworkHost;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("framework.sqlite3");
    let host = fixture_host(FrameworkHost::persistent(&path, None));
    let first = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output_with_usage_and_response_id(
            "first answer",
            None,
            Some("resp-persisted".into()),
        ),
    )]));
    let agent = native_agent_builder(
        first,
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = base_context("24d910a1-ba3b-55a6-b27b-14fc9f21168b");
    context.model.protocol = AgentModelProtocol::Responses;
    context.model.responses_stateful_enabled = true;
    // Contributor data follows history. Inserting a new turn changes that
    // layout, so restore persisted history and safely replay instead of using
    // a provider response ID for a nonmatching prefix. Exact-prefix ID restore
    // is covered by turn_state::durable_continuation_rejects_edited_prefix_and_restores_exact_history.
    context.execution_scope = Default::default();
    context.variables.clear();
    context.skills.clear();
    context.tools.clear();
    agent.run(context.clone()).await.unwrap();
    host.close().await;
    drop(agent);
    drop(host);
    let host = fixture_host(FrameworkHost::persistent(&path, None));
    let second = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output_with_usage_and_response_id(
            "second answer",
            None,
            Some("resp-second".into()),
        ),
    )]));
    let agent = native_agent_builder(
        second.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    context.run_code = "5416d829-e857-539a-89f0-a745a8fc3476".into();
    context.messages = vec![AgentMessage::user("follow up")].into();
    agent.run(context.clone()).await.unwrap();
    let requests = second.requests();
    assert_eq!(
        requests[0].1.previous_response_id.as_deref(),
        None
    );
    assert!(
        requests[0]
            .1
            .messages
            .iter()
            .any(|message| message.plain_text_content() == "follow up")
    );
    assert!(
        requests[0]
            .1
            .messages
            .iter()
            .any(|message| message.plain_text_content().contains("first answer"))
    );
    // Same endpoint/model with another credential must not reuse the previous account ID.
    context.run_code = "36af60a0-7b16-500e-b443-6963d4618f4c".into();
    context.model.api_key = "another-account-key".into();
    context.messages = vec![AgentMessage::user("another account")].into();
    let account = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output_with_usage_and_response_id(
            "new account",
            None,
            Some("resp-account".into()),
        ),
    )]));
    let account_agent = native_agent_builder(
        account.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    account_agent.run(context.clone()).await.unwrap();
    assert!(account.requests()[0].1.previous_response_id.is_none());
    assert!(
        account.requests()[0]
            .1
            .messages
            .iter()
            .any(|m| m.plain_text_content().contains("first answer"))
    );
    // Changing the provider must replay the full transcript, not reuse its ID.
    context.run_code = "ad3ab75a-134b-592b-b9aa-fd24b60d3d30".into();
    context.model.base_url = "https://another-provider.invalid/v1".into();
    context.messages = vec![AgentMessage::user("different provider")].into();
    let switched = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output("switched"),
    )]));
    let agent = native_agent_builder(
        switched.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    agent.run(context).await.unwrap();
    assert!(switched.requests()[0].1.previous_response_id.is_none());
    assert!(
        switched.requests()[0]
            .1
            .messages
            .iter()
            .any(|message| message.plain_text_content().contains("first answer"))
    );
    host.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn framework_question_releases_session_and_resumes_durable_interaction() {
    use tron_backend::application::agent::FrameworkHost;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("questions.sqlite3");
    let host = fixture_host(FrameworkHost::persistent(&path, None));
    let registry = Arc::new(ToolRegistry::default());
    let client = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::tool_call(
            "chat_ask_user",
            json!({"intro":"Choose a target","questions":[{"question":"Where?","options":[{"label":"Local"},{"label":"Remote"}]}]}),
        ),
    )]));
    let agent = native_agent_builder(
        client.clone(),
        registry.clone(),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = base_context("84b2d51f-c1df-5326-85d6-c17c73a3e435");
    context.tools = vec![tron_backend::application::agent::ask_user_definition(
        "en-US",
    )];
    let waiting = agent.run(context.clone()).await.unwrap();
    assert_eq!(
        client.requests().len(),
        1,
        "asking a question must stop model iteration"
    );
    let question: serde_json::Value =
        serde_json::from_str(&waiting.observations[0].output).unwrap();
    let interaction = question["frameworkInteractionId"]
        .as_str()
        .expect("durable interaction identity");
    let request_code = interaction.to_owned();
    host.close().await;
    drop(agent);
    drop(host);
    let host = fixture_host(FrameworkHost::persistent(&path, None));
    let answer = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output("using local"),
    )]));
    let agent = native_agent_builder(
        answer.clone(),
        registry,
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    context.run_code = "d6f8f3c8-2d5a-5ab0-9cca-d7114e085be0".into();
    let mut follow_up = AgentMessage::user("Use Local");
    follow_up.parts.push(tron_backend::domain::agent::AgentMessagePart::Context {
        kind: "tron_interaction_response".into(), text: String::new(),
        metadata: Some(json!({"resume":[{"interruptId":request_code,"status":"resolved","payload":{"answers":[{"questionCode":question["questions"][0]["id"],"answerKind":"Option","optionCode":question["questions"][0]["options"][0]["id"]}]}}]})),
    });
    context.messages = vec![follow_up].into();
    let result = agent.run(context).await.unwrap();
    assert_eq!(result.final_output, "using local");
    let resumed_history = answer.requests()[0]
        .1
        .messages
        .iter()
        .map(|message| {
            format!(
                "{}\n{}",
                message.plain_text_content(),
                message.first_tool_output().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    // The public interaction resumes with its validated structured answer,
    // including the selected stable ID and the original question presentation.
    assert!(resumed_history.contains("Local"), "{resumed_history}");
    assert!(resumed_history.contains("\"answers\""), "{resumed_history}");
    assert!(
        resumed_history.contains(
            question["questions"][0]["options"][0]["id"]
                .as_str()
                .unwrap()
        ),
        "{resumed_history}"
    );
    assert_eq!(answer.requests().len(), 1);
    host.close().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn official_command_returns_after_ten_seconds_and_remains_readable_and_stoppable_across_runs()
{
    let directory = tempfile::tempdir().unwrap();
    let host =
        fixture_host(tron_backend::application::agent::FrameworkHost::temporary(None).unwrap());
    let check_host = host.clone();
    let checked = tokio::spawn(async move {
        let host = check_host;
        let mut context = base_context("c974ffcd-c1ae-5ab3-91dc-1ac96b9c84ed");
        // Real native command schema/history costs 9714 tokens, beyond the old 7496-token allocation.
        context.model.context_window_tokens = Some(16_384);
        context.execution_scope.local_directory = Some(directory.path().display().to_string());
        context.execution_scope.allowed_local_directories =
            vec![directory.path().display().to_string()];
        context.tools = ["bash_exec", "bash_task_read", "bash_task_stop"]
            .into_iter()
            .map(|name| EffectiveTool {
                tool_name: name.into(),
                domain_code: None,
                description: name.into(),
                input_schema: json!({"type":"object"}),
                skill_name: None,
                required_variable_keys: vec![],
            })
            .collect();
        let client = Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call(
                "bash_exec",
                json!({"command":"printf 'started\n'; sleep 60"}),
            ),
            ModelResponse::final_output("backgrounded"),
        ]));
        let event_sink = Arc::new(RecordingEventSink::with_route_class(
            "official-command",
            BusRouteClass::Agui,
        ));
        let events = EventBusRuntimeBuilder::new(
            EventBusConfig::default(),
            Arc::new(FixedSnowflakeIdProvider::new(8500)),
        )
        .register_sink(event_sink.clone())
        .build();
        let agent = native_agent_builder(client, Arc::new(ToolRegistry::default()), events)
            .with_framework_host(Some(host.clone()))
            .build()
            .unwrap();
        let start = std::time::Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(15), agent.run(context.clone()))
            .await
            .unwrap()
            .unwrap();
        assert!(start.elapsed() >= Duration::from_secs(9));
        let output: serde_json::Value =
            serde_json::from_str(&result.observations[0].output).unwrap();
        assert_eq!(output["status"], "running");
        assert!(output["stdout"].as_str().unwrap().contains("started"));
        let id = output["taskId"].as_str().unwrap().to_owned();
        assert!(
            event_sink
                .events()
                .iter()
                .any(|event| event.event_name == "agent.tool.activity")
        );
        for (index, name) in ["bash_task_read", "bash_task_stop"].into_iter().enumerate() {
            let client = Arc::new(ScriptedModelClient::new(vec![
                ModelResponse::tool_call(name, json!({"taskId":id})),
                ModelResponse::final_output("done"),
            ]));
            let agent = native_agent_builder(
                client,
                Arc::new(ToolRegistry::default()),
                tron_backend::shared::event_bus::EventBusHandle::disabled(),
            )
            .with_framework_host(Some(host.clone()))
            .build()
            .unwrap();
            context.run_code = eliteforge_agent_core::RunId::new_v4().to_string();
            context.messages = vec![AgentMessage::user(name)].into();
            let result = agent.run(context.clone()).await.unwrap();
            let output: serde_json::Value =
                serde_json::from_str(&result.observations[0].output).unwrap();
            assert_eq!(
                output["status"],
                if index == 0 { "running" } else { "cancelled" }
            );
        }
    })
    .await;
    let report = host.close().await;
    assert!(report.is_success(), "{report:?}");
    checked.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn framework_refreshes_host_instructions_without_losing_persisted_conversation() {
    use tron_backend::application::agent::FrameworkHost;
    let host =
        fixture_host(tron_backend::application::agent::FrameworkHost::temporary(None).unwrap());
    let mut context = base_context("a00bb4d1-c9c0-5304-825c-da979c0978f7");
    context.messages = vec![
        AgentMessage::system("old host instruction"),
        AgentMessage::user("first question"),
    ].into();
    let first = Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
        "first answer",
    )]));
    let agent = native_agent_builder(
        first,
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    agent.run(context.clone()).await.unwrap();
    context.run_code = "d1f5da0c-ec26-5546-a8ad-cfb090111d94".into();
    context.messages = vec![
        AgentMessage::system("new host instruction"),
        AgentMessage::user("second question"),
    ].into();
    let second = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output("second answer"),
    )]));
    let agent = native_agent_builder(
        second.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    agent.run(context).await.unwrap();
    let requests = second.requests();
    let messages = &requests[0].1.messages;
    assert_eq!(messages[0].plain_text_content(), "new host instruction");
    assert!(!messages.iter().any(|message| {
        message
            .plain_text_content()
            .contains("old host instruction")
    }));
    assert!(
        messages
            .iter()
            .any(|message| message.plain_text_content() == "first answer")
    );
    host.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn framework_retries_only_the_model_after_a_tool_side_effect() {
    let executions = Arc::new(AtomicUsize::new(0));
    let counter = executions.clone();
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            move |request: ToolRequest| {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "written-once".into(),
                })
            },
        )))
        .unwrap();
    let client = Arc::new(ScriptedResultModelClient::new(vec![
        Ok(ModelResponse::tool_call("echo", json!({"value":"once"}))),
        Err(
            eliteforge_agent_official_plugin_model_http::encode_model_error(
                &eliteforge_agent_core::ModelError::Transient(
                    "HTTP 503 temporarily unavailable".into(),
                ),
            ),
        ),
        Ok(ModelResponse::final_output("retried-model-only")),
    ]));
    let agent = native_agent_builder(
        client.clone(),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .unwrap();
    let mut context = base_context("20d49f43-9eaa-52d2-823d-22a4aea4ddad");
    context.model.max_retries = Some(1);
    let result = agent.run(context).await.unwrap();
    assert_eq!(result.final_output, "retried-model-only");
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let requests = client.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].1.messages, requests[2].1.messages);
}

#[derive(Clone)]
struct FloodThenFinalModel {
    calls: Arc<AtomicUsize>,
    dropped: Arc<std::sync::atomic::AtomicBool>,
}
struct FloodInvocationDrop(Arc<std::sync::atomic::AtomicBool>);
impl Drop for FloodInvocationDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
impl ModelClient for FloodThenFinalModel {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async { Ok(ModelResponse::final_output("recovered")) })
    }
    fn generate_stream<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
        sink: tron_backend::application::agent::ModelStreamSink,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                let _drop = FloodInvocationDrop(self.dropped.clone());
                for _ in 0..2048 {
                    sink(tron_backend::domain::agent::ModelStreamEvent::text_delta(
                        "x",
                    ));
                }
                std::future::pending::<Result<ModelResponse, String>>().await
            } else {
                Ok(ModelResponse::final_output("recovered"))
            }
        })
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
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn framework_stream_overflow_stops_unfinished_provider_without_retry_and_releases_session() {
    let calls = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let host =
        fixture_host(tron_backend::application::agent::FrameworkHost::temporary(None).unwrap());
    let agent = native_agent_builder(
        Arc::new(FloodThenFinalModel {
            calls: calls.clone(),
            dropped: dropped.clone(),
        }),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = base_context("b1a11ea9-45bc-53bd-8ad3-52053af44da2");
    context.tools.clear();
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        agent.run_stream(context.clone(), Arc::new(|_| {})),
    )
    .await
    .unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("bounded host handoff capacity")
    );
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "partial output must never be retried"
    );
    context.run_code = "41fcc393-9373-594a-a99a-8cdc7a533b2b".into();
    let result = agent.run_stream(context, Arc::new(|_| {})).await.unwrap();
    assert_eq!(result.final_output, "recovered");
    host.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn framework_main_rich_input_and_remaining_output_budget_reach_actual_http_body() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tron_backend::adapters::outbound::model::{HttpModelClient, HttpModelProviderClient};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut bodies = vec![];
        for index in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let (end, length) = loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < end + length {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let body: serde_json::Value =
                serde_json::from_slice(&bytes[end..end + length]).unwrap();
            bodies.push(body);
            let response = if index == 0 {
                json!({"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[{"id":"call-1","type":"function","function":{"name":"tool_execute","arguments":"{\"tool\":\"echo\",\"arguments\":{\"items\":[{}]}}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":30,"completion_tokens":34162}})
            } else {
                json!({"choices":[{"message":{"role":"assistant","content":"done"},"finish_reason":"stop"}],"usage":{"prompt_tokens":30,"completion_tokens":1}})
            };
            // The native host turn streams its model calls: serve SSE.
            let body = match response["choices"][0]["message"]["tool_calls"].as_array() {
                Some(calls) => {
                    let first = &calls[0];
                    let mut delta = json!({});
                    if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
                        delta["content"] = json!(content);
                    }
                    if let Some(call) = first.as_object() {
                        delta["tool_calls"] = json!([{
                            "index":0,
                            "id":call["id"].clone(),
                            "type":"function",
                            "function":call["function"].clone(),
                        }]);
                    }
                    format!(
                        "data: {}

data: {}

data: [DONE]

",
                        json!({"id":"fixture","object":"chat.completion.chunk","model":"gpt-test","choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
                        json!({"id":"fixture","object":"chat.completion.chunk","model":"gpt-test","choices":[{"index":0,"delta":{},"finish_reason":response["choices"][0]["finish_reason"].clone()}],"usage":response["usage"].clone()}),
                    )
                }
                None => format!(
                    "data: {}

data: {}

data: [DONE]

",
                    json!({"id":"fixture","object":"chat.completion.chunk","model":"gpt-test","choices":[{"index":0,"delta":{"content":response["choices"][0]["message"]["content"].clone()},"finish_reason":null}]}),
                    json!({"id":"fixture","object":"chat.completion.chunk","model":"gpt-test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":response["usage"].clone()}),
                ),
            };
            socket.write_all(format!("HTTP/1.1 200 OK
Content-Type: text/event-stream
Connection: close

{body}").as_bytes()).await.unwrap();
        }
        bodies
    });
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: "small result".into(),
                })
            },
        )))
        .unwrap();
    let client = Arc::new(HttpModelClient::new(Arc::new(
        HttpModelProviderClient::new(
            eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
        ),
    )));
    let agent = native_agent_builder(
        client.clone(),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .unwrap();
    let mut context = base_context("d23f0364-294e-5f9f-862f-dbd154138a10");
    context.model.base_url = url;
    // Leave enough context capacity for the complete protected image/tool turn.
    // The first response leaves 6,000 total tokens: enough for the next image
    // request, but less than its input plus the configured 4,096-token output.
    context.model.context_window_tokens = Some(16000);
    context.model.max_tokens = Some(4096);
    context.model.temperature = Some(0.25);
    context.model.top_p = Some(0.8);
    context.limits.max_steps = 2;
    context.messages=serde_json::from_value(json!([{"role":"user","parts":[{"type":"text","text":"inspect"},{"type":"image","mime_type":"image/png","data":"aW1hZ2U="},{"type":"context","kind":"attachment","text":"reference data"}]}])).unwrap();
    assert_eq!(agent.run(context).await.unwrap().final_output, "done");
    client.close().await;
    let bodies = server.await.unwrap();
    assert!(
        bodies[0]
            .to_string()
            .contains("data:image/png;base64,aW1hZ2U=")
    );
    assert!(bodies[0].to_string().contains("reference data"));
    assert_eq!(bodies[0]["temperature"], json!(0.25));
    assert_eq!(bodies[0]["top_p"], json!(0.8));
    assert_eq!(bodies[0]["max_tokens"], 4096);
    let remaining = bodies[1]["max_tokens"].as_u64().unwrap();
    assert!(
        remaining > 0 && remaining < 4096,
        "remaining output budget must reach actual provider body: {remaining}"
    );
}

fn fixture_host(
    host: tron_backend::application::agent::FrameworkHost,
) -> tron_backend::application::agent::FrameworkHost {
    use tron_backend::application::chat::ChatRepository;
    let mut repository = SqliteChatRepository::with_host(host.clone()).unwrap();
    for fixture_session in ["CONV-1", "SESSION-STEER", "SESSION-TOOL-STEER"] {
        if repository.get_session(fixture_session).is_err() {
            let session=serde_json::from_value(json!({"id":1,"sessionCode":fixture_session,"workspaceCode":"TRON_CORE","title":"runtime fixture","status":"ACTIVE","sourceType":"MANUAL","createdAt":"","updatedAt":""})).unwrap();
            repository.create_session(&session).unwrap();
        }
    }
    host
}
fn native_agent_builder(
    client: Arc<dyn ModelClient>,
    registry: Arc<ToolRegistry>,
    events: tron_backend::shared::event_bus::EventBusHandle,
) -> ReactAgentBuilder {
    ReactAgentBuilder::new(client, registry, events).with_framework_host(Some(fixture_host(
        tron_backend::application::agent::FrameworkHost::temporary(None).unwrap(),
    )))
}
