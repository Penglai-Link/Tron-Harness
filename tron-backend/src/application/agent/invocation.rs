use std::sync::Arc;

use rand::RngCore;

use crate::{
    application::{
        app::AppService, skill::SkillRegistryService, variable::VariableService,
        workspace::WorkspaceService,
    },
    domain::{
        agent::{
            AgentExecutionScope, AgentKind, AgentManagementError, AgentMessage, AgentRunContext,
            AgentRunHistoryRecord, AgentRunStatus, EffectiveSkill, EffectiveVariable, ToolRegistry,
        },
        skill::WorkspaceSkillListQuery,
        variable::VariableCatalogQuery,
        workspace::WorkspaceStatus,
    },
    shared::{event_bus::EventBusHandle, time::current_timestamp},
};

use super::{
    AgentInvocationCommand, AgentInvocationError, AgentInvocationResult, AgentManagementService,
    AgentOrchestrator, AgentRunHistoryService, AgentRuntimeRegistry, ModelClient, ModelStreamSink,
    ResolvedSystemAgentRuntime, RuntimeAdapterSelection, SystemAgentCliKind,
};

const MAIN_AGENT_CODE: &str = "main-agent";

#[derive(Clone)]
pub struct AgentInvocationService {
    workspaces: WorkspaceService,
    apps: AppService,
    variables: VariableService,
    skills: SkillRegistryService,
    management: AgentManagementService,
    history: AgentRunHistoryService,
    event_bus: EventBusHandle,
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
}

