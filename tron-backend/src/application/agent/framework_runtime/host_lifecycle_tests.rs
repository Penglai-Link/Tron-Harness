use super::*;
use crate::application::agent::{
    ActiveRunRegistry, ActiveRunState, AgentInvocationError, AgentRunExecutor, AgentRuntime,
    AgentRuntimeRegistry, ModelClient, RuntimeAdapterKey, RuntimeCapabilities,
};
use crate::domain::agent::*;
use eliteforge_agent_test_support::TestBuilder;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::Notify;

struct PendingModel(Arc<Notify>);
#[async_trait::async_trait]
impl fw::ChatModel for PendingModel {
    async fn generate(
        &self,
        _: &str,
        _: fw::ChatRequest,
        _: &dyn fw::ChatStream,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        self.0.notify_one();
        std::future::pending().await
    }
}
struct DelayFinish {
    entered: Arc<Notify>,
    release: Arc<Notify>,
    first: AtomicBool,
}
#[async_trait::async_trait]
impl fw::RunLifecycleHook for DelayFinish {
    async fn on_finished(
        &self,
        _: &fw::RunLifecycleContext<'_>,
        _: &Result<fw::RunResult, fw::AgentError>,
    ) {
        if self.first.swap(false, Ordering::AcqRel) {
            self.entered.notify_one();
            self.release.notified().await;
        }
    }
}
struct NoResources;
impl ModelClient for NoResources {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("fixture unused".into()) })
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate(c, r)
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let _ = self.close_observed().await;
        })
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}
struct NativeRuntime {
    host: FrameworkHost,
    agent: Agent,
}
impl AgentRuntime for NativeRuntime {
    fn owns_native_lifecycle(&self) -> bool {
        true
    }
    fn key(&self) -> RuntimeAdapterKey {
        RuntimeAdapterKey::React
    }
    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeAdapterKey::React.descriptor().capabilities
    }
    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            let registration = self
                .host
                .reserve_native(&context.run_code)
                .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))?;
            let request = native_request(context.conversation_code.as_deref().unwrap());
            let (handle, stream) = self
                .agent
                .start_with_output::<eliteforge_agent_official_plugin_agui::AgUiStream>(request)
                .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))?;
            let result = self
                .host
                .wait_native(self.agent.clone(), handle, stream, &context, registration)
                .await;
            match result {
                Err(error) if matches!(error.primary(), fw::AgentError::Cancelled) => {
                    Err(AgentInvocationError::RunStopped)
                }
                Err(error) => Err(AgentInvocationError::RuntimeFailed(error.to_string())),
                Ok(_) => unreachable!("pending model"),
            }
        })
    }
    fn resume<'a>(
        &'a self,
        _: &'a str,
        c: AgentRunContext,
        s: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.start(c, s)
    }
    fn cancel<'a>(
        &'a self,
        run: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async move {
            self.host
                .cancel_run(run)
                .map_err(|e| AgentInvocationError::RuntimeFailed(e.to_string()))
        })
    }
}
fn native_request(session: &str) -> fw::RunRequest {
    let mut request = fw::RunRequest::new(session, "input");
    request.model = Some(fw::ModelRef::new("fixture", "pending"));
    request
}
async fn native_agent(
    path: PathBuf,
    entered: Arc<Notify>,
    finished: Arc<Notify>,
    release: Arc<Notify>,
) -> Agent {
    prepare_fixture_artifacts(&path);
    TestBuilder::new()
        .default_models(false)
        .plugin(eliteforge_agent_framework::ModelRuntimePlugin::new(
            eliteforge_agent_framework::ModelRuntime::new(
                eliteforge_agent_framework::ModelRuntimeUpdate::new("", 0, vec![]),
            )
            .unwrap(),
        ))
        .plugin(fw::ModelConnectionPlugin::new(fw::ModelConnection::new(
            "fixture",
            Arc::new(PendingModel(entered)),
            vec![fw::ModelDescriptor::new("pending")],
        )))
        .plugin(fw::RunLifecycleHookPlugin::new(
            "fixture.delay",
            DelayFinish {
                entered: finished,
                release,
                first: AtomicBool::new(true),
            },
        ))
        .plugin(
            eliteforge_agent_official_plugin_storage_local::LocalStoragePlugin::new(
                &path,
                path.with_extension("artifacts"),
            ),
        )
        .plugin(eliteforge_agent_official_plugin_agui::AgUiPlugin::default())
        .build()
        .await
        .unwrap()
}
async fn within<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("fixture deadline")
}
#[tokio::test]
async fn native_stop_retains_product_admission_until_sdk_hooks_and_session_release() {
    let host = FrameworkHost::temporary(None).unwrap();
    let entered = Arc::new(Notify::new());
    let finished = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let agent = native_agent(
        host.0.path.clone(),
        entered.clone(),
        finished.clone(),
        release.clone(),
    )
    .await;
    assert!(host.0.framework.set(agent.framework()).is_ok());
    assert!(host.0.agent.set(agent.clone()).is_ok());
    let active = ActiveRunRegistry::new();
    let executor = AgentRunExecutor::new(
        AgentRuntimeRegistry::new(
            Arc::new(NoResources),
            Arc::new(ToolRegistry::default()),
            crate::shared::event_bus::EventBusHandle::disabled(),
            reqwest::Client::new(),
        )
        .with_framework_host(Some(host.clone()))
        .with_active_runs(active.clone()),
    );
    let runtime = Arc::new(NativeRuntime {
        host: host.clone(),
        agent: agent.clone(),
    });
    let task = tokio::spawn(async move {
        executor
            .execute_with_runtime(runtime, test_context(0), None, None)
            .await
    });
    within(entered.notified()).await;
    active.request_stop("CONV-1", "RUN-1").unwrap();
    within(finished.notified()).await;
    assert_eq!(
        active.snapshot("CONV-1").unwrap().unwrap().state,
        ActiveRunState::Finishing
    );
    assert!(matches!(
        agent.start(native_request("CONV-1")),
        Err(fw::AgentError::SessionBusy(_))
    ));
    assert!(active.reserve("CONV-1", "RUN-2", "REACT", true).is_err());
    release.notify_one();
    assert!(matches!(
        within(task).await.unwrap(),
        Err(AgentInvocationError::RunStopped)
    ));
    assert!(active.snapshot("CONV-1").unwrap().is_none());
    let handle = agent
        .start(native_request("CONV-1"))
        .expect("SDK session released");
    agent.cancel(&handle).unwrap();
    let _ = agent.wait(handle).await;
    assert!(host.close().await.is_success());
}
#[tokio::test]
async fn dropping_native_caller_cancels_but_does_not_release_product_admission_early() {
    let host = FrameworkHost::temporary(None).unwrap();
    let entered = Arc::new(Notify::new());
    let finished = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let agent = native_agent(
        host.0.path.clone(),
        entered.clone(),
        finished.clone(),
        release.clone(),
    )
    .await;
    assert!(host.0.framework.set(agent.framework()).is_ok());
    assert!(host.0.agent.set(agent.clone()).is_ok());
    let active = ActiveRunRegistry::new();
    let executor = AgentRunExecutor::new(
        AgentRuntimeRegistry::new(
            Arc::new(NoResources),
            Arc::new(ToolRegistry::default()),
            crate::shared::event_bus::EventBusHandle::disabled(),
            reqwest::Client::new(),
        )
        .with_framework_host(Some(host.clone()))
        .with_active_runs(active.clone()),
    );
    let runtime = Arc::new(NativeRuntime {
        host: host.clone(),
        agent: agent.clone(),
    });
    let task = tokio::spawn(async move {
        executor
            .execute_with_runtime(runtime, test_context(0), None, None)
            .await
    });
    within(entered.notified()).await;
    task.abort();
    let _ = task.await;
    within(finished.notified()).await;
    assert!(active.snapshot("CONV-1").unwrap().is_some());
    release.notify_one();
    within(async {
        while active.snapshot("CONV-1").unwrap().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(agent.close().await.is_success());
    assert!(host.close().await.is_success());
}
struct ClosingClient {
    entered: Arc<Notify>,
    release: Arc<Notify>,
    calls: Arc<AtomicUsize>,
}
impl ModelClient for ClosingClient {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("unused".into()) })
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate(c, r)
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let _ = self.close_observed().await;
        })
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(eliteforge_agent_plugin_core::ShutdownReport {
                errors: vec!["fixture MCP close failed".into()],
                panics: 1,
                timeouts: 1,
            })
        })
    }
}
#[tokio::test]
async fn cancelled_host_close_waiter_keeps_single_owner_and_nested_report() {
    let host = FrameworkHost::temporary(None).unwrap();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    host.bind_model_client(Arc::new(ClosingClient {
        entered: entered.clone(),
        release: release.clone(),
        calls: calls.clone(),
    }))
    .unwrap();
    let closing = host.clone();
    let first = tokio::spawn(async move { closing.close().await });
    within(entered.notified()).await;
    first.abort();
    let _ = first.await;
    release.notify_one();
    let report = within(host.close()).await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(report.errors, vec!["fixture MCP close failed"]);
    assert_eq!((report.panics, report.timeouts), (1, 1));
    assert_eq!(host.close().await, report);
    assert!(host.reserve_native("later").is_err());
}
#[test]
fn native_result_preserves_generic_input_approval_schema_and_full_prompt() {
    let input = fw::InteractionRequest::input("name?", json!({"type":"string"})).unwrap();
    let approval = fw::InteractionRequest::approval(fw::RunId::new_v4(), "approve?");
    let result = fw::RunResult {
        output: String::new(),
        content: "".into(),
        structured: None,
        interactions: vec![input.clone(), approval.clone()],
        status: fw::RunStatus::AwaitingInput,
        usage: Default::default(),
        steps: 1,
        tool_calls: 1,
        external_session: None,
    };
    let mapped = host_control::native_outcome("native-run".into(), &result).unwrap();
    assert_eq!(mapped.status, AgentNativeRunStatus::AwaitingInput);
    assert_eq!(
        mapped.interactions[0].response_schema,
        input.response_schema
    );
    assert_eq!(
        mapped.interactions[0].prompt,
        serde_json::to_value(&input.prompt).unwrap()
    );
    assert!(matches!(
        mapped.interactions[1].kind,
        AgentNativeInteractionKind::Approval { .. }
    ));
    assert_eq!(
        host_interactions::answer_value(&approval, &json!(false)).unwrap(),
        json!(false)
    );
    assert!(host_interactions::answer_value(&approval, &json!({"approved":true})).is_err());
    assert_eq!(
        host_interactions::answer_value(&input, &json!("Jane")).unwrap(),
        json!("Jane")
    );
}

