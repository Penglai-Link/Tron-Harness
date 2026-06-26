mod agui_warning;
mod attachments;
mod helpers;
mod short_term_memory;

use std::sync::Arc;

use rand::RngCore;

use crate::{
    application::{
        app::AppService, mcp::McpService, short_term_memory::ShortTermMemoryService,
        skill::SkillRegistryService, variable::VariableService, workspace::WorkspaceService,
    },
    domain::{
        agent::{
            AgentExecutionScope, AgentMessage, AgentModelConfig, AgentModelProtocol,
            AgentRunContext, AgentRunHistoryRecord, AgentRunLimits, AgentRunStatus,
            AgentRuntimeType, AgentToolExposureState, EffectiveSkill, EffectiveVariable,
            ModelRequest, ModelResponse, ToolAvailabilityState, ToolExposureMode, ToolRegistry,
        },
        skill::WorkspaceSkillListQuery,
        workspace::WorkspaceStatus,
    },
    shared::{event_bus::EventBusHandle, time::current_timestamp},
};

use super::{
    AgentInvocationCommand, AgentInvocationError, AgentInvocationResult, AgentManagementService,
    AgentOrchestrator, AgentRunHistoryService, AgentRuntimeRegistry, BusinessToolCatalog,
    ModelClient, ModelStreamSink, ResolvedAgentRuntime, RuntimeAdapterSelection,
    build_domain_discovery_guidance, discovery_expand_tool_definition,
    tool_record_to_effective_tool,
};
use agui_warning::{McpWarningPublishContext, publish_mcp_runtime_warnings};
use attachments::user_prompt_message;
use helpers::{
    agent_kind_for_runtime, is_internal_agent, is_main_agent, map_agent_error, map_mcp_error,
    normalize_optional, required_runtime_command_name, runtime_type_label, safe_preview,
};
const MAIN_AGENT_CODE: &str = "main-agent";
const MAIN_AGENT_FINAL_OUTPUT_GUIDANCE: &str = r#"Final assistant output contract:
- The UI already renders Thinking and tool progress separately.
- Do not repeat reasoning summaries, tool-progress narration, or step-by-step progress in the final answer.
- Answer with only the final result, concise conclusion, and necessary user-facing evidence such as command results or artifact paths.
- Do not expose chain-of-thought."#;

#[derive(Clone)]
pub struct AgentInvocationService {
    workspaces: WorkspaceService,
    apps: AppService,
    variables: VariableService,
    skills: SkillRegistryService,
    management: AgentManagementService,
    history: AgentRunHistoryService,
    mcp: McpService,
    event_bus: EventBusHandle,
    model_client: Arc<dyn ModelClient>,
    tool_registry: Arc<ToolRegistry>,
    runtime_http_client: reqwest::Client,
    short_term_memory: Option<ShortTermMemoryService>,
}

impl AgentInvocationService {
    pub fn new(
        workspaces: WorkspaceService,
        apps: AppService,
        variables: VariableService,
        skills: SkillRegistryService,
        management: AgentManagementService,
        history: AgentRunHistoryService,
        mcp: McpService,
        event_bus: EventBusHandle,
        model_client: Arc<dyn ModelClient>,
        tool_registry: Arc<ToolRegistry>,
        runtime_http_client: reqwest::Client,
    ) -> Self {
        Self {
            workspaces,
            apps,
            variables,
            skills,
            management,
            history,
            mcp,
            event_bus,
            model_client,
            tool_registry,
            runtime_http_client,
            short_term_memory: None,
        }
    }

    pub fn with_short_term_memory(mut self, service: ShortTermMemoryService) -> Self {
        self.short_term_memory = Some(service);
        self
    }

    pub fn short_term_memory(&self) -> Option<ShortTermMemoryService> {
        self.short_term_memory.clone()
    }

