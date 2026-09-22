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
    adapters::outbound::persistence::{SqliteChatRepository, SqliteShortTermMemoryRepository},
    application::{
        agent::{ActiveRunRegistry, AgentSteerInput, ModelClient, ReactAgentBuilder},
        chat::{ChatService, CreateChatSessionCommand},
        short_term_memory::{
            ShortTermMemoryConfig, ShortTermMemoryRepository, ShortTermMemoryService,
            ToolOutputConfig,
        },
    },
    domain::{
        agent::{
            AgentError, AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig,
            AgentModelProtocol, AgentRunContext, AgentRunLimits, AgentShortTermMemoryRunContext,
            EffectiveSkill, EffectiveTool, EffectiveVariable, InProcessTool, ModelRequest,
            ModelResponse, Tool, ToolCall, ToolError, ToolExecutionResult, ToolObservation,
            ToolRegistry, ToolRequest,
        },
        short_term_memory::{
            ShortTermMemoryCompaction, ShortTermMemoryContextEpoch, ShortTermMemoryError,
            ShortTermMemoryEvent, ShortTermMemoryFamily, ShortTermMemoryFamilyMember,
            ShortTermMemoryFamilySnapshot, ShortTermMemoryGoal, ShortTermMemoryGoalCriterion,
            ShortTermMemoryGoalEvent, ShortTermMemoryGoalView, ShortTermMemoryInput,
            ShortTermMemoryMessage, ShortTermMemoryPart, ShortTermMemoryTodo,
        },
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
            Ok(guard.remove(0))
        })
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
            guard.remove(0)
        })
    }
}

#[derive(Clone, Default)]
struct ContextEpochFailingRepository {
    events: Arc<std::sync::Mutex<Vec<String>>>,
}

impl ContextEpochFailingRepository {
    fn events(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("events mutex should not be poisoned")
            .clone()
    }
}