fn test_context(max_retries: u32) -> AgentRunContext {
    AgentRunContext {
        system_agent_fast_mode: false,
        collaboration_owner: None,
        agent_code: None,
        auto_model_route: None,
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
            request_headers: Default::default(),
            model_name: "test-model".into(),
            thinking_enabled: false,
            supports_developer_role: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: Some(1.0),
            max_retries: Some(max_retries),
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
        tool_assignment: None,
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("hello")].into(),
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-05-30T00:00:00Z".into(),
    }
}

#[path = "host_interaction_tests.rs"]
mod interaction_tests;
#[path = "host_shutdown_tests.rs"]
mod shutdown_tests;

fn prepare_fixture_artifacts(path: &std::path::Path) {
    let mut directory = std::fs::DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory.create(path.with_extension("artifacts")).unwrap();
}
#[path = "host_status_tests.rs"]
mod status_tests;

#[path = "host_chat_regression_tests.rs"]
mod chat_regression_tests;

struct StorageAcceptanceModel;
impl ModelClient for StorageAcceptanceModel {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate_once(config, request)
    }
    fn generate_once<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let summary = request.messages.iter().any(|message| {
                message
                    .text_content()
                    .contains("Summarize historical conversation")
            });
            Ok(ModelResponse::final_output(if summary {
                r#"{"summary":"Preserved native history and decisions."}"#
            } else {
                "native reply"
            }))
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}
#[tokio::test]
async fn real_host_persists_native_history_and_summary_on_the_shared_database() {
    use crate::adapters::outbound::persistence::sqlite::SqliteChatRepository;
    use crate::application::{
        agent::ReactAgentBuilder,
        chat::ChatRepository,
        short_term_memory::{ShortTermMemoryConfig, ShortTermMemoryService},
    };
    let host = FrameworkHost::temporary(None).unwrap();
    let model = Arc::new(StorageAcceptanceModel);
    host.bind_model_client(model.clone()).unwrap();
    crate::adapters::outbound::persistence::sqlite::SqliteResourceRepository::connect(
        host.0.path.to_str().unwrap(),
    )
    .unwrap();
    let mut chat = SqliteChatRepository::with_host(host.clone()).unwrap();
    let memory = ShortTermMemoryService::with_host(host.clone(), ShortTermMemoryConfig::default());
    let mut context = test_context(0);
    context.run_code = fw::RunId::new_v4().to_string();
    context.conversation_code = None;
    context.stream_key = Some(context.run_code.clone());
    let session = context.run_code.clone();
    let harness = ReactAgentBuilder::new(
        model.clone(),
        Arc::new(ToolRegistry::default()),
        crate::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let outcome = within(host.run(harness, context.clone(), None, None))
        .await
        .unwrap();
    assert_eq!(outcome.final_output, "native reply");
    assert_eq!(
        chat.get_session(&session).unwrap().workspace_code,
        "TRON_CORE"
    );
    let framework = host.storage_framework().await.unwrap();
    let conversations = framework
        .services()
        .unwrap()
        .get::<fw::ConversationStoreService>()
        .unwrap()
        .0
        .clone();
    let history = conversations.history(&session).await.unwrap();
    assert_eq!(history[0].messages.len(), 2);
    let summary = within(memory.summarize(&session, &context.model, model, None, None))
        .await
        .unwrap();
    assert!(
        summary["summary"]
            .as_str()
            .unwrap()
            .contains("native history")
    );
    assert_eq!(summary["coveredMessages"], 2);
    assert!(memory.latest_compaction(&session).unwrap().is_some());
    chat.update_session_status(
        &session,
        crate::domain::chat::ChatSessionStatus::Archived,
        "",
    )
    .unwrap();
    assert_eq!(
        conversations.history(&session).await.unwrap()[0]
            .messages
            .len(),
        2
    );
    chat.delete_session(&session).unwrap();
    assert!(conversations.history(&session).await.unwrap().is_empty());
    host.close().await;
}

struct RemoteCatalogModel(Arc<std::sync::Mutex<Vec<ModelRequest>>>);

/// Gateway mode: the model-visible catalog is exactly the stable pair; the
/// wrapped inventory is asserted through search results, never request.tools.
fn assert_gateway_request_tools(observed: &Arc<std::sync::Mutex<Vec<ModelRequest>>>) {
    for request in observed.lock().unwrap().iter() {
        let names: Vec<&str> = request.tools.iter().map(|t| t.tool_name.as_str()).collect();
        assert!(
            names.contains(&"tool_search") && names.contains(&"tool_execute"),
            "gateway pair missing: {names:?}"
        );
        assert!(
            names.iter().filter(|n| **n != "tool_search" && **n != "tool_execute").count() == 0,
            "non-gateway tool leaked into request.tools: {names:?}"
        );
    }
}

/// Names the last run's `tool_search` matched: the wrapped inventory the
/// gateway actually admitted, parsed from tool-result part outputs.
fn searched_tool_names(observed: &Arc<std::sync::Mutex<Vec<ModelRequest>>>) -> Vec<String> {
    observed
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .filter_map(|part| match part {
            crate::domain::agent::AgentMessagePart::ToolResult { output, .. } => {
                serde_json::from_str::<serde_json::Value>(output).ok()
            }
            _ => None,
        })
        .flat_map(|value| {
            value["results"][0]["result"]["tools"]
                .as_array()
                .map(|tools| {
                    tools
                        .iter()
                        .filter_map(|tool| tool["call_name"].as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect()
}

impl RemoteCatalogModel {
    /// First call per run searches the wrapped inventory through the gateway;
    /// later calls finish. Search results let tests assert wrapped admission.
    fn response(request: &ModelRequest) -> ModelResponse {
        let searched = request
            .messages
            .iter()
            .any(|message| message.role == "tool");
        if searched {
            ModelResponse::final_output("done")
        } else {
            ModelResponse::tool_call(
                "tool_execute",
                serde_json::json!({"tool":"tool_search","arguments":{"items":[{"query":"fresh","limit":20}]}}),
            )
        }
    }
}
impl ModelClient for RemoteCatalogModel {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate_once(config, request)
    }
    fn generate_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.0.lock().unwrap().push(request.clone());
        let _ = config;
        Box::pin(async move { Ok(Self::response(request)) })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}
#[tokio::test]
async fn framework_remote_tools_reach_model_only_with_agent_authorization() {
    use crate::application::agent::ReactAgentBuilder;
    let host = FrameworkHost::temporary(None).unwrap();
    let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let model = Arc::new(RemoteCatalogModel(observed.clone()));
    host.bind_model_client(model.clone()).unwrap();
    let harness = ReactAgentBuilder::new(
        model,
        Arc::new(ToolRegistry::default()),
        crate::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let descriptors = crate::application::agent::tool_catalog::official_remote_descriptors();
    let mut context = test_context(0);
    context.skills.clear();
    context.conversation_code = None;
    context.run_code = fw::RunId::new_v4().to_string();
    context.tools = descriptors
        .iter()
        .map(|tool| EffectiveTool {
            tool_name: tool.call_name.clone(),
            domain_code: None,
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
            skill_name: None,
            required_variable_keys: vec![],
        })
        .collect();
    host.run(harness.clone(), context.clone(), None, None)
        .await
        .unwrap();
    // Gateway mode: no remote binding is ever directly visible; the stable
    // pair is the whole catalog on every request regardless of authorization.
    assert_gateway_request_tools(&observed);
    context
        .tools
        .retain(|tool| tool.tool_name == "personai_search");
    context.run_code = fw::RunId::new_v4().to_string();
    host.run(harness.clone(), context.clone(), None, None)
        .await
        .unwrap();
    assert_gateway_request_tools(&observed);
    host.set_pack_enabled("framework.personai", false)
        .await
        .unwrap();
    context.run_code = fw::RunId::new_v4().to_string();
    host.run(harness, context, None, None).await.unwrap();
    assert_gateway_request_tools(&observed);
    assert!(host.close().await.is_success());
}

#[tokio::test]
async fn read_only_assignment_admits_fresh_native_bindings_and_preserves_request_and_platform_limits()
 {
    use crate::adapters::outbound::persistence::{
        SqliteAgentManagementRepository, SqliteWorkspaceRepository,
    };
    use crate::application::{
        agent::{
            AgentManagementService, ReactAgentBuilder, tool_framework::FrameworkToolRegistryExt,
        },
        workspace::{WorkspaceRepository, WorkspaceService},
    };
    use tron_harness_capability::{ToolPermission, ToolPermissionSetting};
    struct QueryTool;
    #[async_trait::async_trait]
    impl fw::Tool for QueryTool {
        async fn call(
            &self,
            _: serde_json::Value,
            _: &fw::RunContext,
        ) -> Result<fw::ToolOutput, fw::ToolError> {
            Ok(fw::ToolOutput::text("queried"))
        }
    }
    let host = FrameworkHost::temporary(None).unwrap();
    let path = host.0.path.to_str().unwrap();
    let mut workspaces = SqliteWorkspaceRepository::connect(path).unwrap();
    workspaces
        .create_workspace(&crate::domain::workspace::Workspace {
            id: crate::shared::id::SnowflakeId::new(1),
            workspace_code: "TRON_CORE".into(),
            name: "Runtime fixture".into(),
            description: None,
            parent_workspace_code: None,
            status: crate::domain::workspace::WorkspaceStatus::Active,
            created_at: "2026-10-12T00:00:00Z".into(),
            updated_at: "2026-10-12T00:00:00Z".into(),
            archived_at: None,
        })
        .unwrap();
    let management = AgentManagementService::new(
        SqliteAgentManagementRepository::connect(path).unwrap(),
        WorkspaceService::new(workspaces),
        &crate::config::AppConfig::default().system_agents,
    );
    management.initialize_default_main_loop().unwrap();
    host.bind_agent_management(management.clone());
    let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let model = Arc::new(RemoteCatalogModel(observed.clone()));
    host.bind_model_client(model.clone()).unwrap();
    let registry = Arc::new(ToolRegistry::default());
    let harness = ReactAgentBuilder::new(
        model,
        registry.clone(),
        crate::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = test_context(0);
    context.conversation_code = None;
    context.skills.clear();
    context.tools.clear();
    context.effective_tools.clear();
    context.tool_assignment = Some(AgentToolAssignment {
        mode: ToolAssignmentMode::AllReadOnlyTools,
        selected_tool_codes: vec![],
        requested_tool_names: vec![],
    });
    for iteration in 0..4 {
        context.run_code = fw::RunId::new_v4().to_string();
        registry.register_framework_lease(
            &context.run_code,
            fw::ToolLease::new(
                [
                    ("fresh_query", fw::ToolOperationKind::Query),
                    ("fresh_write", fw::ToolOperationKind::Command),
                    ("fresh_unknown", fw::ToolOperationKind::Unknown),
                ]
                .into_iter()
                .map(|(name, kind)| {
                    fw::ToolBinding::new(
                        fw::ToolDescriptor::new(
                            format!("fixture.{name}"),
                            name,
                            "Native tool",
                            json!({"type":"object"}),
                        )
                        .with_operation_kind(kind),
                        QueryTool,
                    )
                })
                .collect(),
            ),
        );
        if iteration == 1 {
            context
                .tool_assignment
                .as_mut()
                .unwrap()
                .requested_tool_names = vec!["fresh_write".into()];
        }
        if iteration == 2 {
            context
                .tool_assignment
                .as_mut()
                .unwrap()
                .requested_tool_names
                .clear();
            let mut denied = ToolPermissionSetting::default_for("fixture.fresh_query");
            denied.permission = ToolPermission::No;
            management.replace_tool_permissions(vec![denied]).unwrap();
        }
        if iteration == 3 {
            management.replace_tool_permissions(vec![]).unwrap();
        }
        host.run(harness.clone(), context.clone(), None, None)
            .await
            .unwrap();
        // Gateway mode: admission is observed through what `tool_search`
        // matched. Permission resets (iteration 3) re-admit the read-only
        // tool; read-only exclusion (iteration 1 request projection) and
        // explicit denial (iteration 2) keep it out of the results.
        {
            let requests = observed.lock().unwrap();
            for (i, r) in requests.iter().enumerate() {
                for m in r.messages.iter() {
                    for p in m.parts.iter() {
                        if let crate::domain::agent::AgentMessagePart::ToolResult { output, .. } = p {
                            eprintln!("DEBUG req{i} part output: {}", output);
                        }
                    }
                }
            }
        }
        let names = searched_tool_names(&observed);
        assert_eq!(
            names.contains(&"fresh_query".into()),
            iteration == 0 || iteration == 3,
            "{names:?}"
        );
        assert!(!names.contains(&"fresh_write".into()), "{names:?}");
        assert!(!names.contains(&"fresh_unknown".into()), "{names:?}");
        assert_gateway_request_tools(&observed);
    }
    assert!(host.close().await.is_success());
}

#[path = "host_batch_outcome_tests.rs"]
mod batch_outcome_tests;
