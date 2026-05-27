use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use serde::{Deserialize, Serialize};

use crate::{
    application::{
        app::AppService, skill::SkillRegistryService, variable::VariableService,
        workspace::WorkspaceService,
    },
    domain::{
        agent::{
            AgentAguiEvent, AgentAguiEventSnapshot, AgentAguiStreamState, AgentError,
            AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentRunContext,
            AgentRunHistoryError, AgentRunHistoryQuery, AgentRunHistoryRecord, AgentRunLimits,
            AgentRunStatus, EffectiveSkill, EffectiveTool, EffectiveVariable,
            OpenAiCompatibleModelClient, PagedAgentRunHistoryRecords, ReactAgentBuilder,
            ToolRegistry,
        },
        workspace::WorkspaceStatus,
    },
    shared::{
        event_bus::{AguiEventSink, BusEvent, BusOutcome, BusRouteClass, EventBusHandle},
        time::{current_timestamp, timestamp_from_millis},
    },
};

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationCommand {
    pub prompt: Option<String>,
    pub workspace_code: Option<String>,
    pub conversation_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub model: Option<AgentModelConfig>,
    pub limits: Option<AgentRunLimits>,
    #[serde(default)]
    pub tools: Vec<EffectiveTool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocationResult {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub status: &'static str,
    pub final_output: String,
    pub step_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentInvocationError {
    PromptRequired,
    WorkspaceRequired,
    WorkspaceInvalid,
    ModelRequired,
    HistoryStorageFailed,
    RuntimeFailed(String),
    ModelFailed(String),
    ToolFailed(String),
    MaxStepsExceeded,
}

pub trait AgentRunHistoryRepository: Send {
    fn list_runs(&mut self) -> Result<Vec<AgentRunHistoryRecord>, AgentRunHistoryError>;
    fn get_run(&mut self, run_code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError>;
    fn next_id(&mut self) -> Result<i64, AgentRunHistoryError>;
    fn create_run(&mut self, record: &AgentRunHistoryRecord) -> Result<(), AgentRunHistoryError>;
}

#[derive(Clone)]
pub struct AgentRunHistoryService {
    repository: Arc<Mutex<Box<dyn AgentRunHistoryRepository>>>,
}

impl AgentRunHistoryService {
    pub fn new(repository: impl AgentRunHistoryRepository + 'static) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
        }
    }

    pub fn create_run(
        &self,
        mut record: AgentRunHistoryRecord,
    ) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        let mut repository = self
            .repository
            .lock()
            .expect("agent run history repository mutex poisoned");
        record.id = repository.next_id()?;
        repository.create_run(&record)?;
        Ok(record)
    }

    pub fn list_runs(
        &self,
        query: AgentRunHistoryQuery,
    ) -> Result<PagedAgentRunHistoryRecords<AgentRunHistoryRecord>, AgentRunHistoryError> {
        let current = query.current.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(20).clamp(1, 100);
        let mut records = self
            .repository
            .lock()
            .expect("agent run history repository mutex poisoned")
            .list_runs()?;
        if let Some(workspace_code) = normalized_optional(query.workspace_code) {
            records.retain(|record| record.workspace_code == workspace_code);
        }
        if let Some(conversation_code) = normalized_optional(query.conversation_code) {
            records
                .retain(|record| record.conversation_code.as_deref() == Some(&conversation_code));
        }
        records.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then(right.id.cmp(&left.id))
        });
        let total = records.len();
        let start = (current - 1) * size;
        Ok(PagedAgentRunHistoryRecords {
            current,
            size,
            total,
            records: records.into_iter().skip(start).take(size).collect(),
        })
    }

    pub fn get_run(&self, run_code: &str) -> Result<AgentRunHistoryRecord, AgentRunHistoryError> {
        self.repository
            .lock()
            .expect("agent run history repository mutex poisoned")
            .get_run(run_code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAguiStreamError {
    StreamKeyRequired,
}

#[derive(Clone)]
pub struct AgentAguiStreamService {
    sink: Arc<AguiEventSink>,
}

impl AgentAguiStreamService {
    pub fn new(sink: Arc<AguiEventSink>) -> Self {
        Self { sink }
    }

    pub fn activate(&self, stream_key: &str) -> Result<AgentAguiStreamState, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        self.sink.activate_stream(stream_key.clone());
        Ok(self.state(stream_key))
    }

    pub fn deactivate(
        &self,
        stream_key: &str,
    ) -> Result<AgentAguiStreamState, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        self.sink.deactivate_stream(&stream_key);
        Ok(self.state(stream_key))
    }

    pub fn events(&self, stream_key: &str) -> Result<AgentAguiEventSnapshot, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        let snapshot = self.sink.snapshot();
        let events = self
            .sink
            .replay(&stream_key)
            .into_iter()
            .filter_map(|event| convert_agui_event(&stream_key, event))
            .collect::<Vec<_>>();
        Ok(AgentAguiEventSnapshot {
            stream_key,
            active_streams: snapshot.active_streams,
            total_streams: snapshot.total_streams,
            events,
        })
    }

    fn state(&self, stream_key: String) -> AgentAguiStreamState {
        let snapshot = self.sink.snapshot();
        AgentAguiStreamState {
            stream_key,
            active_streams: snapshot.active_streams,
            total_streams: snapshot.total_streams,
            total_replay_bytes: snapshot.total_replay_bytes,
            replay_evicted: snapshot.replay_evicted,
        }
    }
}