impl ShortTermMemoryRepository for ContextEpochFailingRepository {
    fn session_workspace(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<String>, ShortTermMemoryError> {
        Ok(Some("TRON_CORE".into()))
    }

    fn next_id(&mut self, _table: &str) -> Result<i64, ShortTermMemoryError> {
        Ok(735_900_000_000_000_001)
    }

    fn next_seq(&mut self, _session_code: &str) -> Result<i64, ShortTermMemoryError> {
        Ok(1)
    }

    fn input_code_by_idempotency_key(
        &mut self,
        _session_code: &str,
        _idempotency_key: &str,
    ) -> Result<Option<String>, ShortTermMemoryError> {
        Ok(None)
    }

    fn create_input(&mut self, _input: &ShortTermMemoryInput) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn create_input_with_message_and_events(
        &mut self,
        _input: &ShortTermMemoryInput,
        _message: Option<&ShortTermMemoryMessage>,
        _parts: &[ShortTermMemoryPart],
        _events: &[ShortTermMemoryEvent],
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn list_inputs_for_run(
        &mut self,
        _session_code: &str,
        _run_code: &str,
    ) -> Result<Vec<ShortTermMemoryInput>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn create_event(&mut self, event: &ShortTermMemoryEvent) -> Result<(), ShortTermMemoryError> {
        self.events
            .lock()
            .expect("events mutex should not be poisoned")
            .push(event.event_type.clone());
        Ok(())
    }

    fn create_event_and_update_run_lease(
        &mut self,
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError> {
        self.create_event(event)
    }

    fn create_message(
        &mut self,
        _message: &ShortTermMemoryMessage,
        _parts: &[ShortTermMemoryPart],
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn create_message_with_event(
        &mut self,
        _message: &ShortTermMemoryMessage,
        _parts: &[ShortTermMemoryPart],
        _event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn create_message_with_events(
        &mut self,
        _message: &ShortTermMemoryMessage,
        _parts: &[ShortTermMemoryPart],
        _events: &[ShortTermMemoryEvent],
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn create_context_epoch(
        &mut self,
        _epoch: &ShortTermMemoryContextEpoch,
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn create_context_epoch_with_event(
        &mut self,
        _epoch: &ShortTermMemoryContextEpoch,
        _event: &ShortTermMemoryEvent,
    ) -> Result<ShortTermMemoryContextEpoch, ShortTermMemoryError> {
        Err(ShortTermMemoryError::StorageFailure)
    }

    fn latest_context_epoch_revision(
        &mut self,
        _session_code: &str,
    ) -> Result<i64, ShortTermMemoryError> {
        Ok(0)
    }

    fn latest_context_epoch_snapshot(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<serde_json::Value>, ShortTermMemoryError> {
        Ok(None)
    }

    fn list_messages_after_seq(
        &mut self,
        _session_code: &str,
        _min_seq: i64,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn list_messages_for_run(
        &mut self,
        _session_code: &str,
        _run_code: &str,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn list_recent_messages(
        &mut self,
        _session_code: &str,
        _min_seq: i64,
        _limit: usize,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn message_parts(
        &mut self,
        _message_code: &str,
    ) -> Result<Vec<ShortTermMemoryPart>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn list_todos(
        &mut self,
        _session_code: &str,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn replace_todos(
        &mut self,
        _session_code: &str,
        _workspace_code: &str,
        _todos: &[ShortTermMemoryTodo],
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn replace_todos_with_event(
        &mut self,
        _session_code: &str,
        _workspace_code: &str,
        _todos: &[ShortTermMemoryTodo],
        _event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn latest_compaction(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<ShortTermMemoryCompaction>, ShortTermMemoryError> {
        Ok(None)
    }

    fn create_compaction(
        &mut self,
        _compaction: &ShortTermMemoryCompaction,
        _message: &ShortTermMemoryMessage,
        _parts: &[ShortTermMemoryPart],
        _event: Option<&ShortTermMemoryEvent>,
        _enforce_idle: bool,
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn active_run_exists(&mut self, _session_code: &str) -> Result<bool, ShortTermMemoryError> {
        Ok(false)
    }

    fn upsert_family_memberships(
        &mut self,
        _families: &[ShortTermMemoryFamily],
        _members: &[ShortTermMemoryFamilyMember],
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn list_enabled_family_members(
        &mut self,
        _session_code: &str,
    ) -> Result<Vec<(ShortTermMemoryFamily, ShortTermMemoryFamilyMember)>, ShortTermMemoryError>
    {
        Ok(Vec::new())
    }

    fn list_completed_family_snapshots(
        &mut self,
        _family_code: &str,
        _exclude_session_code: &str,
        _limit: usize,
    ) -> Result<Vec<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn session_summary_for_run(
        &mut self,
        _session_code: &str,
        _run_code: &str,
    ) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
        Ok(None)
    }

    fn latest_completed_session_summary(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
        Ok(None)
    }

    fn latest_completed_family_handoff(
        &mut self,
        _family_code: &str,
        _session_code: &str,
    ) -> Result<Option<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
        Ok(None)
    }

    fn list_family_session_codes(
        &mut self,
        _family_code: &str,
        _exclude_session_code: &str,
        _limit: usize,
    ) -> Result<Vec<String>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn create_family_snapshots(
        &mut self,
        _snapshots: &[ShortTermMemoryFamilySnapshot],
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn recover_stale_session_summaries(
        &mut self,
        _completed_at: &str,
    ) -> Result<usize, ShortTermMemoryError> {
        Ok(0)
    }

    fn ensure_hidden_heartbeat_session(
        &mut self,
        _workspace_code: &str,
        _agent_code: &str,
        session_code: &str,
        _id: i64,
        _now: &str,
    ) -> Result<String, ShortTermMemoryError> {
        Ok(session_code.to_string())
    }

    fn get_active_goal(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        Ok(None)
    }

    fn get_latest_goal(
        &mut self,
        _session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        Ok(None)
    }

    fn list_open_goals_batch(
        &mut self,
        _after_id: i64,
        _limit: usize,
    ) -> Result<Vec<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn get_goal(
        &mut self,
        _goal_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        Ok(None)
    }

    fn list_goal_events(
        &mut self,
        _goal_code: &str,
        _limit: usize,
    ) -> Result<Vec<ShortTermMemoryGoalEvent>, ShortTermMemoryError> {
        Ok(Vec::new())
    }

    fn append_goal_event(
        &mut self,
        _event: &ShortTermMemoryGoalEvent,
    ) -> Result<(), ShortTermMemoryError> {
        Ok(())
    }

    fn create_goal(
        &mut self,
        _goal: &ShortTermMemoryGoal,
        _criteria: &[ShortTermMemoryGoalCriterion],
        _event: &ShortTermMemoryGoalEvent,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        Err(ShortTermMemoryError::GoalInvalid)
    }

    fn update_goal(
        &mut self,
        _goal: &ShortTermMemoryGoal,
        _criteria: &[ShortTermMemoryGoalCriterion],
        _event: &ShortTermMemoryGoalEvent,
        _expected_revision: i64,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        Err(ShortTermMemoryError::GoalInvalid)
    }

    fn recover_stale_runs(&mut self, _limit: usize) -> Result<usize, ShortTermMemoryError> {
        Ok(0)
    }

    fn cleanup_event_log_before(
        &mut self,
        _cutoff_created_at_ms: i64,
        _batch_size: usize,
        _max_rows: usize,
    ) -> Result<usize, ShortTermMemoryError> {
        Ok(0)
    }

    fn list_externalized_tool_output_paths(
        &mut self,
        _root_path: &str,
    ) -> Result<Vec<String>, ShortTermMemoryError> {
        Ok(Vec::new())
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
        tool_exposure: Default::default(),
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
        messages: vec![AgentMessage::user("hello")],
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

fn auto_compaction_fixture(
    label: &str,
    id_seed: i64,
) -> (
    String,
    String,
    ShortTermMemoryService,
    AgentRunContext,
    String,
) {
    let database_url = temp_database_url(label);
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some(label.into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config
        .context
        .blocks
        .current_session_checkpoint
        .max_ratio_bps = None;
    config.context.blocks.current_session_checkpoint.max_tokens = Some(300);
    config.compaction.trigger_ratio_bps = 4_000;
    config.compaction.retained_history_ratio_bps = 2_000;

    let short_term_memory = ShortTermMemoryService::with_id_provider(
        repository,
        config,
        Arc::new(FixedSnowflakeIdProvider::new(id_seed)),
    );
    let mut old_context = base_context(&format!("RUN-{label}-OLD"));
    old_context.conversation_code = Some(session.session_code.clone());
    old_context.workspace_code = session.workspace_code.clone();
    for index in 0..5 {
        old_context.run_code = format!("RUN-AUTO-OLD-{index}");
        short_term_memory
            .record_user_input(
                &old_context,
                &format!("old auto context {index} {}", "A".repeat(2_500)),
            )
            .expect("old input should be recorded");
    }

    let mut context = base_context(&format!("RUN-{label}-CURRENT"));
    context.conversation_code = Some(session.session_code.clone());
    context.workspace_code = session.workspace_code.clone();
    context.model.context_window_tokens = Some(8_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let current_prompt = format!("current auto input stays raw {}", "B".repeat(200));
    context.messages = vec![AgentMessage::user(current_prompt.clone())];
    let input_code = short_term_memory
        .record_user_input(&context, &current_prompt)
        .expect("current input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    (
        database_url,
        session.session_code,
        short_term_memory,
        context,
        current_prompt,
    )
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "done",
        )])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(base_context("RUN-FINAL"))
        .await
        .expect("final answer run should pass");

    assert_eq!(result.final_output, "done");
    assert_eq!(result.step_count, 1);
    assert!(sink.wait_for_len(6, Duration::from_secs(2)));
    let events = sink.events();
    let names: Vec<_> = events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "agent.run.requested",
            "agent.run.started",
            "agent.model.requested",
            "agent.model.responded",
            "agent.output.created",
            "agent.run.completed",
        ]
    );
    assert!(events.iter().all(|event| {
        event
            .payload_fields
            .iter()
            .any(|field| field.key == "model_name" && field.value == "gpt-test")
    }));
    for event in events {
        let serialized = format!("{:?}", event);
        assert!(!serialized.contains("hello"));
        assert!(!serialized.contains("done"));
        assert!(!serialized.contains("sk-test"));
        assert!(!serialized.contains("prompt"));
        assert!(!serialized.contains("completion"));
    }
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "done",
        )])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("RUN-AUTO");
    context.model.model_code = "PERSON_AI:ROUTE:ChatAuto".into();
    context.model.model_name.clear();

    let result = agent
        .run_stream(context, Arc::new(|_event| {}))
        .await
        .expect("Auto run should succeed without a physical model name");

    assert_eq!(result.final_output, "done");
    assert!(sink.wait_for_len(8, Duration::from_secs(2)));
    let events = sink.events();
    assert!(
        events
            .iter()
            .any(|event| event.event_name == "agent.output.started")
    );
    assert!(
        events
            .iter()
            .any(|event| event.event_name == "agent.output.delta")
    );
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

#[tokio::test]
async fn react_agent_recovers_context_overflow_with_checkpoint_retry() {
    let database_url = temp_database_url("agent-runtime-overflow-retry");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("Overflow recovery".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let config = ShortTermMemoryConfig::default();

    let short_term_memory = ShortTermMemoryService::new(repository, config);
    let mut old_context = base_context("RUN-OVERFLOW-OLD");
    old_context.conversation_code = Some(session.session_code.clone());
    old_context.workspace_code = session.workspace_code.clone();
    for index in 0..5 {
        old_context.run_code = format!("RUN-OVERFLOW-OLD-{index}");
        short_term_memory
            .record_user_input(
                &old_context,
                &format!("old overflow context {index} {}", "x".repeat(1_500)),
            )
            .expect("old input should be recorded");
    }
    let mut context = base_context("RUN-OVERFLOW-RECOVERY");
    context.conversation_code = Some(session.session_code.clone());
    context.workspace_code = session.workspace_code.clone();
    context.model.context_window_tokens = Some(8_000);
    context.model.max_tokens = Some(0);
    context.messages = vec![AgentMessage::user("trigger overflow recovery")];
    let input_code = short_term_memory
        .record_user_input(&context, "trigger overflow recovery")
        .expect("input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let model_client = Arc::new(ScriptedResultModelClient::new(vec![
        Err("context_length_exceeded: too many tokens".into()),
        Ok(ModelResponse::final_output(
            "## Goal\nRecover from overflow\n\n## Decisions\nModel-generated overflow checkpoint.",
        )),
        Ok(ModelResponse::final_output("recovered after checkpoint")),
    ]));
    let agent = ReactAgentBuilder::new(
        model_client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_short_term_memory(Some(short_term_memory.clone()))
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(context)
        .await
        .expect("overflow checkpoint retry should recover");
    assert_eq!(result.final_output, "recovered after checkpoint");
    let requests = model_client.requests();
    assert_eq!(requests.len(), 3);
    assert!(requests[1].1.tools.is_empty());
    assert!(requests[1].1.previous_response_id.is_none());
    assert_eq!(requests[1].0.temperature, Some(0.0));
    assert_eq!(requests[1].0.max_tokens, Some(4_096));

    let connection = rusqlite::Connection::open(&database_url)
        .expect("short-term memory database should reopen");
    let compaction_metadata: String = connection
        .query_row(
            "SELECT metadata_json
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND message_type = 'compaction'
             ORDER BY seq DESC
             LIMIT 1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("overflow checkpoint should be persisted");
    assert!(compaction_metadata.contains("OVERFLOW_CHECKPOINT"));
    assert!(compaction_metadata.contains("provider_context_overflow"));
    assert!(compaction_metadata.contains("\"compactionMode\":\"model\""));
    assert!(compaction_metadata.contains("\"coverage\":\"head_only_tail_replayed\""));
    assert!(compaction_metadata.contains("\"retainedHistoryRatioBps\":2000"));
    assert!(compaction_metadata.contains("\"effectiveRetainedHistoryTokens\""));
}

#[tokio::test]
async fn react_agent_auto_compaction_calls_model_and_pins_current_input() {
    let database_url = temp_database_url("agent-runtime-auto-model-compaction");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("Auto model compaction".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config
        .context
        .blocks
        .current_session_checkpoint
        .max_ratio_bps = None;
    config.context.blocks.current_session_checkpoint.max_tokens = Some(300);
    config.compaction.trigger_ratio_bps = 4_000;
    config.compaction.retained_history_ratio_bps = 2_000;

    let short_term_memory = ShortTermMemoryService::with_id_provider(
        repository,
        config,
        Arc::new(FixedSnowflakeIdProvider::new(801_000_000_000_000_000)),
    );

    let mut old_context = base_context("RUN-AUTO-OLD");
    old_context.conversation_code = Some(session.session_code.clone());
    old_context.workspace_code = session.workspace_code.clone();
    for index in 0..5 {
        old_context.run_code = format!("RUN-AUTO-OLD-{index}");
        short_term_memory
            .record_user_input(
                &old_context,
                &format!("old auto context {index} {}", "A".repeat(2_500)),
            )
            .expect("old input should be recorded");
    }

    let mut context = base_context("RUN-AUTO-CURRENT");
    context.conversation_code = Some(session.session_code.clone());
    context.workspace_code = session.workspace_code.clone();
    context.model.context_window_tokens = Some(8_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let current_prompt = format!("current auto input stays raw {}", "B".repeat(500));
    context.messages = vec![AgentMessage::user(current_prompt.clone())];
    let input_code = short_term_memory
        .record_user_input(&context, &current_prompt)
        .expect("current input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let model_client = Arc::new(ScriptedResultModelClient::new(vec![
        Ok(ModelResponse::final_output(
            "## Goal\nMODEL AUTO SUMMARY\n\n## Decisions\nAuto compaction used semantic model output.",
        )),
        Ok(ModelResponse::final_output("done after auto compaction")),
    ]));
    let agent = ReactAgentBuilder::new(
        model_client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_short_term_memory(Some(short_term_memory.clone()))
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(context)
        .await
        .expect("auto model compaction should recover budget");
    assert_eq!(result.final_output, "done after auto compaction");
    let requests = model_client.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].1.tools.is_empty());
    assert_eq!(requests[0].0.temperature, Some(0.0));
    assert_eq!(requests[0].0.max_tokens, Some(4_096));
    let final_request_text = requests[1]
        .1
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");
    assert!(final_request_text.contains("MODEL AUTO SUMMARY"));
    assert!(final_request_text.contains(&current_prompt));

    let connection = rusqlite::Connection::open(&database_url)
        .expect("short-term memory database should reopen");
    let (summary, metadata): (String, String) = connection
        .query_row(
            "SELECT p.content_text, m.metadata_json
             FROM agent_transcript_messages m
             JOIN agent_transcript_parts p ON p.message_code = m.message_code
             WHERE m.session_code = ?1
               AND m.message_type = 'compaction'
               AND p.part_type = 'compaction_summary'
             ORDER BY m.seq DESC
             LIMIT 1",
            [session.session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("model checkpoint should be persisted");
    assert!(summary.contains("MODEL AUTO SUMMARY"));
    assert!(metadata.contains("\"compactionMode\":\"model\""));
    assert!(metadata.contains("\"coverage\":\"head_only_tail_replayed\""));
    assert!(metadata.contains("\"selectorVersion\":4"));
    assert!(metadata.contains("\"retainedHistoryRatioBps\":2000"));
    assert!(metadata.contains("\"effectiveRetainedHistoryTokens\""));
    assert!(metadata.contains("\"pinnedStartSeq\""));
}

#[tokio::test]
async fn react_agent_auto_compaction_model_failures_do_not_write_checkpoint() {
    let cases = vec![
        (
            "auto-model-request-failed",
            Err("compaction upstream unavailable".to_string()),
            "COMPACTION_MODEL_REQUEST_FAILED",
        ),
        (
            "auto-model-empty-output",
            Ok(ModelResponse::final_output("")),
            "COMPACTION_MODEL_EMPTY_OUTPUT",
        ),
        (
            "auto-model-tool-call",
            Ok(ModelResponse::tool_call("unexpected.tool", json!({}))),
            "COMPACTION_MODEL_TOOL_CALL",
        ),
    ];

    for (index, (label, response, expected_code)) in cases.into_iter().enumerate() {
        let (database_url, session_code, short_term_memory, context, _) =
            auto_compaction_fixture(label, 802_000_000_000_000_000 + index as i64 * 1_000);
        let model_client = Arc::new(ScriptedResultModelClient::new(vec![response]));
        let agent = ReactAgentBuilder::new(
            model_client,
            Arc::new(ToolRegistry::default()),
            tron_backend::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_short_term_memory(Some(short_term_memory))
        .build()
        .expect("agent builder should succeed");

        agent
            .run(context)
            .await
            .expect_err("compaction model failure should fail the run");

        let connection = rusqlite::Connection::open(&database_url)
            .expect("short-term memory database should reopen");
        let completed_checkpoints: i64 = connection
            .query_row(
                "SELECT count(1)
                 FROM agent_transcript_messages
                 WHERE session_code = ?1
                   AND message_type = 'compaction'
                   AND status = 'completed'",
                [session_code.as_str()],
                |row| row.get(0),
            )
            .expect("checkpoint count should be queryable");
        assert_eq!(completed_checkpoints, 0, "{label}");
        let completed_events: i64 = connection
            .query_row(
                "SELECT count(1)
                 FROM agent_transcript_events
                 WHERE session_code = ?1
                   AND event_type = 'agent.compaction.completed'",
                [session_code.as_str()],
                |row| row.get(0),
            )
            .expect("completed event count should be queryable");
        assert_eq!(completed_events, 0, "{label}");
        let failed_payload: String = connection
            .query_row(
                "SELECT payload_json
                 FROM agent_transcript_events
                 WHERE session_code = ?1
                   AND event_type = 'agent.compaction.failed'
                 ORDER BY seq DESC
                 LIMIT 1",
                [session_code.as_str()],
                |row| row.get(0),
            )
            .expect("failed event should be persisted");
        assert!(failed_payload.contains(expected_code), "{label}");
    }
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
        .run(base_context("RUN-TOOL"))
        .await
        .expect("tool run should pass");
    let second = agent
        .run(base_context("RUN-SECOND"))
        .await
        .expect("second run should pass");

    assert_eq!(first.final_output, "after-tool");
    assert_eq!(first.observations.len(), 1);
    assert_eq!(second.final_output, "second-run");
    assert!(second.observations.is_empty());
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("flaky", json!({"value": "x"})),
            ModelResponse::final_output("after-failure"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run({
            let mut context = base_context("RUN-TOOL-FAILURE");
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
    assert!(sink.wait_for_len(12, Duration::from_secs(2)));
    let events = sink.events();
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
    assert!(
        events
            .iter()
            .any(|event| event.event_name == "agent.run.completed")
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_name == "agent.run.failed")
    );
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(base_context("RUN-EMPTY-FINAL-OUTPUT"))
        .await
        .expect_err("an empty final response must fail the run");

    assert!(matches!(
        error,
        AgentError::EmptyFinalOutput(message)
            if message.contains("fs_write_text: tool input is invalid: function arguments are missing")
    ));
    assert!(sink.wait_for_len(11, Duration::from_secs(2)));
    let event_names = sink
        .events()
        .into_iter()
        .map(|event| event.event_name)
        .collect::<Vec<_>>();
    assert!(event_names.contains(&"agent.tool.failed".to_string()));
    assert!(event_names.contains(&"agent.run.failed".to_string()));
    assert!(!event_names.contains(&"agent.run.completed".to_string()));
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
            let mut context = base_context("RUN-EMPTY-QUERY-RESULT");
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

#[tokio::test]
async fn react_agent_persists_failed_tool_observation_in_short_term_memory() {
    let database_url = temp_database_url("agent-runtime-failed-stm");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("Failed STM tool".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let short_term_memory =
        ShortTermMemoryService::new(repository, ShortTermMemoryConfig::default());
    let mut context = base_context("RUN-FAILED-STMEM");
    context.conversation_code = Some(session.session_code.clone());
    context.workspace_code = session.workspace_code.clone();
    context.messages = vec![AgentMessage::user("run failing short-term memory tool")];
    context.tools.push(EffectiveTool {
        tool_name: "flaky".into(),
        domain_code: None,
        description: "Flaky".into(),
        input_schema: json!({"type": "object"}),
        skill_name: Some("planner".into()),
        required_variable_keys: vec![],
    });
    let input_code = short_term_memory
        .record_user_input(&context, "run failing short-term memory tool")
        .expect("input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "flaky",
            |_request: ToolRequest| Err(ToolError::ExecutionFailed("boom".into())),
        )))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call_with_id("call-flaky", "flaky", json!({"value": "x"})),
            ModelResponse::final_output("after-failure"),
        ])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_short_term_memory(Some(short_term_memory))
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(context)
        .await
        .expect("tool execution failure should be converted to an observation");
    assert_eq!(result.final_output, "after-failure");

    let connection = rusqlite::Connection::open(&database_url)
        .expect("short-term memory database should reopen");
    let tool_started_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.tool.started'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool started event should be queryable");
    let tool_completed_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.tool.completed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool completed event should be queryable");
    let tool_failed_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.tool.failed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool failed event should be queryable");
    let failed_part_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_parts WHERE session_code = ?1 AND part_type = 'tool_result' AND status = 'failed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("failed tool result part should be queryable");
    assert_eq!(tool_started_count, 1);
    assert_eq!(tool_completed_count, 0);
    assert_eq!(tool_failed_count, 1);
    assert_eq!(failed_part_count, 1);
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
        .run(base_context("RUN-MULTI-TOOL"))
        .await
        .expect("multi tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(result.observations[1].output, "observed:\"b\"");
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
        agent.run(base_context("RUN-PARALLEL")),
    )
    .await
    .expect("parallel tool calls should complete within timeout")
    .expect("parallel tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(result.observations[1].output, "observed:\"b\"");
    assert_eq!(max_running.load(Ordering::SeqCst), 2);
}

#[tokio::test]
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
        .register(Arc::new(InProcessTool::new("hidden.tool", {
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
    let agent = ReactAgentBuilder::new(
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
                    tool_name: "hidden.tool".into(),
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
        .run(base_context("RUN-PARALLEL-UNAUTHORIZED"))
        .await
        .expect("an unauthorized parallel tool should not abort the run");

    assert_eq!(result.final_output, "recovered after parallel rejection");
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.observations[0].output, "echo completed");
    assert_eq!(result.observations[1].tool_name, "hidden.tool");
    assert!(result.observations[1].output.contains("is not allowed"));
    assert_eq!(hidden_executions.load(Ordering::SeqCst), 0);
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
        .run(base_context("RUN-SINGLE-TOOL"))
        .await
        .expect("single tool run should pass");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations.len(), 1);
    assert_eq!(result.observations[0].output, "observed:\"a\"");
    assert_eq!(max_running.load(Ordering::SeqCst), 1);
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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

    let mut context = base_context("RUN-BATCHED-PARALLEL");
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

#[tokio::test]
async fn react_agent_externalizes_parallel_tool_outputs_with_unique_sequences() {
    let database_url = temp_database_url("agent-runtime-parallel-stm");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("Parallel STM".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let artifact_root = temp_artifact_root("agent-runtime-parallel-stm");
    let mut memory_config = ShortTermMemoryConfig::default();
    memory_config.tool_output = ToolOutputConfig {
        root: artifact_root.display().to_string(),
        externalize_above_bytes: 8,
    };
    let short_term_memory = ShortTermMemoryService::new(repository, memory_config);
    let mut context = base_context("RUN-PARALLEL-STMEM");
    context.conversation_code = Some(session.session_code.clone());
    context.workspace_code = session.workspace_code.clone();
    context.messages = vec![AgentMessage::user("parallel short-term memory tools")];
    let input_code = short_term_memory
        .record_user_input(&context, "parallel short-term memory tools")
        .expect("input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new(
            "echo",
            move |request: ToolRequest| {
                Ok(ToolObservation {
                    tool_name: request.tool_name,
                    output: format!(
                        "large output for {}\nsecond line\nthird line",
                        request.input["value"]
                    ),
                })
            },
        )))
        .expect("tool should register");
    let agent = ReactAgentBuilder::new(
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
    .with_short_term_memory(Some(short_term_memory))
    .build()
    .expect("agent builder should succeed");

    let result = agent
        .run(context)
        .await
        .expect("parallel short-term memory run should pass");
    assert_eq!(result.final_output, "after-tool");

    let connection = rusqlite::Connection::open(&database_url)
        .expect("short-term memory database should reopen");
    let mut statement = connection
        .prepare(
            "SELECT content_json
             FROM agent_transcript_parts
             WHERE session_code = ?1
               AND part_type = 'tool_result'
             ORDER BY id ASC",
        )
        .expect("tool result parts should be queryable");
    let relative_paths = statement
        .query_map([session.session_code.as_str()], |row| {
            let content: String = row.get(0)?;
            let value: serde_json::Value = serde_json::from_str(&content).unwrap();
            Ok(value["metadata"]["relativePath"]
                .as_str()
                .unwrap()
                .to_string())
        })
        .expect("tool result paths should map")
        .collect::<Result<Vec<_>, _>>()
        .expect("tool result paths should load");
    assert_eq!(relative_paths.len(), 2);
    assert_ne!(relative_paths[0], relative_paths[1]);
    assert!(relative_paths[0].contains("0001-echo.txt"));
    assert!(relative_paths[1].contains("0002-echo.txt"));
    for relative_path in relative_paths {
        assert!(
            artifact_root.join(relative_path).exists(),
            "externalized output file should exist"
        );
    }
    let tool_started_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.tool.started'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool started events should be queryable");
    let tool_completed_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.tool.completed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool completed events should be queryable");
    let tool_failed_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.tool.failed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool failed events should be queryable");
    let failed_part_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_parts WHERE session_code = ?1 AND part_type = 'tool_result' AND status = 'failed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool result status should be queryable");
    assert_eq!(tool_started_count, 2);
    assert_eq!(tool_completed_count, 2);
    assert_eq!(tool_failed_count, 0);
    assert_eq!(failed_part_count, 0);
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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

    let mut context = base_context("RUN-PARALLEL-BUDGET");
    context.limits.max_tool_calls = 3;
    context.limits.max_parallel_tool_calls = 2;
    let error = agent
        .run(context)
        .await
        .expect_err("run should fail after exhausting total tool-call budget");

    assert_eq!(error.to_string(), "agent run exceeded max tool calls");
    assert_eq!(executed.load(Ordering::SeqCst), 3);
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"command": long_command})),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    agent
        .run(base_context("RUN-TOOL-META"))
        .await
        .expect("tool run should pass");

    assert!(sink.wait_for_len(11, Duration::from_secs(2)));
    let events = sink.events();
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

    assert_eq!(
        payload("tool_call_id", tool_started).as_deref(),
        Some("RUN-TOOL-META-tool-1")
    );
    assert_eq!(payload("tool_name", tool_started).as_deref(), Some("echo"));
    assert_eq!(
        payload("tool_args_json", tool_started).as_deref(),
        Some(expected_args.as_str())
    );
    assert_eq!(
        payload("tool_call_id", tool_completed).as_deref(),
        Some("RUN-TOOL-META-tool-1")
    );
    assert_eq!(
        payload("tool_result_json", tool_completed).as_deref(),
        Some(expected_result.as_str())
    );
    assert_eq!(
        payload("tool_call_id", observation).as_deref(),
        Some("RUN-TOOL-META-tool-1")
    );
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("progress", json!({})),
            ModelResponse::final_output("after-progress"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("RUN-TOOL-PROGRESS");
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
    assert!(sink.wait_for_len(13, Duration::from_secs(2)));
    let events = sink.events();
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
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
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
    .build()
    .expect("agent builder should succeed");

    let mut context = base_context("RUN-PARALLEL-TOOL-PROGRESS");
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
    for tool_call_id in [
        "RUN-PARALLEL-TOOL-PROGRESS-tool-1",
        "RUN-PARALLEL-TOOL-PROGRESS-tool-2",
    ] {
        let lines = activity_events
            .iter()
            .filter(|event| payload("tool_call_id", event).as_deref() == Some(tool_call_id))
            .map(|event| payload("tool_result_json", event).unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines.contains(&"first progress line".to_string()));
        assert!(lines.contains(&"second progress line".to_string()));
    }
}

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![
            ModelResponse::tool_call("echo", json!({"command": "large"})),
            ModelResponse::final_output("after-tool"),
        ])),
        Arc::new(registry),
        event_bus,
    )
    .build()
    .expect("agent builder should succeed");

    // Exercise event truncation without also overflowing the model input budget.
    let mut context = base_context("RUN-TOOL-OVERSIZE");
    context.model.context_window_tokens = Some(32_000);
    let result = agent
        .run(context)
        .await
        .expect("oversize tool event payload should not fail the run");

    assert_eq!(result.final_output, "after-tool");
    assert_eq!(result.observations[0].output, oversized_output);
    assert!(sink.wait_for_len(11, Duration::from_secs(2)));
    let events = sink.events();
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
}

#[tokio::test]
async fn tool_registry_rejects_missing_or_unauthorized_tools() {
    let registry = ToolRegistry::default();
    let context = base_context("RUN-DENIED");

    let missing = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "echo".into(),
                input: json!({}),
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
    let mut context = base_context("RUN-EFFECTIVE-HIDDEN");
    context.tools = vec![discovery.clone()];
    context.effective_tools = vec![discovery, write_text];

    let execution = registry
        .execute_allowed(
            ToolRequest {
                tool_name: "fs_write_text".into(),
                input: json!({}),
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

#[tokio::test]
async fn react_agent_recovers_after_missing_and_unauthorized_tools() {
    for (label, tool_name, register_tool, allow_tool, expected_fragment) in [
        ("missing", "missing.tool", false, true, "is not registered"),
        ("unauthorized", "hidden.tool", true, false, "is not allowed"),
    ] {
        let database_url = temp_database_url(&format!("agent-runtime-stm-tool-{label}"));
        let chats = ChatService::new(
            SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
        );
        let session = chats
            .create_session(
                CreateChatSessionCommand {
                    workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                    title: Some(format!("STM tool {label}")),
                    app_code: None,
                    task_code: None,
                    source_type: None,
                    source_code: None,
                    agent_code: Some("main-agent".into()),
                },
                Some("USR-1".into()),
            )
            .expect("session should be created");
        let short_term_memory = ShortTermMemoryService::new(
            SqliteShortTermMemoryRepository::connect(&database_url)
                .expect("short-term memory repository should connect"),
            ShortTermMemoryConfig::default(),
        );
        let mut context = base_context(&format!("RUN-STM-TOOL-{label}"));
        context.conversation_code = Some(session.session_code.clone());
        context.workspace_code = session.workspace_code.clone();
        context.messages = vec![AgentMessage::user("trigger tool failure")];
        context.tools = if allow_tool {
            vec![EffectiveTool {
                tool_name: tool_name.into(),
                domain_code: None,
                description: format!("{tool_name} test tool"),
                input_schema: json!({"type": "object"}),
                skill_name: Some("planner".into()),
                required_variable_keys: Vec::new(),
            }]
        } else {
            Vec::new()
        };
        let input_code = short_term_memory
            .record_user_input(&context, "trigger tool failure")
            .expect("input should be recorded");
        context.short_term_memory = Some(AgentShortTermMemoryRunContext {
            session_code: session.session_code.clone(),
            input_code,
            input_kind: "USER_PROMPT".into(),
            input_metadata: None,
            prompt_prefix: vec![AgentMessage::system("system seed")],
            current_input_message: None,
            agent_code: Some("main-agent".into()),
            todo_enabled: false,
        });

        let registry = ToolRegistry::default();
        if register_tool {
            registry
                .register(Arc::new(InProcessTool::new(tool_name, |request| {
                    Ok(ToolObservation {
                        tool_name: request.tool_name,
                        output: "hidden".into(),
                    })
                })))
                .expect("hidden tool should register");
        }
        let final_output = format!("recovered after {label} tool");
        let agent = ReactAgentBuilder::new(
            Arc::new(ScriptedModelClient::new(vec![
                ModelResponse::tool_call(tool_name, json!({})),
                ModelResponse::final_output(final_output.clone()),
            ])),
            Arc::new(registry),
            tron_backend::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_short_term_memory(Some(short_term_memory))
        .build()
        .expect("agent builder should succeed");

        let result = agent
            .run(context)
            .await
            .expect("a rejected tool invocation should be recoverable by the model");
        assert_eq!(result.final_output, final_output, "{label}");
        assert_eq!(result.observations.len(), 1, "{label}");
        assert_eq!(result.observations[0].tool_name, tool_name, "{label}");
        assert!(
            result.observations[0].output.contains(expected_fragment),
            "{label}: {}",
            result.observations[0].output
        );

        let connection = rusqlite::Connection::open(&database_url)
            .expect("short-term memory database should reopen");
        let tool_started_count: i64 = connection
            .query_row(
                "SELECT count(1)
                 FROM agent_transcript_events
                 WHERE session_code = ?1
                   AND event_type = 'agent.tool.started'",
                [session.session_code.as_str()],
                |row| row.get(0),
            )
            .expect("tool started event should be queryable");
        let tool_failed_count: i64 = connection
            .query_row(
                "SELECT count(1)
                 FROM agent_transcript_events
                 WHERE session_code = ?1
                   AND event_type = 'agent.tool.failed'",
                [session.session_code.as_str()],
                |row| row.get(0),
            )
            .expect("tool failed event should be queryable");
        let (status, visibility, content_text, content_json): (String, String, String, String) =
            connection
                .query_row(
                    "SELECT status, visibility, content_text, content_json
                 FROM agent_transcript_parts
                 WHERE session_code = ?1
                   AND part_type = 'tool_result'
                 ORDER BY id DESC
                 LIMIT 1",
                    [session.session_code.as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("failed tool result should be queryable");
        let tool_call_visibility: String = connection
            .query_row(
                "SELECT visibility
                 FROM agent_transcript_parts
                 WHERE session_code = ?1
                   AND part_type = 'tool_call'
                 ORDER BY id DESC
                 LIMIT 1",
                [session.session_code.as_str()],
                |row| row.get(0),
            )
            .expect("failed tool call should remain in model-visible memory");
        assert_eq!(tool_started_count, 1, "{label}");
        assert_eq!(tool_failed_count, 1, "{label}");
        assert_eq!(status, "failed", "{label}");
        assert_eq!(visibility, "model", "{label}");
        assert_eq!(tool_call_visibility, "model", "{label}");
        assert!(
            content_text.contains(expected_fragment),
            "{label}: {content_text}"
        );
        let parsed_content: serde_json::Value =
            serde_json::from_str(&content_json).expect("tool result content JSON should parse");
        assert_eq!(parsed_content["toolName"], json!(tool_name), "{label}");
        let call_id = parsed_content["callId"]
            .as_str()
            .expect("failed tool result should keep generated call id");
        assert!(call_id.starts_with("generated_call_"), "{label}: {call_id}");
        assert_eq!(parsed_content["modelCallId"], json!(call_id), "{label}");
    }
}

#[tokio::test]
async fn model_request_builders_use_frozen_model_config() {
    let config = base_context("RUN-MODEL").model;
    let request = ModelRequest {
        messages: vec![AgentMessage::user("hi")],
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

#[tokio::test]
async fn max_step_exhaustion_fails_without_output_success() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
            max_parallel_tool_calls: 1,
        },
        ..base_context("RUN-LIMIT")
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
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::tool_call(
            "echo",
            json!({}),
        )])),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(context)
        .await
        .expect_err("tool loop should exhaust max steps");

    assert_eq!(error.to_string(), "agent run exceeded max steps");
}

#[tokio::test]
async fn react_agent_marks_short_term_memory_run_failed_when_prepare_persistence_fails() {
    let repository = ContextEpochFailingRepository::default();
    let event_probe = repository.clone();
    let short_term_memory =
        ShortTermMemoryService::new(repository, ShortTermMemoryConfig::default());
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "unreachable",
        )])),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_short_term_memory(Some(short_term_memory))
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("RUN-STM-PREPARE-FAIL");
    context.conversation_code = Some("SESSION-STM-PREPARE-FAIL".into());
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: "SESSION-STM-PREPARE-FAIL".into(),
        input_code: "ATI-1".into(),
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: Vec::new(),
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let error = agent
        .run(context)
        .await
        .expect_err("prepare persistence failure should fail the run");

    assert_eq!(error.to_string(), "short-term memory storage failure");
    let events = event_probe.events();
    assert!(events.contains(&"agent.run.started".to_string()));
    assert!(events.contains(&"agent.run.failed".to_string()));
}

#[tokio::test]
async fn zero_tool_call_limit_is_rejected_before_model_execution() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 0,
            max_parallel_tool_calls: 1,
        },
        ..base_context("RUN-ZERO-TOOL")
    };
    let agent = ReactAgentBuilder::new(
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

#[tokio::test]
async fn zero_parallel_tool_call_limit_is_rejected_before_model_execution() {
    let context = AgentRunContext {
        limits: AgentRunLimits {
            max_steps: 1,
            max_tool_calls: 1,
            max_parallel_tool_calls: 0,
        },
        ..base_context("RUN-ZERO-PARALLEL")
    };
    let agent = ReactAgentBuilder::new(
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

#[tokio::test]
async fn react_agent_consumes_steer_at_final_model_boundary_without_starting_a_second_run() {
    let active_runs = ActiveRunRegistry::new();
    let guard = active_runs
        .register("SESSION-STEER", "RUN-STEER", "REACT", true)
        .expect("active run should register");
    let control = active_runs
        .control("SESSION-STEER", "RUN-STEER")
        .expect("react run control should resolve");
    let model_client = BlockingFirstResponseModelClient::new();
    let started = model_client.started.clone();
    let release = model_client.release.clone();
    let agent = ReactAgentBuilder::new(
        Arc::new(model_client.clone()),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("RUN-STEER");
    context.conversation_code = Some("SESSION-STEER".into());
    let started_signal = started.notified();
    tokio::pin!(started_signal);
    let run = tokio::spawn(async move { agent.run_with_control(context, Some(control)).await });

    started_signal.await;
    active_runs
        .accept_steer("SESSION-STEER", "RUN-STEER", || {
            Ok::<_, ()>((
                (),
                AgentSteerInput {
                    steer_code: "MSG-STEER-1".into(),
                    content: "change direction now".into(),
                    attachments: Vec::new(),
                },
            ))
        })
        .expect("steer should route to the active run")
        .expect("steer acceptance should succeed");
    release.notify_one();

    let result = run
        .await
        .expect("react run task should join")
        .expect("react run should complete");
    assert_eq!(result.run_code, "RUN-STEER");
    assert_eq!(result.step_count, 2);
    assert_eq!(
        result.final_output,
        "first boundary outputsteered final output"
    );
    let requests = model_client.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert!(request.messages.iter().any(|message| {
            message.role == "system"
                && message
                    .plain_text_content()
                    .contains("unresolved earlier user requests")
        }));
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
            .last()
            .expect("second request should contain steer input")
            .plain_text_content(),
        "change direction now"
    );
    drop(guard);
}

#[tokio::test]
async fn react_agent_preserves_unresolved_requests_when_steering_with_short_term_memory() {
    let database_url = temp_database_url("agent-runtime-steer-memory");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("Steer memory".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USR-1".into()),
        )
        .expect("session should be created");
    let memory_repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let short_term_memory =
        ShortTermMemoryService::new(memory_repository, ShortTermMemoryConfig::default());

    let active_runs = ActiveRunRegistry::new();
    let guard = active_runs
        .register(&session.session_code, "RUN-STEER-MEMORY", "REACT", true)
        .expect("active run should register");
    let control = active_runs
        .control(&session.session_code, "RUN-STEER-MEMORY")
        .expect("react run control should resolve");
    let model_client = BlockingFirstResponseModelClient::new();
    let started = model_client.started.clone();
    let release = model_client.release.clone();
    let agent = ReactAgentBuilder::new(
        Arc::new(model_client.clone()),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_short_term_memory(Some(short_term_memory.clone()))
    .build()
    .expect("agent builder should succeed");

    let original_request = "summarize today's five trending AI projects";
    let mut context = base_context("RUN-STEER-MEMORY");
    context.conversation_code = Some(session.session_code.clone());
    context.workspace_code = session.workspace_code.clone();
    context.messages = vec![AgentMessage::user(original_request)];
    let input_code = short_term_memory
        .record_user_input(&context, original_request)
        .expect("original request should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let started_signal = started.notified();
    tokio::pin!(started_signal);
    let run = tokio::spawn(async move { agent.run_with_control(context, Some(control)).await });

    started_signal.await;
    active_runs
        .accept_steer(&session.session_code, "RUN-STEER-MEMORY", || {
            Ok::<_, ()>((
                (),
                AgentSteerInput {
                    steer_code: "MSG-STEER-MEMORY-1".into(),
                    content: "what time is it?".into(),
                    attachments: Vec::new(),
                },
            ))
        })
        .expect("steer should route to the active run")
        .expect("steer acceptance should succeed");
    release.notify_one();

    let result = run
        .await
        .expect("react run task should join")
        .expect("react run should complete");
    assert_eq!(result.step_count, 2);

    let requests = model_client.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert!(request.messages.iter().any(|message| {
            message.role == "system"
                && message
                    .plain_text_content()
                    .contains("unresolved earlier user requests")
        }));
        assert!(request.messages.iter().any(|message| {
            message.role == "user" && message.plain_text_content() == original_request
        }));
    }
    assert!(requests[1].messages.iter().any(|message| {
        message.role == "user" && message.plain_text_content() == "what time is it?"
    }));
    drop(guard);
}

#[tokio::test]
async fn react_agent_waits_for_current_tool_step_before_injecting_steer() {
    let active_runs = ActiveRunRegistry::new();
    let _guard = active_runs
        .register("SESSION-TOOL-STEER", "RUN-TOOL-STEER", "REACT", true)
        .expect("active run should register");
    let control = active_runs
        .control("SESSION-TOOL-STEER", "RUN-TOOL-STEER")
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
    let agent = ReactAgentBuilder::new(
        Arc::new(model_client.clone()),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .expect("agent builder should succeed");
    let mut context = base_context("RUN-TOOL-STEER");
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
        .accept_steer("SESSION-TOOL-STEER", "RUN-TOOL-STEER", || {
            Ok::<_, ()>((
                (),
                AgentSteerInput {
                    steer_code: "MSG-STEER-TOOL".into(),
                    content: "use the completed tool result differently".into(),
                    attachments: Vec::new(),
                },
            ))
        })
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
    assert_eq!(result.run_code, "RUN-TOOL-STEER");
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
            .last()
            .expect("second request should contain steer input")
            .plain_text_content(),
        "use the completed tool result differently"
    );
}
