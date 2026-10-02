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
            guard.remove(0)
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

    fn create_compaction_at_revision(
        &mut self,
        compaction: &ShortTermMemoryCompaction,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        event: Option<&ShortTermMemoryEvent>,
        enforce_idle: bool,
        _: &eliteforge_agent_memory::MemoryRevision,
    ) -> Result<(), ShortTermMemoryError> {
        self.create_compaction(compaction, message, parts, event, enforce_idle)
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
        prompt_prefix_sources: Vec::new(),
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

// Observe official SDK output through the production owned consumer and real SQLite.
fn native_presentation_fixture() -> (
    tron_backend::application::agent::FrameworkHost,
    tron_backend::application::agent::AgentAguiStreamService,
) {
    use tron_backend::{
        adapters::outbound::persistence::SqliteAgentEventReplayStore,
        application::agent::AgentAguiStreamService, shared::event_bus::AguiEventSink,
    };
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        Arc::new(SqliteAgentEventReplayStore::connect(":memory:").unwrap()),
    );
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
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
    let (native_host, presentation) = native_presentation_fixture();
    let agent = ReactAgentBuilder::new(
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
        .run(base_context("RUN-FINAL"))
        .await
        .expect("final answer run should pass");

    assert_eq!(result.final_output, "done");
    assert_eq!(result.step_count, 1);
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "RUN-FINAL");
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
    let (native_host, presentation) = native_presentation_fixture();
    let agent = ReactAgentBuilder::new(
        Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
            "done",
        )])),
        Arc::new(ToolRegistry::default()),
        event_bus,
    )
    .with_framework_host(Some(native_host))
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
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "RUN-AUTO");
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
    context.model.context_window_tokens = Some(40_000);
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
        prompt_prefix_sources: Vec::new(),
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let model_client = Arc::new(ScriptedResultModelClient::new(vec![
        Err(
            eliteforge_agent_official_plugin_model_http::encode_model_error(
                &eliteforge_agent_core::ModelError::ContextOverflow(
                    "context_length_exceeded: too many tokens".into(),
                ),
            ),
        ),
        Ok(ModelResponse::final_output(
            r#"{"summary":"Recover from overflow. Model-generated overflow checkpoint."}"#,
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
    assert!(
        requests[1]
            .0
            .max_tokens
            .is_some_and(|limit| limit > 0 && limit <= 4_096)
    );

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
    config.compaction.trigger_ratio_bps = 2_000;
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
    context.model.context_window_tokens = Some(40_000);
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
        prompt_prefix_sources: Vec::new(),
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let model_client = Arc::new(ScriptedResultModelClient::new(vec![
        Ok(ModelResponse::final_output(
            r#"{"summary":"MODEL AUTO SUMMARY. Auto compaction used semantic model output."}"#,
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
    assert!(
        requests[0]
            .0
            .max_tokens
            .is_some_and(|limit| limit > 0 && limit <= 4_096)
    );
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
    let audit: serde_json::Value = serde_json::from_str(&metadata).unwrap();
    assert!(
        audit["tokenUsage"].is_null(),
        "unreported summary usage must remain unavailable"
    );
    assert_eq!(audit["usageAudit"]["usageSource"], "unavailable");
    assert_eq!(audit["usageAudit"]["usageConfidence"], "unavailable");
    assert!(audit["usageAudit"]["providerUsage"].is_null());
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
            "COMPACTION_MODEL_REQUEST_FAILED",
        ),
        (
            "auto-model-tool-call",
            Ok(ModelResponse::tool_call("unexpected.tool", json!({}))),
            "COMPACTION_MODEL_REQUEST_FAILED",
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
    let (native_host, presentation) = native_presentation_fixture();
    let agent = ReactAgentBuilder::new(
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
    let protocol = presentation.protocol_events("CONV-1").unwrap();
    assert_native_source_identity(&protocol, "RUN-TOOL-FAILURE");
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
    let (native_host, presentation) = native_presentation_fixture();
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
    .with_framework_host(Some(native_host))
    .build()
    .expect("agent builder should succeed");

    let error = agent
        .run(base_context("RUN-EMPTY-FINAL-OUTPUT"))
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
    assert_native_source_identity(&protocol, "RUN-EMPTY-FINAL-OUTPUT");
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
        prompt_prefix_sources: Vec::new(),
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
        .run(base_context("RUN-PARALLEL-UNAUTHORIZED"))
        .await
        .expect("an unauthorized parallel tool should not abort the run");

    assert_eq!(result.final_output, "recovered after parallel rejection");
    assert_eq!(result.observations.len(), 2);
    assert_eq!(result.observations[0].output, "echo completed");
    assert_eq!(result.observations[1].tool_name, "hidden_tool");
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
        prompt_prefix_sources: Vec::new(),
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
    assert_eq!(result.observations.len(), 4);
    assert!(
        result.observations[3]
            .output
            .contains("tool-call budget exhausted")
    );
    assert!(result.final_output.is_empty());
    let snapshot = result.sdk_execution.as_ref().unwrap();
    assert_eq!(snapshot.model_calls, 1);
    assert_eq!(snapshot.steps, 1);
    assert_eq!(snapshot.tool_calls, 3);
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
    let (native_host, presentation) = native_presentation_fixture();
    let agent = ReactAgentBuilder::new(
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
        .run(base_context("RUN-TOOL-META"))
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
    assert_native_source_identity(&protocol, "RUN-TOOL-META");
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
    assert_eq!(start.data["toolCallId"], invocation_id);
    assert_eq!(start.data["toolCallName"], "echo");
    assert_eq!(args.data["toolCallId"], invocation_id);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(args.data["delta"].as_str().unwrap()).unwrap(),
        serde_json::from_str::<serde_json::Value>(&expected_args).unwrap()
    );
    assert_eq!(result.data["toolCallId"], invocation_id);
    let output: serde_json::Value =
        serde_json::from_str(result.data["content"].as_str().unwrap()).unwrap();
    assert_eq!(output["isError"], false);
    assert_eq!(output["content"][0]["text"], expected_result);
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
    let (native_host, presentation) = native_presentation_fixture();
    let agent = ReactAgentBuilder::new(
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
    assert_native_source_identity(&protocol, "RUN-TOOL-PROGRESS");
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
    let (native_host, presentation) = native_presentation_fixture();
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
    .with_framework_host(Some(native_host))
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
    assert_native_source_identity(&protocol, "RUN-PARALLEL-TOOL-PROGRESS");
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
    let (native_host, presentation) = native_presentation_fixture();
    let agent = ReactAgentBuilder::new(
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
    let mut context = base_context("RUN-TOOL-OVERSIZE");
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
    assert_native_source_identity(&protocol, "RUN-TOOL-OVERSIZE");
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

#[tokio::test]
async fn tool_registry_rejects_missing_or_unauthorized_tools() {
    let registry = ToolRegistry::default();
    let context = base_context("RUN-DENIED");

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
    let mut context = base_context("RUN-EFFECTIVE-HIDDEN");
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

#[tokio::test]
async fn react_agent_recovers_after_missing_and_unauthorized_tools() {
    for (label, tool_name, register_tool, allow_tool, expected_fragment) in [
        ("missing", "missing_tool", false, true, "is not registered"),
        ("unauthorized", "hidden_tool", true, false, "is not allowed"),
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
            prompt_prefix_sources: Vec::new(),
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
        prompt_prefix_sources: Vec::new(),
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
        prompt_prefix_sources: Vec::new(),
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

#[tokio::test]
async fn framework_restores_responses_continuation_after_host_restart() {
    use tron_backend::application::agent::FrameworkHost;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("framework.sqlite3");
    let host = FrameworkHost::persistent(&path, None);
    let first = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output_with_usage_and_response_id(
            "first answer",
            None,
            Some("resp-persisted".into()),
        ),
    )]));
    let agent = ReactAgentBuilder::new(
        first,
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = base_context("DURABLE-1");
    context.model.protocol = AgentModelProtocol::Responses;
    context.model.responses_stateful_enabled = true;
    context.tools.clear();
    agent.run(context.clone()).await.unwrap();
    host.close().await;
    drop(agent);
    drop(host);
    let host = FrameworkHost::persistent(&path, None);
    let second = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output_with_usage_and_response_id(
            "second answer",
            None,
            Some("resp-second".into()),
        ),
    )]));
    let agent = ReactAgentBuilder::new(
        second.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    context.run_code = "DURABLE-2".into();
    context.messages = vec![AgentMessage::user("follow up")];
    agent.run(context.clone()).await.unwrap();
    let requests = second.requests();
    assert_eq!(
        requests[0].1.previous_response_id.as_deref(),
        Some("resp-persisted")
    );
    assert!(
        requests[0]
            .1
            .messages
            .iter()
            .any(|message| message.plain_text_content() == "follow up")
    );
    assert!(
        !requests[0]
            .1
            .messages
            .iter()
            .any(|message| message.plain_text_content().contains("first answer"))
    );
    // Same endpoint/model with another credential must not reuse the previous account ID.
    context.run_code = "DURABLE-ACCOUNT".into();
    context.model.api_key = "another-account-key".into();
    context.messages = vec![AgentMessage::user("another account")];
    let account = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output_with_usage_and_response_id(
            "new account",
            None,
            Some("resp-account".into()),
        ),
    )]));
    let account_agent = ReactAgentBuilder::new(
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
    context.run_code = "DURABLE-3".into();
    context.model.base_url = "https://another-provider.invalid/v1".into();
    context.messages = vec![AgentMessage::user("different provider")];
    let switched = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output("switched"),
    )]));
    let agent = ReactAgentBuilder::new(
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

#[tokio::test]
async fn framework_question_releases_session_and_resumes_durable_interaction() {
    use tron_backend::application::agent::FrameworkHost;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("questions.sqlite3");
    let host = FrameworkHost::persistent(&path, None);
    let registry = Arc::new(ToolRegistry::default());
    let client = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::tool_call(
            "chat_ask_user",
            json!({"intro":"Choose a target","questions":[{"question":"Where?","options":[{"label":"Local"},{"label":"Remote"}]}]}),
        ),
    )]));
    let agent = ReactAgentBuilder::new(
        client.clone(),
        registry.clone(),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = base_context("QUESTION-1");
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
    let request_code = format!("INPUT-FW-{interaction}");
    host.close().await;
    drop(agent);
    drop(host);
    let host = FrameworkHost::persistent(&path, None);
    let answer = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output("using local"),
    )]));
    let agent = ReactAgentBuilder::new(
        answer.clone(),
        registry,
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    context.run_code = "QUESTION-2".into();
    context.messages = vec![AgentMessage::user("Use Local")];
    context.messages[0].parts.push(tron_backend::domain::agent::AgentMessagePart::Context {
        kind: "tron_interaction_response".into(), text: String::new(),
        metadata: Some(json!({"resume":[{"interruptId":request_code,"status":"resolved","payload":{"answers":[{"questionCode":question["questions"][0]["id"],"answerKind":"Option","optionCode":question["questions"][0]["options"][0]["id"]}]}}]})),
    });
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
#[tokio::test]
async fn official_command_returns_after_ten_seconds_and_remains_readable_and_stoppable_across_runs()
{
    use tron_backend::application::agent::FrameworkHost;
    let directory = tempfile::tempdir().unwrap();
    let host = FrameworkHost::temporary(None).unwrap();
    let check_host = host.clone();
    let checked = tokio::spawn(async move {
        let host = check_host;
        let mut context = base_context("COMMAND-1");
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
        let agent = ReactAgentBuilder::new(client, Arc::new(ToolRegistry::default()), events)
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
            let agent = ReactAgentBuilder::new(
                client,
                Arc::new(ToolRegistry::default()),
                tron_backend::shared::event_bus::EventBusHandle::disabled(),
            )
            .with_framework_host(Some(host.clone()))
            .build()
            .unwrap();
            context.run_code = format!("COMMAND-{}", index + 2);
            context.messages = vec![AgentMessage::user(name)];
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

#[tokio::test]
async fn framework_refreshes_host_instructions_without_losing_persisted_conversation() {
    use tron_backend::application::agent::FrameworkHost;
    let host = FrameworkHost::temporary(None).unwrap();
    let mut context = base_context("PREFIX-1");
    context.messages = vec![
        AgentMessage::system("old host instruction"),
        AgentMessage::user("first question"),
    ];
    let first = Arc::new(ScriptedModelClient::new(vec![ModelResponse::final_output(
        "first answer",
    )]));
    let agent = ReactAgentBuilder::new(
        first,
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    agent.run(context.clone()).await.unwrap();
    context.run_code = "PREFIX-2".into();
    context.messages = vec![
        AgentMessage::system("new host instruction"),
        AgentMessage::user("second question"),
    ];
    let second = Arc::new(ScriptedResultModelClient::new(vec![Ok(
        ModelResponse::final_output("second answer"),
    )]));
    let agent = ReactAgentBuilder::new(
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

#[tokio::test]
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
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .unwrap();
    let mut context = base_context("MODEL-RETRY-AFTER-TOOL");
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
#[tokio::test]
async fn framework_stream_overflow_stops_unfinished_provider_without_retry_and_releases_session() {
    use tron_backend::application::agent::FrameworkHost;
    let calls = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let host = FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
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
    let mut context = base_context("STREAM-FLOOD");
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
    context.run_code = "STREAM-RECOVERED".into();
    let result = agent.run_stream(context, Arc::new(|_| {})).await.unwrap();
    assert_eq!(result.final_output, "recovered");
    host.close().await;
}

#[tokio::test]
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
                json!({"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[{"id":"call-1","type":"function","function":{"name":"echo","arguments":"{}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":21192,"completion_tokens":0}})
            } else {
                json!({"choices":[{"message":{"role":"assistant","content":"done"},"finish_reason":"stop"}],"usage":{"prompt_tokens":30,"completion_tokens":1}})
            };
            let response = response.to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).as_bytes()).await.unwrap();
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
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .build()
    .unwrap();
    let mut context = base_context("MAIN-WIRE-BUDGET");
    context.model.base_url = url;
    context.model.context_window_tokens = Some(8000);
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

// Real provider requests: fixtures are independent loopback services and temp SQLite.
struct ProviderTcpServer {
    url: String,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    active: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for ProviderTcpServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl ProviderTcpServer {
    async fn new(responses: Vec<serde_json::Value>, stall: bool) -> Self {
        Self::with_gate(responses, stall, None).await
    }
    async fn with_gate(
        responses: Vec<serde_json::Value>,
        stall: bool,
        gate: Option<Arc<Notify>>,
    ) -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let active = Arc::new(AtomicUsize::new(0));
        let connections = active.clone();
        let task = tokio::spawn(async move {
            let mut index = 0;
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let requests = recorded.clone();
                let active = connections.clone();
                let value = &responses[index.min(responses.len() - 1)];
                let status = value
                    .get("_http_status")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200);
                let response = value.get("_http_body").unwrap_or(value).to_string();
                index += 1;
                let gate = gate.clone();
                active.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    struct Connection(Arc<AtomicUsize>);
                    impl Drop for Connection {
                        fn drop(&mut self) {
                            self.0.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                    let _connection = Connection(active);
                    let mut bytes = vec![];
                    let mut buffer = [0; 4096];
                    let (end, length) = loop {
                        let n = socket.read(&mut buffer).await.unwrap();
                        if n == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..n]);
                        if let Some(end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
                            let length = String::from_utf8_lossy(&bytes[..end])
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
                        if n == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..n]);
                    }
                    requests
                        .lock()
                        .unwrap()
                        .push(serde_json::from_slice(&bytes[end..end + length]).unwrap());
                    if stall {
                        let _ = socket.read(&mut buffer).await;
                        return;
                    }
                    if let Some(gate) = gate {
                        tokio::select! { _ = gate.notified() => {}, _ = socket.read(&mut buffer) => {return;} }
                    }
                    let _ = socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).as_bytes()).await;
                });
            }
        });
        Self {
            url,
            requests,
            active,
            task,
        }
    }
    fn bodies(&self) -> Vec<serde_json::Value> {
        self.requests.lock().unwrap().clone()
    }
    async fn drained(&self) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while self.active.load(Ordering::SeqCst) != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