#[derive(Clone)]
pub struct AgentInvocationService {
    workspaces: WorkspaceService,
    apps: AppService,
    variables: VariableService,
    skills: SkillRegistryService,
    history: AgentRunHistoryService,
    event_bus: EventBusHandle,
}

impl AgentInvocationService {
    pub fn new(
        workspaces: WorkspaceService,
        apps: AppService,
        variables: VariableService,
        skills: SkillRegistryService,
        history: AgentRunHistoryService,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            workspaces,
            apps,
            variables,
            skills,
            history,
            event_bus,
        }
    }

    pub async fn invoke(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        let prompt = command
            .prompt
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or(AgentInvocationError::PromptRequired)?;
        let workspace_code = command
            .workspace_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or(AgentInvocationError::WorkspaceRequired)?;
        let workspace = self
            .workspaces
            .get_workspace(&workspace_code)
            .map_err(|_| AgentInvocationError::WorkspaceInvalid)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(AgentInvocationError::WorkspaceInvalid);
        }
        let model = command.model.ok_or(AgentInvocationError::ModelRequired)?;
        let model_code = model.model_code.clone();
        let app = command
            .app_code
            .as_deref()
            .map(|app_code| self.apps.get_app(app_code))
            .transpose()
            .map_err(|_| AgentInvocationError::WorkspaceInvalid)?;
        if let Some(app) = &app {
            if app.workspace_code != workspace.workspace_code {
                return Err(AgentInvocationError::WorkspaceInvalid);
            }
        }
        let variables = self
            .variables
            .list_variables(
                &workspace.workspace_code,
                crate::domain::variable::VariableCatalogQuery { search: None },
            )
            .unwrap_or_default()
            .into_iter()
            .map(|variable| EffectiveVariable {
                key: variable.key,
                value: variable.value,
                source: variable.source_label,
            })
            .collect::<Vec<_>>();
        let skills = self
            .skills
            .list_workspace_skills(
                &workspace.workspace_code,
                crate::domain::skill::WorkspaceSkillListQuery { search: None },
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|skill| skill.enabled)
            .map(|skill| EffectiveSkill {
                skill_name: skill.skill_name,
                description: skill.source_label.unwrap_or_default(),
                instruction: None,
                tool_names: Vec::new(),
            })
            .collect::<Vec<_>>();

        let run_code = format!("RUN-{}", RUN_SEQUENCE.fetch_add(1, Ordering::SeqCst));
        let context = AgentRunContext {
            conversation_code: command.conversation_code.clone(),
            run_code: run_code.clone(),
            workspace_code: workspace.workspace_code.clone(),
            user_code: user_code.clone(),
            request_id: None,
            stream_key: command
                .conversation_code
                .clone()
                .or_else(|| Some(run_code.clone())),
            agent_kind: AgentKind::React,
            model,
            variables,
            skills,
            tools: command.tools,
            execution_scope: AgentExecutionScope {
                app_code: app.as_ref().map(|app| app.app_code.clone()),
                repository_url: app.as_ref().and_then(|app| app.repository_url.clone()),
                local_directory: app.as_ref().map(|app| app.local_directory.clone()),
                branch_name: app.as_ref().and_then(|app| app.default_branch.clone()),
                task_code: command.task_code,
            },
            limits: command.limits.unwrap_or_default(),
            messages: vec![AgentMessage::user(prompt)],
            observations: Vec::new(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        let agent = ReactAgentBuilder::new(
            Arc::new(OpenAiCompatibleModelClient::default()),
            Arc::new(ToolRegistry::default()),
            self.event_bus.clone(),
        )
        .build()
        .map_err(map_agent_error)?;
        let started_at = context.started_at.clone();
        let conversation_code = context.conversation_code.clone();
        let workspace_code = context.workspace_code.clone();
        let app_code = context.execution_scope.app_code.clone();
        let task_code = context.execution_scope.task_code.clone();

        match agent.run(context).await {
            Ok(result) => {
                let finished_at = current_timestamp();
                self.history
                    .create_run(AgentRunHistoryRecord {
                        id: 0,
                        run_code: result.run_code.clone(),
                        conversation_code: result.conversation_code.clone(),
                        workspace_code: result.workspace_code.clone(),
                        user_code,
                        app_code,
                        task_code,
                        model_code,
                        status: AgentRunStatus::Completed,
                        outcome: "completed".into(),
                        step_count: result.step_count,
                        observation_count: result.observations.len() as u32,
                        output_preview: Some(safe_preview("output", &result.final_output)),
                        error_code: None,
                        error_message: None,
                        started_at,
                        finished_at,
                        created_at: current_timestamp(),
                    })
                    .map_err(|_| AgentInvocationError::HistoryStorageFailed)?;
                Ok(AgentInvocationResult {
                    run_code: result.run_code,
                    conversation_code: result.conversation_code,
                    workspace_code: result.workspace_code,
                    status: "COMPLETED",
                    final_output: result.final_output,
                    step_count: result.step_count,
                })
            }
            Err(error) => {
                let invocation_error = map_agent_error(error);
                let finished_at = current_timestamp();
                self.history
                    .create_run(AgentRunHistoryRecord {
                        id: 0,
                        run_code,
                        conversation_code,
                        workspace_code,
                        user_code,
                        app_code,
                        task_code,
                        model_code,
                        status: AgentRunStatus::Failed,
                        outcome: "failed".into(),
                        step_count: 0,
                        observation_count: 0,
                        output_preview: None,
                        error_code: Some(invocation_error.code().into()),
                        error_message: Some(safe_preview(
                            "diagnostic",
                            &invocation_error.message(),
                        )),
                        started_at,
                        finished_at,
                        created_at: current_timestamp(),
                    })
                    .map_err(|_| AgentInvocationError::HistoryStorageFailed)?;
                Err(invocation_error)
            }
        }
    }
}

