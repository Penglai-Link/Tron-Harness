use std::sync::Arc;

use rand::RngCore;

use crate::{
    application::{
        app::AppService, skill::SkillRegistryService, variable::VariableService,
        workspace::WorkspaceService,
    },
    domain::{
        agent::{
            AgentError, AgentExecutionScope, AgentKind, AgentMessage, AgentRunContext,
            AgentRunHistoryRecord, AgentRunStatus, EffectiveSkill, EffectiveVariable, ToolRegistry,
        },
        skill::WorkspaceSkillListQuery,
        variable::VariableCatalogQuery,
        workspace::WorkspaceStatus,
    },
    shared::{event_bus::EventBusHandle, time::current_timestamp},
};

use super::{
    AgentInvocationCommand, AgentInvocationError, AgentInvocationResult, AgentRunHistoryService,
    ModelClient, ModelStreamSink, ReactAgentBuilder,
};

#[derive(Clone)]
pub struct AgentInvocationService {
    workspaces: WorkspaceService,
    apps: AppService,
    variables: VariableService,
    skills: SkillRegistryService,
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
        self.invoke_with_stream(command, user_code, Some(sink)).await
    }

    async fn invoke_with_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        sink: Option<ModelStreamSink>,
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

        let run_code = command.run_code.unwrap_or_else(generate_agent_run_code);
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
            self.model_client.clone(),
            self.tool_registry.clone(),
            self.event_bus.clone(),
        )
        .build()
        .map_err(map_agent_error)?;
        let started_at = context.started_at.clone();
        let conversation_code = context.conversation_code.clone();
        let workspace_code = context.workspace_code.clone();
        let app_code = context.execution_scope.app_code.clone();
        let task_code = context.execution_scope.task_code.clone();

        let result = if let Some(sink) = sink {
            agent.run_stream(context, sink).await
        } else {
            agent.run(context).await
        };

        match result {
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

pub fn generate_agent_run_code() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut rng = rand::rng();
    format!("RUN-{timestamp}-{:016X}", rng.next_u64())
}

fn safe_preview(label: &str, value: &str) -> String {
    format!("{label}_chars:{}", value.chars().count())
}