fn provider_answer(text: &str) -> serde_json::Value {
    json!({"choices":[{"message":{"role":"assistant","content":text},"finish_reason":"stop"}],"usage":{"prompt_tokens":24,"completion_tokens":12,"prompt_tokens_details":{"cached_tokens":16},"completion_tokens_details":{"reasoning_tokens":2}}})
}
fn real_provider_client() -> Arc<tron_backend::adapters::outbound::model::HttpModelClient> {
    Arc::new(
        tron_backend::adapters::outbound::model::HttpModelClient::new(Arc::new(
            tron_backend::adapters::outbound::model::HttpModelProviderClient::new(
                eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
            ),
        )),
    )
}
fn configured_provider_fixture(
    label: &str,
    config: ShortTermMemoryConfig,
    id: i64,
) -> (
    String,
    String,
    ShortTermMemoryService,
    AgentRunContext,
    String,
) {
    let (database, session, old, mut context, prompt) = auto_compaction_fixture(label, id);
    drop(old);
    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database).unwrap(),
        config,
        Arc::new(FixedSnowflakeIdProvider::new(id + 500)),
    );
    context.model.context_window_tokens = Some(40_000);
    context.model.max_tokens = Some(512);
    context.stream_key = Some(session.clone());
    (database, session, service, context, prompt)
}
#[tokio::test]
async fn provider_layout_order_changes_actual_http_and_protects_system_from_data() {
    for reversed in [false, true] {
        let mut config = ShortTermMemoryConfig::default();
        config.compaction.trigger_ratio_bps = 10_000;
        if reversed {
            config.context.providers.order.swap(1, 4);
        }
        let (database, _, memory, mut context, _) = configured_provider_fixture(
            if reversed {
                "provider-order-b"
            } else {
                "provider-order-a"
            },
            config,
            820_000_000_000_000_000 + i64::from(reversed) * 1000,
        );
        context
            .short_term_memory
            .as_mut()
            .unwrap()
            .prompt_prefix
            .push(AgentMessage::context_block(
                "application_execution_context",
                "HOST APPLICATION DATA",
            ));
        add_provider_working(&memory, &mut context, "HOST WORKING DATA");
        let server = ProviderTcpServer::new(vec![provider_answer("done")], false).await;
        context.model.base_url = server.url.clone();
        let client = real_provider_client();
        let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
        let agent = ReactAgentBuilder::new(
            client.clone(),
            Arc::new(ToolRegistry::default()),
            tron_backend::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(Some(memory.clone()))
        .build()
        .unwrap();
        assert_eq!(agent.run(context).await.unwrap().final_output, "done");
        let bodies = server.bodies();
        assert_eq!(bodies.len(), 1, "no trigger means zero auxiliary requests");
        let messages = bodies[0]["messages"].as_array().unwrap();
        let system = messages
            .iter()
            .position(|m| m["role"] == "system" && m.to_string().contains("system seed"))
            .unwrap();
        let application = messages
            .iter()
            .position(|m| m.to_string().contains("HOST APPLICATION DATA"))
            .unwrap();
        assert_eq!(messages[application]["role"], "user");
        let working = messages
            .iter()
            .position(|message| message.to_string().contains("HOST WORKING DATA"))
            .unwrap();
        assert!(
            system < application && system < working,
            "trusted prefix remains stable"
        );
        assert_eq!(
            application < working,
            !reversed,
            "nonempty scoped provider sources must follow configuration order"
        );
        host.close().await;
        client.close().await;
        server.drained().await;
        drop(agent);
        drop(memory);
        let _ = std::fs::remove_file(database);
    }
}
#[tokio::test]
async fn provider_history_single_trigger_below_global_commits_true_checkpoint_usage() {
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.trigger_ratio_bps = 9_000;
    config.context.providers.history.trigger_ratio_bps = 2_000;
    config.context.providers.history.target_ratio_bps = 1_000;
    let (database, session, memory, mut context, prompt) =
        configured_provider_fixture("provider-history-only", config, 821_000_000_000_000_000);
    let server = ProviderTcpServer::new(
        vec![
            provider_answer(r#"{"summary":"HISTORY SINGLE SUMMARY"}"#),
            provider_answer("done"),
        ],
        false,
    )
    .await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .build()
    .unwrap();
    let source = memory.prepare_model_context(&context).unwrap();
    assert!(
        !source.compaction_trigger_reached,
        "single block pressure must be below the global trigger"
    );
    let result = tokio::time::timeout(Duration::from_secs(5), agent.run(context))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.final_output, "done");
    let bodies = server.bodies();
    assert_eq!(bodies.len(), 2);
    assert!(bodies[1].to_string().contains("HISTORY SINGLE SUMMARY"));
    assert!(bodies[1].to_string().contains(&prompt));
    let checkpoint = memory.latest_compaction(&session).unwrap().unwrap();
    let connection = rusqlite::Connection::open(&database).unwrap();
    let metadata: String = connection
        .query_row(
            "SELECT metadata_json FROM agent_transcript_messages WHERE message_code=?1",
            [checkpoint.checkpoint_message_code],
            |row| row.get(0),
        )
        .unwrap();
    let audit: serde_json::Value = serde_json::from_str(&metadata).unwrap();
    assert_eq!(audit["tokenUsage"]["cachedInputTokens"], 16);
    assert_eq!(audit["tokenUsage"]["reasoningTokens"], 2);
    assert_eq!(audit["coverage"], "head_only_tail_replayed");
    assert_eq!(
        audit["retainedHistoryRatioBps"], 1000,
        "checkpoint must report the actual provider target"
    );
    let final_event:String=connection.query_row("SELECT payload_json FROM agent_transcript_events WHERE event_type='agent.context.assembled' AND json_extract(payload_json,'$.source')='framework.default-context-assembler' ORDER BY seq DESC LIMIT 1",[],|row|row.get(0)).unwrap();
    let final_event: serde_json::Value = serde_json::from_str(&final_event).unwrap();
    assert_eq!(
        final_event["budgetBreakdown"]["retainedHistoryRatioBps"],
        1000
    );

    host.close().await;
    client.close().await;
    server.drained().await;
    drop(connection);
    drop(agent);
    drop(memory);
    let _ = std::fs::remove_file(database);
}

fn semantic_working(config: &mut ShortTermMemoryConfig) {
    use tron_backend::application::short_term_memory::ContextReductionStrategy;
    config.context.providers.working.strategy = ContextReductionStrategy::Semantic;
    config.context.providers.working.trigger_ratio_bps = 5_000;
    config.context.providers.working.target_ratio_bps = 2_000;
    config.context.providers.history.trigger_ratio_bps = 2_000;
    config.context.providers.history.target_ratio_bps = 1_000;
}
fn add_provider_working(
    memory: &ShortTermMemoryService,
    context: &mut AgentRunContext,
    content: &str,
) {
    context.short_term_memory.as_mut().unwrap().todo_enabled = true;
    let characters = content.chars().collect::<Vec<_>>();
    let items = characters
        .chunks(400)
        .enumerate()
        .map(
            |(position, chunk)| tron_backend::application::short_term_memory::TodoUpdateItem {
                todo_code: None,
                position: position as i64,
                content: chunk.iter().collect(),
                status: if position == 0 {
                    "in_progress"
                } else {
                    "pending"
                }
                .into(),
                priority: "high".into(),
            },
        )
        .collect();
    memory.replace_todos(context, items).unwrap();
}
#[tokio::test]
async fn provider_global_coordinates_strategies_reuses_data_after_history_refresh_and_replays_usage()
 {
    use tron_backend::application::agent::AgentAguiReplayStore;
    let mut config = ShortTermMemoryConfig::default();
    semantic_working(&mut config);
    config.compaction.trigger_ratio_bps = 2_000;
    let (database, session, memory, mut context, _) =
        configured_provider_fixture("provider-global", config, 822_000_000_000_000_000);
    add_provider_working(
        &memory,
        &mut context,
        &format!("RETAIN AUTHORIZED TODO {}", "W".repeat(1_500)),
    );
    let server = ProviderTcpServer::new(
        vec![
            provider_answer(r#"{"summary":"WORKING SUMMARY"}"#),
            provider_answer(r#"{"summary":"HISTORY SUMMARY"}"#),
            provider_answer("done"),
        ],
        false,
    )
    .await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let sink = Arc::new(RecordingEventSink::with_route_class(
        "provider-usage",
        BusRouteClass::Agui,
    ));
    let replay = Arc::new(
        tron_backend::adapters::outbound::persistence::SqliteAgentEventReplayStore::connect(
            &database,
        )
        .unwrap(),
    );
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(829_000_000_000_000_000)),
    )
    .register_sink(sink.clone())
    .register_sink(replay.clone())
    .build();
    let agent = ReactAgentBuilder::new(client.clone(), Arc::new(ToolRegistry::default()), bus)
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(Some(memory.clone()))
        .build()
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), agent.run(context))
            .await
            .unwrap()
            .unwrap()
            .final_output,
        "done"
    );
    let bodies = server.bodies();
    assert_eq!(
        bodies.len(),
        3,
        "both paid reducers share the old 3-call ceiling and unchanged working summary must be cached on refresh"
    );
    assert!(bodies[0].to_string().contains("RETAIN AUTHORIZED TODO"));
    assert!(bodies[2].to_string().contains("WORKING SUMMARY"));
    assert!(bodies[2].to_string().contains("HISTORY SUMMARY"));
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if replay
                .replay_agui_events(&session)
                .unwrap()
                .iter()
                .any(|e| e.event_name == "agent.context.block_compressed")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let events = replay.replay_agui_events(&session).unwrap();
    let audits = events
        .iter()
        .filter(|event| event.event_name == "agent.context.block_compressed")
        .collect::<Vec<_>>();
    assert_eq!(audits.len(), 1, "cached bytes must not repeat prior audit");
    let protocol = replay.replay_protocol_events(&session).unwrap();
    let custom = protocol
        .iter()
        .find(|event| event.data["name"] == "tron.short_term_memory.context.block_compressed")
        .unwrap();
    assert_eq!(
        custom.data["value"]["blockId"],
        "current_session_working_state"
    );
    assert_eq!(
        custom.data["value"]["usageAudit"]["providerUsage"]["cachedInputTokens"],
        16
    );
    let audit: serde_json::Value =
        serde_json::from_str(audits[0].metadata.get("usage_audit_json").unwrap()).unwrap();
    assert_eq!(audit["providerUsage"]["cachedInputTokens"], 16);
    assert_eq!(audit["providerUsage"]["reasoningTokens"], 2);
    assert_eq!(audit["usageConfidence"], "actual");
    let connection = rusqlite::Connection::open(&database).unwrap();
    let metadata:String=connection.query_row("SELECT metadata_json FROM agent_transcript_events WHERE event_type='agent.context.block_compressed'",[],|row|row.get(0)).unwrap();
    assert!(metadata.contains("usage_audit_json"));
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    drop(replay);
    drop(connection);
    let _ = std::fs::remove_file(database);
}
#[tokio::test]
async fn provider_stop_during_semantic_auxiliary_closes_transport_without_audit_or_checkpoint() {
    let mut config = ShortTermMemoryConfig::default();
    semantic_working(&mut config);
    config.compaction.trigger_ratio_bps = 10_000;
    let (database, session, memory, mut context, _) =
        configured_provider_fixture("provider-stop", config, 823_000_000_000_000_000);
    add_provider_working(
        &memory,
        &mut context,
        &format!("UNFINISHED TODO {}", "W".repeat(1_500)),
    );
    let server = ProviderTcpServer::new(vec![provider_answer("unreachable")], true).await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let registry = ActiveRunRegistry::new();
    let runtime_registry = tron_backend::application::agent::AgentRuntimeRegistry::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
        reqwest::Client::new(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .with_active_runs(registry.clone());
    let executor = tron_backend::application::agent::AgentRunExecutor::new(runtime_registry);
    let run_code = context.run_code.clone();
    let task = tokio::spawn(async move {
        executor
            .execute_with_selection(
                tron_backend::application::agent::RuntimeAdapterSelection::React,
                context,
                None,
                None,
                None,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        while server.bodies().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    registry.request_stop(&session, &run_code).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    server.drained().await;
    assert_eq!(server.bodies().len(), 1);
    assert!(memory.latest_compaction(&session).unwrap().is_none());
    let connection = rusqlite::Connection::open(&database).unwrap();
    let audits:i64=connection.query_row("SELECT COUNT(*) FROM agent_transcript_events WHERE event_type='agent.context.block_compressed'",[],|row|row.get(0)).unwrap();
    assert_eq!(audits, 0);
    host.close().await;
    client.close().await;
    drop(memory);
    drop(connection);
    let _ = std::fs::remove_file(database);
}

fn provider_responses_answer(id: &str) -> serde_json::Value {
    json!({"id":id,"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"done"}]}],"usage":{"input_tokens":24,"output_tokens":12,"input_tokens_details":{"cached_tokens":16},"output_tokens_details":{"reasoning_tokens":2}}})
}
#[tokio::test]
async fn provider_final_layout_resets_responses_continuation_but_plain_append_reuses_it() {
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.auto_enabled = false;
    let (database, session, memory, mut context, _) = configured_provider_fixture(
        "provider-continuation",
        config.clone(),
        824_000_000_000_000_000,
    );
    let server = ProviderTcpServer::new(
        vec![
            provider_responses_answer("response-a"),
            provider_responses_answer("response-b"),
            provider_responses_answer("response-c"),
        ],
        false,
    )
    .await;
    context.model.base_url = server.url.clone();
    context.model.protocol = AgentModelProtocol::Responses;
    context.model.responses_stateful_enabled = true;
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    for index in 0..3 {
        if index == 1 {
            config.context.providers.order.swap(0, 1);
        }
        let memory = if index == 0 {
            memory.clone()
        } else {
            ShortTermMemoryService::with_id_provider(
                SqliteShortTermMemoryRepository::connect(&database).unwrap(),
                config.clone(),
                Arc::new(FixedSnowflakeIdProvider::new(
                    824_100_000_000_000_000 + index * 1000,
                )),
            )
        };
        let mut current = context.clone();
        current.run_code = format!("PROVIDER-CONTINUATION-{index}");
        if index > 0 {
            let prompt = format!("append {index}");
            current.messages = vec![AgentMessage::user(&prompt)];
            current.short_term_memory.as_mut().unwrap().input_code =
                memory.record_user_input(&current, &prompt).unwrap();
        }
        let agent = ReactAgentBuilder::new(
            client.clone(),
            Arc::new(ToolRegistry::default()),
            tron_backend::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(Some(memory))
        .build()
        .unwrap();
        assert_eq!(agent.run(current).await.unwrap().final_output, "done");
    }
    let bodies = server.bodies();
    assert_eq!(bodies.len(), 3);
    assert!(bodies[0].get("previous_response_id").is_none());
    assert!(
        bodies[1].get("previous_response_id").is_none(),
        "changed final layout must replay full inputs"
    );
    assert_eq!(
        bodies[2]["previous_response_id"], "response-b",
        "an ordinary appended history turn should keep the same stable nonhistory context"
    );
    assert!(bodies[1].to_string().contains("system seed"));
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(memory);
    let _ = std::fs::remove_file(database);
    let _ = session;
}
#[tokio::test]
async fn provider_semantic_actual_summary_input_capacity_rejects_before_http() {
    let mut config = ShortTermMemoryConfig::default();
    semantic_working(&mut config);
    // Reserve a usable reduced data target to reach the separate summary model input-capacity check.
    config
        .context
        .blocks
        .current_session_working_state
        .max_ratio_bps = None;
    config.compaction.trigger_ratio_bps = 10_000;
    let (database, _, memory, mut context, _) =
        configured_provider_fixture("provider-aux-capacity", config, 825_000_000_000_000_000);
    add_provider_working(
        &memory,
        &mut context,
        &format!("TOO LARGE SUMMARY SOURCE {}", "W".repeat(3_800)),
    );
    let server = ProviderTcpServer::new(vec![provider_answer("unreachable")], false).await;
    context.model.base_url = server.url.clone();
    context.model.max_tokens = Some(0);
    context.model.context_window_tokens = Some(8_000);
    // Main can fit this full source. Existing summary policy reserves 4096,
    // leaving only 3904 input tokens for its actual summary request.
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute("DELETE FROM agent_transcript_parts WHERE message_code IN (SELECT message_code FROM agent_transcript_messages WHERE run_code LIKE 'RUN-AUTO-OLD-%')",[]).unwrap();
    connection
        .execute(
            "DELETE FROM agent_transcript_messages WHERE run_code LIKE 'RUN-AUTO-OLD-%'",
            [],
        )
        .unwrap();
    let raw = memory.prepare_model_context(&context).unwrap();
    assert!(
        raw.estimated_tokens < 8_000,
        "main source must fit: {}",
        raw.estimated_tokens
    );
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .build()
    .unwrap();
    let error = agent.run(context).await.unwrap_err();
    assert!(
        error.to_string().contains("summary-model input limit"),
        "must fail specifically at auxiliary capacity: {error}"
    );
    assert!(
        server.bodies().is_empty(),
        "output reserve must not be treated as additional auxiliary input capacity"
    );
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    let _ = std::fs::remove_file(database);
}

fn provider_overflow() -> serde_json::Value {
    json!({"_http_status":400,"_http_body":{"error":{"code":"context_length_exceeded","message":"maximum context length exceeded"}}})
}
#[tokio::test]
async fn provider_overflow_respects_keep_and_zero_auxiliary_allowance() {
    use tron_backend::application::short_term_memory::ContextReductionStrategy;
    for keep in [false, true] {
        let mut config = ShortTermMemoryConfig::default();
        config.compaction.trigger_ratio_bps = 10_000;
        if keep {
            config.context.providers.history.strategy = ContextReductionStrategy::Keep;
        } else {
            config.context.providers.max_auxiliary_calls_per_step = 0;
        }
        let (database, session, memory, mut context, _) = configured_provider_fixture(
            if keep {
                "provider-overflow-keep"
            } else {
                "provider-overflow-zero"
            },
            config,
            826_000_000_000_000_000 + i64::from(keep) * 1000,
        );
        let server = ProviderTcpServer::new(vec![provider_overflow()], false).await;
        context.model.base_url = server.url.clone();
        context.model.max_retries = Some(1);
        let client = real_provider_client();
        let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
        let agent = ReactAgentBuilder::new(
            client.clone(),
            Arc::new(ToolRegistry::default()),
            tron_backend::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(Some(memory.clone()))
        .build()
        .unwrap();
        assert!(agent.run(context).await.is_err());
        assert_eq!(
            server.bodies().len(),
            1,
            "configured keep/zero auxiliary ceiling must not dispatch implicit history recovery"
        );
        assert!(memory.latest_compaction(&session).unwrap().is_none());
        host.close().await;
        client.close().await;
        server.drained().await;
        drop(agent);
        drop(memory);
        let _ = std::fs::remove_file(database);
    }
}
#[tokio::test]
async fn provider_overflow_cached_same_final_projection_stops_without_second_main_call() {
    use tron_backend::application::short_term_memory::ContextReductionStrategy;
    let mut config = ShortTermMemoryConfig::default();
    semantic_working(&mut config);
    config.context.providers.history.strategy = ContextReductionStrategy::Keep;
    config.compaction.trigger_ratio_bps = 2_000;
    let (database, _, memory, mut context, _) =
        configured_provider_fixture("provider-overflow-no-gain", config, 827_000_000_000_000_000);
    add_provider_working(
        &memory,
        &mut context,
        &format!("KEEP SOURCE STABLE {}", "W".repeat(1500)),
    );
    let server = ProviderTcpServer::new(
        vec![
            provider_answer(r#"{"summary":"STABLE WORKING SUMMARY"}"#),
            provider_overflow(),
        ],
        false,
    )
    .await;
    context.model.base_url = server.url.clone();
    context.model.max_retries = Some(1);
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .build()
    .unwrap();
    assert!(agent.run(context).await.is_err());
    assert_eq!(
        server.bodies().len(),
        2,
        "cached source-relative reduction is not progress relative to the actual failed model input"
    );
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    let _ = std::fs::remove_file(database);
}
#[tokio::test]
async fn provider_live_guard_rejects_todo_change_after_paid_aux_and_preserves_actual_audit() {
    use tron_backend::application::agent::AgentAguiReplayStore;
    let mut config = ShortTermMemoryConfig::default();
    semantic_working(&mut config);
    config.compaction.trigger_ratio_bps = 10_000;
    let (database, session, memory, mut context, _) =
        configured_provider_fixture("provider-live-update", config, 828_000_000_000_000_000);
    add_provider_working(
        &memory,
        &mut context,
        &format!("ORIGINAL TODO {}", "W".repeat(1500)),
    );
    let gate = Arc::new(Notify::new());
    let server = ProviderTcpServer::with_gate(
        vec![provider_answer(r#"{"summary":"STALE OLD SUMMARY"}"#)],
        false,
        Some(gate.clone()),
    )
    .await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let replay = Arc::new(
        tron_backend::adapters::outbound::persistence::SqliteAgentEventReplayStore::connect(
            &database,
        )
        .unwrap(),
    );
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(828_500_000_000_000_000)),
    )
    .register_sink(replay.clone())
    .build();
    let agent = Arc::new(
        ReactAgentBuilder::new(client.clone(), Arc::new(ToolRegistry::default()), bus)
            .with_framework_host(Some(host.clone()))
            .with_short_term_memory(Some(memory.clone()))
            .build()
            .unwrap(),
    );
    let task = tokio::spawn({
        let agent = agent.clone();
        let context = context.clone();
        async move { agent.run(context).await }
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        while server.bodies().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    add_provider_working(&memory, &mut context, "NEW AUTHORIZED TODO");
    gate.notify_one();
    assert!(
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_eq!(
        server.bodies().len(),
        1,
        "stale data must not reach primary HTTP"
    );
    assert!(memory.latest_compaction(&session).unwrap().is_none());
    tokio::time::timeout(Duration::from_secs(3), async {
        while replay
            .replay_agui_events(&session)
            .unwrap()
            .iter()
            .all(|e| e.event_name != "agent.context.block_compressed")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let events = replay.replay_agui_events(&session).unwrap();
    let audit = events
        .iter()
        .find(|e| e.event_name == "agent.context.block_compressed")
        .unwrap();
    let usage: serde_json::Value =
        serde_json::from_str(audit.metadata.get("usage_audit_json").unwrap()).unwrap();
    assert_eq!(usage["providerUsage"]["inputTokens"], 24);
    assert_eq!(
        memory.list_todos(&session).unwrap()[0].content,
        "NEW AUTHORIZED TODO"
    );
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    drop(replay);
    let _ = std::fs::remove_file(database);
}

#[tokio::test]
async fn provider_after_protocol_percentages_coordinate_recoverable_tools_pressure() {
    use tron_backend::application::short_term_memory::{
        CompactionContextPolicy, ModelCompactionOutput,
    };
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.trigger_ratio_bps = 5_000;
    config.context.blocks.seed_prompt_prefix.max_tokens = None;
    config
        .context
        .blocks
        .current_session_checkpoint
        .max_ratio_bps = Some(3_000);
    config.context.blocks.current_session_checkpoint.max_tokens = None;
    config.context.blocks.session_family_context.max_ratio_bps = Some(0);
    config
        .context
        .blocks
        .current_session_working_state
        .max_ratio_bps = Some(2_000);
    config
        .context
        .blocks
        .current_session_working_state
        .max_tokens = None;
    config.context.providers.checkpoint.target_ratio_bps = 5_000;
    config.context.providers.working.strategy =
        tron_backend::application::short_term_memory::ContextReductionStrategy::Bound;
    config.context.providers.working.target_ratio_bps = 5_000;
    let (database, session, memory, mut context, _) = configured_provider_fixture(
        "provider-protocol-pressure",
        config,
        830_000_000_000_000_000,
    );
    let prepared = memory
        .prepare_automatic_model_compaction_input(
            &session,
            "fixture-existing-checkpoint",
            CompactionContextPolicy {
                todo_enabled: false,
            },
            context.model.context_window_tokens,
            context.model.max_tokens,
            Some(&context.run_code),
            None,
            Some(6_000),
        )
        .unwrap();
    memory
        .commit_model_compaction(
            prepared,
            ModelCompactionOutput {
                summary: format!("EXISTING CHECKPOINT {}", "C".repeat(12_000)),
                token_usage: None,
                provider_metadata: None,
            },
            &context.model,
        )
        .unwrap();
    add_provider_working(
        &memory,
        &mut context,
        &format!("AUTHORIZED WORKING {}", "W".repeat(8_000)),
    );
    context.short_term_memory.as_mut().unwrap().prompt_prefix = vec![AgentMessage::system(
        format!("PROTECTED SYSTEM {}", "S".repeat(13_000)),
    )];
    context.tools = vec![EffectiveTool {
        tool_name: "pressure_tool".into(),
        domain_code: None,
        description: "T".repeat(44_000),
        input_schema: json!({"type":"object"}),
        skill_name: None,
        required_variable_keys: vec![],
    }];
    let registry = ToolRegistry::default();
    registry
        .register(Arc::new(InProcessTool::new("pressure_tool", |_| {
            Ok(ToolObservation {
                tool_name: "pressure_tool".into(),
                output: "unused".into(),
            })
        })))
        .unwrap();
    let server = ProviderTcpServer::new(
        vec![
            provider_answer(r#"{"summary":"COORDINATED HISTORY"}"#),
            provider_answer("done"),
        ],
        false,
    )
    .await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(registry),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .build()
    .unwrap();
    assert_eq!(agent.run(context).await.unwrap().final_output, "done");
    let bodies = server.bodies();
    assert!(bodies.len() <= 3);
    let final_body = bodies.last().unwrap();
    assert!(final_body.to_string().contains("PROTECTED SYSTEM"));
    assert!(final_body.to_string().contains(&"S".repeat(13_000)));
    assert_eq!(final_body["tools"].as_array().unwrap().len(), 1);
    let connection = rusqlite::Connection::open(&database).unwrap();
    let report:String=connection.query_row("SELECT payload_json FROM agent_transcript_events WHERE event_type='agent.context.assembled' AND json_extract(payload_json,'$.source')='framework.default-context-assembler' ORDER BY seq DESC LIMIT 1",[],|row|row.get(0)).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report).unwrap();
    let assembly = &report["contextReport"];
    assert!(
        assembly["protocol_tokens"].as_u64().unwrap()
            > assembly["input_capacity"].as_u64().unwrap() / 10
    );
    assert!(
        assembly["estimated_input_tokens"].as_u64().unwrap()
            <= assembly["input_capacity"].as_u64().unwrap()
    );
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    drop(connection);
    let _ = std::fs::remove_file(database);
}

#[tokio::test]
async fn provider_auxiliary_transport_timeout_releases_connection_and_audits_unavailable_usage() {
    use tron_backend::application::agent::AgentAguiReplayStore;
    let mut config = ShortTermMemoryConfig::default();
    semantic_working(&mut config);
    config.compaction.trigger_ratio_bps = 10_000;
    let (database, session, memory, mut context, _) =
        configured_provider_fixture("provider-timeout", config, 831_000_000_000_000_000);
    add_provider_working(
        &memory,
        &mut context,
        &format!("TIMEOUT TODO {}", "W".repeat(1500)),
    );
    let server = ProviderTcpServer::new(vec![provider_answer("unreachable")], true).await;
    context.model.base_url = server.url.clone();
    let transport = eliteforge_agent_official_plugin_model_http::ModelHttpClient::from_builder(
        reqwest::Client::builder().timeout(Duration::from_millis(150)),
    )
    .unwrap();
    let client = Arc::new(
        tron_backend::adapters::outbound::model::HttpModelClient::new(Arc::new(
            tron_backend::adapters::outbound::model::HttpModelProviderClient::new(transport),
        )),
    );
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let replay = Arc::new(
        tron_backend::adapters::outbound::persistence::SqliteAgentEventReplayStore::connect(
            &database,
        )
        .unwrap(),
    );
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(831_500_000_000_000_000)),
    )
    .register_sink(replay.clone())
    .build();
    let agent = ReactAgentBuilder::new(client.clone(), Arc::new(ToolRegistry::default()), bus)
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(Some(memory.clone()))
        .build()
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(3), agent.run(context))
            .await
            .unwrap()
            .is_err()
    );
    server.drained().await;
    assert_eq!(server.bodies().len(), 1);
    assert!(memory.latest_compaction(&session).unwrap().is_none());
    tokio::time::timeout(Duration::from_secs(3), async {
        while replay
            .replay_agui_events(&session)
            .unwrap()
            .iter()
            .all(|e| e.event_name != "agent.context.block_compressed")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let events = replay.replay_agui_events(&session).unwrap();
    let event = events
        .iter()
        .find(|e| e.event_name == "agent.context.block_compressed")
        .unwrap();
    assert_eq!(event.outcome, "FAILURE");
    let audit: serde_json::Value =
        serde_json::from_str(event.metadata.get("usage_audit_json").unwrap()).unwrap();
    assert!(audit["providerUsage"].is_null());
    assert_eq!(audit["usageConfidence"], "unavailable");
    assert!(
        audit["requestEstimate"]["estimatedInputTokens"]
            .as_u64()
            .unwrap()
            > 0
    );
    host.close().await;
    client.close().await;
    drop(agent);
    drop(memory);
    drop(replay);
    let _ = std::fs::remove_file(database);
}

#[tokio::test]
async fn provider_default_working_bounds_complete_legal_todo_source_inside_sdk() {
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.trigger_ratio_bps = 10_000;
    let (database, session, memory, mut context, _) =
        configured_provider_fixture("provider-default-todos", config, 832_000_000_000_000_000);
    context.short_term_memory.as_mut().unwrap().todo_enabled = true;
    let items = (0..40)
        .map(
            |position| tron_backend::application::short_term_memory::TodoUpdateItem {
                todo_code: None,
                position,
                content: format!("LEGAL TODO {position} {}", "W".repeat(400)),
                status: "pending".into(),
                priority: "high".into(),
            },
        )
        .collect();
    memory.replace_todos(&context, items).unwrap();
    let raw = memory.prepare_model_context(&context).unwrap();
    let source = serde_json::to_string(&raw.sources.as_ref().unwrap().working).unwrap();
    assert!(source.contains("LEGAL TODO 39"));
    assert!(source.len() > 4_000, "no legacy 4k pre-truncation");
    let server = ProviderTcpServer::new(vec![provider_answer("done")], false).await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .build()
    .unwrap();
    assert_eq!(agent.run(context).await.unwrap().final_output, "done");
    let bodies = server.bodies();
    assert_eq!(bodies.len(), 1, "default bound uses no auxiliary HTTP");
    assert!(bodies[0].to_string().contains("LEGAL TODO 0"));
    assert!(!bodies[0].to_string().contains("LEGAL TODO 39"));
    let connection = rusqlite::Connection::open(&database).unwrap();
    let report:String=connection.query_row("SELECT payload_json FROM agent_transcript_events WHERE event_type='agent.context.assembled' AND json_extract(payload_json,'$.source')='framework.default-context-assembler' ORDER BY seq DESC LIMIT 1",[],|row|row.get(0)).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report).unwrap();
    let block = report["contextReport"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == "current_session_working_state")
        .unwrap();
    assert!(block["final_tokens"].as_u64().unwrap() <= 2_000);
    assert!(block["original_tokens"].as_u64().unwrap() > 4_000);
    assert_eq!(memory.list_todos(&session).unwrap().len(), 40);
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    drop(connection);
    let _ = std::fs::remove_file(database);
}

#[tokio::test]
async fn provider_working_bound_preserves_goal_before_todos() {
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.trigger_ratio_bps = 10_000;
    config
        .context
        .blocks
        .current_session_working_state
        .max_ratio_bps = None;
    config
        .context
        .blocks
        .current_session_working_state
        .max_tokens = Some(800);
    let (database, session, memory, mut context, _) =
        configured_provider_fixture("provider-goal-priority", config, 833_000_000_000_000_000);
    context.short_term_memory.as_mut().unwrap().todo_enabled = true;
    memory
        .set_goal(
            tron_backend::application::short_term_memory::SetGoalCommand {
                session_code: session.clone(),
                objective: "Preserve the active session goal under pressure".into(),
                max_runs: Some(5),
            },
        )
        .unwrap();
    memory
        .replace_todos(
            &context,
            (0..10)
                .map(
                    |position| tron_backend::application::short_term_memory::TodoUpdateItem {
                        todo_code: None,
                        position,
                        content: format!("LOWER PRIORITY TODO {position} {}", "T".repeat(400)),
                        status: "pending".into(),
                        priority: "high".into(),
                    },
                )
                .collect(),
        )
        .unwrap();
    let raw = memory.prepare_model_context(&context).unwrap();
    assert!(
        serde_json::to_string(&raw.sources.unwrap().working)
            .unwrap()
            .contains("LOWER PRIORITY TODO 9")
    );
    let server = ProviderTcpServer::new(vec![provider_answer("done")], false).await;
    context.model.base_url = server.url.clone();
    let client = real_provider_client();
    let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
    let agent = ReactAgentBuilder::new(
        client.clone(),
        Arc::new(ToolRegistry::default()),
        tron_backend::shared::event_bus::EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(memory.clone()))
    .build()
    .unwrap();
    assert_eq!(agent.run(context).await.unwrap().final_output, "done");
    let bodies = server.bodies();
    assert_eq!(bodies.len(), 1);
    let rendered = bodies[0].to_string();
    assert!(
        rendered.contains("Preserve the active session goal under pressure"),
        "{rendered}"
    );
    assert!(!rendered.contains("LOWER PRIORITY TODO 9"));
    let connection = rusqlite::Connection::open(&database).unwrap();
    let report:String=connection.query_row("SELECT payload_json FROM agent_transcript_events WHERE event_type='agent.context.assembled' AND json_extract(payload_json,'$.source')='framework.default-context-assembler' ORDER BY seq DESC LIMIT 1",[],|row|row.get(0)).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report).unwrap();
    let working = report["contextReport"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|block| block["id"] == "current_session_working_state")
        .unwrap();
    assert!(working["final_tokens"].as_u64().unwrap() <= 800);
    assert!(working["original_tokens"].as_u64().unwrap() > 800);
    host.close().await;
    client.close().await;
    server.drained().await;
    drop(agent);
    drop(memory);
    drop(connection);
    let _ = std::fs::remove_file(database);
}

#[tokio::test]
async fn provider_full_retention_keep_is_noop_and_explicit_invalid_checkpoint_is_rejected() {
    for keep in [true, false] {
        let mut config = ShortTermMemoryConfig::default();
        config.compaction.retained_history_ratio_bps = 10_000;
        config.compaction.trigger_ratio_bps = 10_000;
        config.context.providers.history.target_ratio_bps = 10_000;
        config.context.providers.history.strategy = if keep {
            tron_backend::application::short_term_memory::ContextReductionStrategy::Keep
        } else {
            tron_backend::application::short_term_memory::ContextReductionStrategy::Checkpoint
        };
        let (database, session, memory, mut context, _) = configured_provider_fixture(
            if keep {
                "provider-retain-all"
            } else {
                "provider-invalid-retain"
            },
            config,
            834_000_000_000_000_000 + if keep { 0 } else { 10000 },
        );
        let server = ProviderTcpServer::new(vec![provider_answer("done")], false).await;
        context.model.base_url = server.url.clone();
        let client = real_provider_client();
        let host = tron_backend::application::agent::FrameworkHost::temporary(None).unwrap();
        let agent = ReactAgentBuilder::new(
            client.clone(),
            Arc::new(ToolRegistry::default()),
            tron_backend::shared::event_bus::EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .with_short_term_memory(Some(memory.clone()))
        .build()
        .unwrap();
        let result = agent.run(context).await;
        if keep {
            assert_eq!(result.unwrap().final_output, "done");
            assert_eq!(server.bodies().len(), 1);
            assert!(memory.latest_compaction(&session).unwrap().is_none());
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("compression target must be below trigger")
            );
            assert!(server.bodies().is_empty());
        }
        host.close().await;
        client.close().await;
        server.drained().await;
        drop(agent);
        drop(memory);
        let _ = std::fs::remove_file(database);
    }
}

#[path = "support/native_model_runtime.rs"]
mod native_model_runtime;