    pub async fn generate_stateless_model(
        &self,
        model: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, AgentInvocationError> {
        self.model_client
            .generate(model, request)
            .await
            .map_err(AgentInvocationError::ModelFailed)
    }

    pub async fn invoke(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        self.invoke_with_stream(command, user_code, locale, None)
            .await
    }

    pub async fn invoke_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: ModelStreamSink,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        self.invoke_with_stream(command, user_code, locale, Some(sink))
            .await
    }

    async fn invoke_with_stream(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
        sink: Option<ModelStreamSink>,
    ) -> Result<AgentInvocationResult, AgentInvocationError> {
        let AgentInvocationCommand {
            run_code: requested_run_code,
            prompt: requested_prompt,
            workspace_code: requested_workspace_code,
            conversation_code: requested_conversation_code,
            agent_code: requested_agent_code,
            system_model_id: requested_system_model_id,
            system_agent_session_id: requested_system_agent_session_id,
            app_code: requested_app_code,
            task_code: requested_task_code,
            model: requested_model,
            limits: requested_limits,
            tools: requested_tools,
            attachments: requested_attachments,
            transcript_input_kind,
            transcript_input_metadata,
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
        let model =
            resolve_invocation_model(&resolved_agent, requested_model, requested_system_model_id)?;
        let model_code = model.model_code.clone();
        let agent_kind = agent_kind_for_runtime(resolved_agent.runtime_type);
        let locale = locale.unwrap_or_else(|| "en-US".to_string());
        let run_code = requested_run_code.unwrap_or_else(generate_agent_run_code);
        let mut tools = requested_tools;
        let mut tool_exposure = AgentToolExposureState::default();
        let internal_agent_policy = if is_internal_agent(&resolved_agent.agent_code) {
            Some(
                self.management
                    .workspace_internal_agent_policy(
                        &workspace.workspace_code,
                        &resolved_agent.agent_code,
                    )
                    .map_err(|error| map_agent_error(&resolved_agent.agent_code, error))?,
            )
        } else {
            None
        };
        if tools.is_empty()
            && let Some(policy) = internal_agent_policy.as_ref()
        {
            let effective_tool_records = self
                .management
                .resolve_internal_agent_effective_tool_definitions(
                    &workspace.workspace_code,
                    &resolved_agent.agent_code,
                    &locale,
                )
                .map_err(|error| map_agent_error(&resolved_agent.agent_code, error))?
                .into_iter()
                .filter(|tool| tool.availability_state == ToolAvailabilityState::Available)
                .collect::<Vec<_>>();
            match policy.tool_exposure_mode {
                ToolExposureMode::DirectList => {
                    tools = effective_tool_records
                        .into_iter()
                        .map(tool_record_to_effective_tool)
                        .collect();
                }
                ToolExposureMode::DomainDiscovery => {
                    let visible_domains =
                        BusinessToolCatalog::domains_for_records(&effective_tool_records, &locale);
                    let hidden_tools = effective_tool_records
                        .into_iter()
                        .map(tool_record_to_effective_tool)
                        .collect::<Vec<_>>();
                    tools = vec![discovery_expand_tool_definition(&locale)];
                    tool_exposure = AgentToolExposureState {
                        tool_exposure_mode: Some(policy.tool_exposure_mode),
                        expanded_domain_persistence_mode: Some(
                            policy.expanded_domain_persistence_mode,
                        ),
                        visible_domains,
                        hidden_tools,
                        expanded_domain_codes: Vec::new(),
                    };
                }
            }
        }
        let limits = requested_limits.unwrap_or_else(|| {
            internal_agent_policy
                .as_ref()
                .map(|policy| AgentRunLimits {
                    max_steps: policy.max_steps,
                    max_tool_calls: policy.max_tool_calls,
                    max_parallel_tool_calls: policy.max_parallel_tool_calls,
                })
                .unwrap_or_default()
        });
        let variables = self
            .variables
            .runtime_variables(&workspace.workspace_code)
            .map_err(|_| AgentInvocationError::RuntimeSecretDeliveryUntrusted)?
            .into_iter()
            .map(|variable| EffectiveVariable {
                key: variable.key,
                value: variable.value,
                source: variable.source,
                sensitive: variable.is_sensitive,
            })
            .collect::<Vec<_>>();
        let variable_values = variables
            .iter()
            .map(|variable| (variable.key.clone(), variable.value.clone()))
            .collect::<std::collections::HashMap<_, _>>();
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

        if let Some(policy) = internal_agent_policy.as_ref() {
            let materialization = self
                .mcp
                .materialize_runtime_tools(
                    &run_code,
                    &workspace.workspace_code,
                    policy.mcp_access_mode,
                    &policy.selected_server_codes,
                    &variable_values,
                    self.tool_registry.as_ref(),
                )
                .await
                .map_err(map_mcp_error)?;
            publish_mcp_runtime_warnings(
                &self.event_bus,
                &McpWarningPublishContext {
                    run_code: &run_code,
                    conversation_code: requested_conversation_code.as_deref(),
                    workspace_code: &workspace.workspace_code,
                    user_code: user_code.as_deref(),
                    agent_kind,
                    model: &model,
                    app_code: requested_app_code.as_deref(),
                    task_code: requested_task_code.as_deref(),
                },
                &materialization.warnings,
            );
            tools.extend(materialization.tools);
        }

        let mut messages = Vec::new();
        if requested_task_code.is_none() && is_main_agent(&resolved_agent.agent_code) {
            if let Some(guidance) = self.management.current_main_agent_guidance().map_err(|_| {
                AgentInvocationError::RuntimeFailed("main agent config load failed".into())
            })? {
                messages.push(AgentMessage::system(guidance));
            }
            messages.push(AgentMessage::system(MAIN_AGENT_FINAL_OUTPUT_GUIDANCE));
        }
        if tool_exposure.tool_exposure_mode == Some(ToolExposureMode::DomainDiscovery)
            && !tool_exposure.visible_domains.is_empty()
        {
            let persistence_mode = tool_exposure
                .expanded_domain_persistence_mode
                .expect("domain discovery mode should include persistence mode");
            messages.push(AgentMessage::system(build_domain_discovery_guidance(
                &tool_exposure.visible_domains,
                persistence_mode,
                &locale,
            )));
        }
        let prompt_prefix_messages = messages.clone();
        let user_prompt_message = user_prompt_message(prompt.clone(), &requested_attachments);
        messages.push(user_prompt_message.clone());
        let local_directory = app.as_ref().map(|app| app.local_directory.clone());
        let branch_name = app.as_ref().and_then(|app| app.default_branch.clone());
        let repository_url = app.as_ref().and_then(|app| app.repository_url.clone());
        let app_code = app.as_ref().map(|app| app.app_code.clone());
        let task_code = requested_task_code;
        let runtime_registry = AgentRuntimeRegistry::new(
            self.model_client.clone(),
            self.tool_registry.clone(),
            self.event_bus.clone(),
            self.runtime_http_client.clone(),
        )
        .with_short_term_memory(self.short_term_memory.clone());
        let orchestrator = AgentOrchestrator::new(runtime_registry);
        let runtime_selection =
            runtime_selection_for_agent(&resolved_agent, local_directory.clone())?;
        let requested_system_agent_session_id =
            normalize_optional(requested_system_agent_session_id);
        let mut context = AgentRunContext {
            conversation_code: requested_conversation_code.clone(),
            run_code: run_code.clone(),
            workspace_code: workspace.workspace_code.clone(),
            user_code: user_code.clone(),
            locale,
            request_id: None,
            stream_key: requested_conversation_code
                .clone()
                .or_else(|| Some(run_code.clone())),
            agent_kind,
            system_agent_session_id: None,
            model,
            variables,
            skills,
            tools,
            tool_exposure,
            execution_scope: AgentExecutionScope {
                app_code: app_code.clone(),
                repository_url,
                local_directory,
                branch_name,
                task_code: task_code.clone(),
            },
            limits,
            messages,
            observations: Vec::new(),
            short_term_memory: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        self.activate_short_term_memory(
            &mut context,
            &runtime_selection,
            requested_conversation_code.as_deref(),
            &resolved_agent.agent_code,
            &prompt,
            prompt_prefix_messages,
            Some(user_prompt_message),
            transcript_input_kind.as_deref(),
            transcript_input_metadata,
        )?;
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
                    latest_request_estimate: result.latest_request_estimate,
                    latest_usage_audit: result.latest_usage_audit,
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

fn resolve_invocation_agent(
    management: &AgentManagementService,
    workspace_code: &str,
    requested_agent_code: Option<String>,
) -> Result<ResolvedAgentRuntime, AgentInvocationError> {
    let normalized = normalize_optional(requested_agent_code);
    let requested_agent_code = normalized.as_deref();
    management
        .resolve_workspace_agent_runtime(workspace_code, requested_agent_code)
        .map_err(|error| map_agent_error(requested_agent_code.unwrap_or(MAIN_AGENT_CODE), error))
}

fn resolve_invocation_model(
    resolved_agent: &ResolvedAgentRuntime,
    requested_model: Option<AgentModelConfig>,
    requested_system_model_id: Option<String>,
) -> Result<AgentModelConfig, AgentInvocationError> {
    match resolved_agent.runtime_type {
        AgentRuntimeType::React => requested_model.ok_or(AgentInvocationError::ModelRequired),
        AgentRuntimeType::CodexExec
        | AgentRuntimeType::CodexAppServer
        | AgentRuntimeType::OpenCodeRun
        | AgentRuntimeType::OpenCodeServe
        | AgentRuntimeType::OpenCodeAcp => {
            let model_name = normalize_optional(requested_system_model_id)
                .ok_or(AgentInvocationError::ModelRequired)?;
            Ok(build_system_runtime_model_config(&model_name, None))
        }
    }
}

fn runtime_selection_for_agent(
    resolved_agent: &ResolvedAgentRuntime,
    local_directory: Option<String>,
) -> Result<RuntimeAdapterSelection, AgentInvocationError> {
    match resolved_agent.runtime_type {
        AgentRuntimeType::React => Ok(RuntimeAdapterSelection::React),
        AgentRuntimeType::CodexExec => Ok(RuntimeAdapterSelection::CodexExec {
            command_name: required_runtime_command_name(resolved_agent)?,
            working_directory: local_directory,
        }),
        AgentRuntimeType::OpenCodeRun => Ok(RuntimeAdapterSelection::OpenCodeRun {
            command_name: required_runtime_command_name(resolved_agent)?,
            working_directory: local_directory,
        }),
        AgentRuntimeType::CodexAppServer
        | AgentRuntimeType::OpenCodeServe
        | AgentRuntimeType::OpenCodeAcp => Err(AgentInvocationError::RuntimeFailed(format!(
            "agent `{}` runtime type `{}` is not configured for invocation",
            resolved_agent.agent_code,
            runtime_type_label(resolved_agent.runtime_type)
        ))),
    }
}

pub fn generate_agent_run_code() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut rng = rand::rng();
    format!("RUN-{timestamp}-{:016X}", rng.next_u64())
}

pub(crate) fn build_system_runtime_model_config(
    model_name: &str,
    context_window_tokens: Option<u32>,
) -> AgentModelConfig {
    AgentModelConfig {
        model_code: "SYSTEM_AGENT_MODEL".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "http://system-agent.internal".into(),
        api_path: None,
        api_key: "sk-system-agent".into(),
        model_name: model_name.to_string(),
        thinking_enabled: true,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens,
        top_p: None,
        max_retries: Some(0),
    }
}