impl AgentInvocationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PromptRequired => "AGENT_PROMPT_REQUIRED",
            Self::WorkspaceRequired => "AGENT_WORKSPACE_REQUIRED",
            Self::WorkspaceInvalid => "AGENT_WORKSPACE_INVALID",
            Self::ModelRequired => "AGENT_MODEL_REQUIRED",
            Self::HistoryStorageFailed => "AGENT_HISTORY_STORAGE_FAILED",
            Self::RuntimeFailed(_) => "AGENT_RUNTIME_FAILED",
            Self::ModelFailed(_) => "AGENT_MODEL_FAILED",
            Self::ToolFailed(_) => "AGENT_TOOL_FAILED",
            Self::MaxStepsExceeded => "AGENT_MAX_STEPS_EXCEEDED",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::PromptRequired => "prompt is required".into(),
            Self::WorkspaceRequired => "workspaceCode is required".into(),
            Self::WorkspaceInvalid => "workspace must reference an active workspace".into(),
            Self::ModelRequired => "model config is required".into(),
            Self::HistoryStorageFailed => "agent run history storage failed".into(),
            Self::RuntimeFailed(message)
            | Self::ModelFailed(message)
            | Self::ToolFailed(message) => message.clone(),
            Self::MaxStepsExceeded => "agent run exceeded max steps".into(),
        }
    }
}

fn map_agent_error(error: AgentError) -> AgentInvocationError {
    match error {
        AgentError::ModelFailure(message) => AgentInvocationError::ModelFailed(message),
        AgentError::ToolFailure(message)
        | AgentError::MissingTool(message)
        | AgentError::UnauthorizedTool(message) => AgentInvocationError::ToolFailed(message),
        AgentError::MaxStepsExceeded => AgentInvocationError::MaxStepsExceeded,
        AgentError::InvalidConfiguration(message) | AgentError::EventPublishFailed(message) => {
            AgentInvocationError::RuntimeFailed(message)
        }
    }
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_stream_key(stream_key: &str) -> Result<String, AgentAguiStreamError> {
    let stream_key = stream_key.trim();
    if stream_key.is_empty() {
        return Err(AgentAguiStreamError::StreamKeyRequired);
    }
    Ok(stream_key.to_string())
}

fn safe_preview(label: &str, value: &str) -> String {
    format!("{label}_chars:{}", value.chars().count())
}

fn convert_agui_event(stream_key: &str, event: BusEvent) -> Option<AgentAguiEvent> {
    if event.route_class != BusRouteClass::Agui || !event.event_name.starts_with("agent.") {
        return None;
    }
    let run_code = event
        .subject
        .as_ref()
        .and_then(|subject| subject.subject_code.clone());
    let mut metadata = BTreeMap::new();
    metadata.insert("producer".into(), event.producer.clone());
    if let Some(subject) = &event.subject {
        metadata.insert("subjectType".into(), subject.subject_type.clone());
    }
    if let Some(summary) = &event.summary {
        metadata.insert("summary".into(), summary.clone());
    }
    for attr in event.attrs {
        metadata.insert(attr.key, attr.value);
    }
    Some(AgentAguiEvent {
        id: event.event_id.as_decimal_string(),
        event_type: event.event_name.clone(),
        event_name: event.event_name,
        run_code,
        stream_key: stream_key.to_string(),
        occurred_at: timestamp_from_millis(event.occurred_at_ms),
        outcome: outcome_to_code(event.outcome).into(),
        metadata,
    })
}

fn outcome_to_code(outcome: BusOutcome) -> &'static str {
    match outcome {
        BusOutcome::Success => "SUCCESS",
        BusOutcome::Failure => "FAILURE",
        BusOutcome::Rejected => "REJECTED",
        BusOutcome::Started => "STARTED",
        BusOutcome::Progress => "PROGRESS",
        BusOutcome::Completed => "COMPLETED",
    }
}