impl AgentInvocationService {
    pub fn new(
        workspaces: WorkspaceService,
        apps: AppService,
        variables: VariableService,
        skills: SkillRegistryService,
        management: AgentManagementService,
        history: AgentRunHistoryService,
        event_bus: EventBusHandle,
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            workspaces,
            apps,
            variables,
            skills,
            management,
            history,
            event_bus,
            model_client,
            tool_registry,
        }
    }

    pub async fn invoke(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        self.invoke_with_stream(command, user_code, None).await
    }

    pub async fn invoke_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        sink: ModelStreamSink,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        self.invoke_with_stream(command, user_code, Some(sink))
            .await
    }

    async fn invoke_with_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        let AgentInvocationCommand {
            run_code: requested_run_code,
            prompt: requested_prompt,
            workspace_code: requested_workspace_code,
            conversation_code: requested_conversation_code,
            agent_code: requested_agent_code,
            system_agent_session_id: requested_system_agent_session_id,
            app_code: requested_app_code,
            task_code: requested_task_code,
            model: requested_model,
            limits: requested_limits,
            tools: requested_tools,
        } = command;
        let prompt = requested_prompt
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or(AgentInvocationError::PromptRequired)?;
        let workspace_code = requested_workspace_code
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
        let model = requested_model.ok_or(AgentInvocationError::ModelRequired)?;
        let model_code = model.model_code.clone();
        let app = requested_app_code
            .as_deref()
            .map(|app_code| self.apps.get_app(app_code))
            .transpose()
            .map_err(|_| AgentInvocationError::WorkspaceInvalid)?;
        if let Some(app) = &app {
            if app.workspace_code != workspace.workspace_code {
                return Err(AgentInvocationError::WorkspaceInvalid);
            }
        }
        let resolved_agent = resolve_invocation_agent(
            &self.management,
            &workspace.workspace_code,
            requested_agent_code,
        )?;
        let variables = self
            .variables
            .list_variables(
                &workspace.workspace_code,
                VariableCatalogQuery { search: None },
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
                WorkspaceSkillListQuery { search: None },
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

        let run_code = requested_run_code.unwrap_or_else(generate_agent_run_code);
        let mut messages = Vec::new();
        if requested_task_code.is_none() && matches!(resolved_agent, ResolvedInvocationAgent::Main)
        {
            if let Some(guidance) = self.management.current_main_agent_guidance().map_err(|_| {
                AgentInvocationError::RuntimeFailed("main agent config load failed".into())
            })? {
                messages.push(AgentMessage::system(guidance));
            }
        }
        messages.push(AgentMessage::user(prompt));
        let local_directory = app.as_ref().map(|app| app.local_directory.clone());
        let branch_name = app.as_ref().and_then(|app| app.default_branch.clone());
        let repository_url = app.as_ref().and_then(|app| app.repository_url.clone());
        let app_code = app.as_ref().map(|app| app.app_code.clone());
        let task_code = requested_task_code;
        let runtime_registry = AgentRuntimeRegistry::new(
            self.model_client.clone(),
            self.tool_registry.clone(),
            self.event_bus.clone(),
        );
        let orchestrator = AgentOrchestrator::new(runtime_registry);
        let runtime_selection =
            runtime_selection_for_agent(&resolved_agent, local_directory.clone())?;
        let requested_system_agent_session_id =
            normalize_optional(requested_system_agent_session_id);
        let context = AgentRunContext {
            conversation_code: requested_conversation_code.clone(),
            run_code: run_code.clone(),
            workspace_code: workspace.workspace_code.clone(),
            user_code: user_code.clone(),
            request_id: None,
            stream_key: requested_conversation_code
                .clone()
                .or_else(|| Some(run_code.clone())),
            agent_kind: resolved_agent.agent_kind(),
            system_agent_session_id: None,
            model,
            variables,
            skills,
            tools: requested_tools,
            execution_scope: AgentExecutionScope {
                app_code: app_code.clone(),
                repository_url,
                local_directory,
                branch_name,
                task_code: task_code.clone(),
            },
            limits: requested_limits.unwrap_or_default(),
            messages,
            observations: Vec::new(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        let started_at = context.started_at.clone();
        let conversation_code = context.conversation_code.clone();
        let workspace_code = context.workspace_code.clone();

        let result = orchestrator
            .execute_with_selection(
                runtime_selection,
                context,
                requested_system_agent_session_id.clone(),
                sink,
            )
            .await;

        match result {
            Ok(orchestrated) => {
                let result = orchestrated.run;
                let finished_at = current_timestamp();
                let token_usage = result.token_usage;
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
                        input_tokens: token_usage.map(|usage| usage.input_tokens),
                        output_tokens: token_usage.map(|usage| usage.output_tokens),
                        total_tokens: token_usage.map(|usage| usage.total_tokens),
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
                    runtime_adapter_key: orchestrated.runtime_adapter_key,
                    status: "COMPLETED",
                    final_output: result.final_output,
                    step_count: result.step_count,
                    token_usage,
                    system_agent_session_id: result.system_agent_session_id,
                })
            }
            Err(invocation_error) => {
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
                        input_tokens: None,
                        output_tokens: None,
                        total_tokens: None,
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

#[derive(Debug, Clone)]
enum ResolvedInvocationAgent {
    Main,
    System(ResolvedSystemAgentRuntime),
}

impl ResolvedInvocationAgent {
    fn agent_kind(&self) -> AgentKind {
        match self {
            Self::Main => AgentKind::React,
            Self::System(_) => AgentKind::SystemCli,
        }
    }
}

fn resolve_invocation_agent(
    management: &AgentManagementService,
    workspace_code: &str,
    requested_agent_code: Option<String>,
) -> Result<ResolvedInvocationAgent, AgentInvocationError> {
    let normalized = normalize_optional(requested_agent_code);
    if normalized
        .as_deref()
        .map(is_main_agent_code)
        .unwrap_or(true)
    {
        return Ok(ResolvedInvocationAgent::Main);
    }
    let agent_code = normalized.expect("normalized agent code should exist");
    let runtime = management
        .resolve_workspace_system_agent_runtime(workspace_code, &agent_code)
        .map_err(|error| map_system_agent_error(&agent_code, error))?;
    Ok(ResolvedInvocationAgent::System(runtime))
}

fn runtime_selection_for_agent(
    resolved_agent: &ResolvedInvocationAgent,
    local_directory: Option<String>,
) -> Result<RuntimeAdapterSelection, AgentInvocationError> {
    match resolved_agent {
        ResolvedInvocationAgent::Main => Ok(RuntimeAdapterSelection::MainReact),
        ResolvedInvocationAgent::System(runtime) => {
            let kind =
                SystemAgentCliKind::from_agent_code(&runtime.agent_code).ok_or_else(|| {
                    AgentInvocationError::RuntimeFailed(format!(
                        "system agent `{}` runtime adapter is not supported",
                        runtime.agent_code
                    ))
                })?;
            match kind {
                SystemAgentCliKind::Codex => Ok(RuntimeAdapterSelection::CodexExec {
                    command_name: runtime.command_name.clone(),
                    working_directory: local_directory,
                }),
                SystemAgentCliKind::OpenCode => Ok(RuntimeAdapterSelection::OpenCodeRun {
                    command_name: runtime.command_name.clone(),
                    working_directory: local_directory,
                }),
            }
        }
    }
}

fn map_system_agent_error(agent_code: &str, error: AgentManagementError) -> AgentInvocationError {
    match error {
        AgentManagementError::InvalidWorkspace => AgentInvocationError::WorkspaceInvalid,
        AgentManagementError::SystemAgentNotFound => AgentInvocationError::RuntimeFailed(format!(
            "system agent `{agent_code}` was not found"
        )),
        AgentManagementError::SystemAgentUnavailable => AgentInvocationError::RuntimeFailed(
            format!("system agent `{agent_code}` is unavailable or disabled for this workspace"),
        ),
        AgentManagementError::StorageFailure => {
            AgentInvocationError::RuntimeFailed("system agent configuration load failed".into())
        }
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_main_agent_code(agent_code: &str) -> bool {
    agent_code.eq_ignore_ascii_case(MAIN_AGENT_CODE)
}

pub fn generate_agent_run_code() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut rng = rand::rng();
    format!("RUN-{timestamp}-{:016X}", rng.next_u64())
}

fn safe_preview(label: &str, value: &str) -> String {
    format!("{label}_chars:{}", value.chars().count())
}
