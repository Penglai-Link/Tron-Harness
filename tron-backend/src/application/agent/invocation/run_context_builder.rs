use std::{collections::HashMap, sync::Arc};

use crate::{
    application::{
        app::AppService, mcp::McpService, prompt::PromptService,
        short_term_memory::ShortTermMemoryService, skill::SkillRegistryService,
        variable::VariableService, workspace::WorkspaceService,
    },
    domain::{
        agent::{
            AgentExecutionScope, AgentMessage, AgentModelConfig, AgentPromptPrefixSource,
            AgentRunContext, AgentRunLimits, AgentRuntimeType, AgentToolExposureState,
            AutoModelRouteContext, EffectiveSkill, EffectiveVariable, ToolAvailabilityState,
            ToolExposureMode, ToolRegistry,
        },
        prompt::PromptRuntimeProvider,
        skill::WorkspaceSkillListQuery,
        workspace::WorkspaceStatus,
    },
    shared::{event_bus::EventBusHandle, locale::SupportedLocale},
};

use crate::application::agent::{
    AgentInvocationCommand, AgentInvocationError, AgentManagementService, BusinessToolCatalog,
    ResolvedAgentRuntime, RuntimeAdapterSelection, discovery_expand_tool_definition,
    domain_discovery_runtime_values, tool_record_to_effective_tool,
};

use super::{
    agui_warning::{McpWarningPublishContext, publish_mcp_runtime_warnings},
    attachments::user_prompt_message,
    build_system_runtime_model_config, generate_agent_run_code,
    helpers::{
        agent_kind_for_runtime, is_main_agent, map_agent_error, map_mcp_error,
        normalize_optional, required_runtime_command_name, runtime_type_label,
    },
    short_term_memory::activate_short_term_memory,
};

const MAIN_AGENT_CODE: &str = "main-agent";

fn managed_system_prompt(
    prompt_service: Option<&PromptService>,
    workspace_code: &str,
    catalog_key: &str,
    locale: &str,
    runtime_values: &HashMap<PromptRuntimeProvider, serde_json::Value>,
) -> Result<String, AgentInvocationError> {
    if let Some(prompt_service) = prompt_service {
        match prompt_service.runtime_prompt_text(
            Some(workspace_code),
            catalog_key,
            locale,
            runtime_values,
        ) {
            Ok(Some(prompt)) => return Ok(prompt),
            Ok(None) => {}
            Err(error) => {
                return Err(AgentInvocationError::RuntimeFailed(format!(
                    "managed system prompt could not be resolved: {error}"
                )));
            }
        }
    }
    Ok(
        crate::domain::prompt::builtin_prompt_text(catalog_key, locale)
            .unwrap_or_else(|| panic!("managed system prompt is missing: {catalog_key}"))
            .to_string(),
    )
}

fn managed_system_prompt_source(catalog_key: &str, locale: &str) -> AgentPromptPrefixSource {
    let definition = crate::domain::prompt::builtin_prompt(catalog_key)
        .unwrap_or_else(|| panic!("managed system prompt is missing: {catalog_key}"));
    AgentPromptPrefixSource {
        catalog_key: definition.catalog_key.clone(),
        prompt_code: definition.prompt_code.clone(),
        locale: locale.to_string(),
    }
}

fn invocation_locale(request_locale: Option<&str>) -> String {
    SupportedLocale::from_raw(request_locale.unwrap_or("zh-CN"))
        .as_str()
        .to_string()
}

pub(crate) struct PreparedAgentRun {
    pub run_code: String,
    pub conversation_code: Option<String>,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub app_code: Option<String>,
    pub task_code: Option<String>,
    pub agent_code: String,
    pub objective: String,
    pub model_code: String,
    pub requested_system_agent_session_id: Option<String>,
    pub runtime_selection: RuntimeAdapterSelection,
    pub context: AgentRunContext,
}

#[derive(Clone)]
pub(crate) struct AgentRunContextBuilder {
    workspaces: WorkspaceService,
    apps: AppService,
    variables: VariableService,
    skills: SkillRegistryService,
    management: AgentManagementService,
    mcp: McpService,
    event_bus: EventBusHandle,
    tool_registry: Arc<ToolRegistry>,
    short_term_memory: Option<ShortTermMemoryService>,
    prompt_service: Option<PromptService>,
    auto_model_route: Option<AutoModelRouteContext>,
    directory_inspector: Option<Arc<dyn crate::application::project::ApplicationDirectoryInspector>>,
}

impl AgentRunContextBuilder {
    pub(crate) fn new(
        workspaces: WorkspaceService,
        apps: AppService,
        variables: VariableService,
        skills: SkillRegistryService,
        management: AgentManagementService,
        mcp: McpService,
        event_bus: EventBusHandle,
        tool_registry: Arc<ToolRegistry>,
        short_term_memory: Option<ShortTermMemoryService>,
        prompt_service: Option<PromptService>,
        auto_model_route: Option<AutoModelRouteContext>,
        directory_inspector: Option<Arc<dyn crate::application::project::ApplicationDirectoryInspector>>,
    ) -> Self {
        Self {
            workspaces,
            apps,
            variables,
            skills,
            management,
            mcp,
            event_bus,
            tool_registry,
            short_term_memory,
            prompt_service,
            auto_model_route,
            directory_inspector,
        }
    }

    pub(crate) async fn prepare(
        &self,
        command: AgentInvocationCommand,
        user_code: Option<String>,
        locale: Option<String>,
    ) -> Result<PreparedAgentRun, AgentInvocationError> {
        let AgentInvocationCommand {
            run_code: requested_run_code,
            prompt: requested_prompt,
            workspace_code: requested_workspace_code,
            conversation_code: requested_conversation_code,
            agent_code: requested_agent_code,
            system_model_id: requested_system_model_id,
            system_agent_session_id: requested_system_agent_session_id,
            app_code: requested_app_code,
            execution_scope_override,
            task_code: requested_task_code,
            model: requested_model,
            limits: requested_limits,
            tools: requested_tools,
            attachments: requested_attachments,
            supports_user_input_requests,
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
        if let Some(app) = &app
            && app.workspace_code != workspace.workspace_code
        {
            return Err(AgentInvocationError::WorkspaceInvalid);
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
        let locale = invocation_locale(locale.as_deref());
        let run_code = requested_run_code.unwrap_or_else(generate_agent_run_code);
        let mut tools = requested_tools;
        let mut effective_tools = tools.clone();
        let mut tool_exposure = AgentToolExposureState::default();
        let internal_agents = self
            .management
            .list_workspace_internal_agents(&workspace.workspace_code)
            .map_err(|error| map_agent_error(&resolved_agent.agent_code, error))?;
        let internal_agent_policy = if internal_agents
            .iter()
            .any(|policy| policy.agent_code.eq_ignore_ascii_case(&resolved_agent.agent_code))
        {
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
                    effective_tools = tools.clone();
                }
                ToolExposureMode::DomainDiscovery => {
                    let visible_domains =
                        BusinessToolCatalog::domains_for_records(&effective_tool_records, &locale);
                    let hidden_tools = effective_tool_records
                        .into_iter()
                        .map(tool_record_to_effective_tool)
                        .collect::<Vec<_>>();
                    let discovery_tool = discovery_expand_tool_definition(&locale);
                    tools = vec![discovery_tool.clone()];
                    effective_tools = Vec::with_capacity(hidden_tools.len() + 1);
                    effective_tools.push(discovery_tool);
                    effective_tools.extend(hidden_tools.iter().cloned());
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
            .collect::<HashMap<_, _>>();
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
            effective_tools.extend(materialization.tools.iter().cloned());
            tools.extend(materialization.tools);
        }

        if matches!(resolved_agent.runtime_type, AgentRuntimeType::React)
            && requested_conversation_code.is_some()
            && supports_user_input_requests
            && tools
                .iter()
                .all(|tool| tool.tool_name != crate::application::agent::CHAT_ASK_USER_TOOL)
        {
            let ask_user = crate::application::agent::ask_user_definition(&locale);
            tools.push(ask_user.clone());
            effective_tools.push(ask_user);
        }

        let mut messages = Vec::new();
        let mut prompt_prefix_sources = Vec::new();
        let empty_runtime_prompt_values = HashMap::new();
        if requested_task_code.is_none() && is_main_agent(&resolved_agent.agent_code) {
            if let Some(guidance) = self.management.current_main_agent_guidance(&locale).map_err(|_| {
                AgentInvocationError::RuntimeFailed("main agent config load failed".into())
            })? {
                messages.push(AgentMessage::system(guidance));
                prompt_prefix_sources.push(None);
            }
            messages.push(AgentMessage::system(managed_system_prompt(
                self.prompt_service.as_ref(),
                &workspace.workspace_code,
                "agent.final-output",
                &locale,
                &empty_runtime_prompt_values,
            )?));
            prompt_prefix_sources.push(Some(managed_system_prompt_source(
                "agent.final-output",
                &locale,
            )));
        }
        if matches!(resolved_agent.runtime_type, AgentRuntimeType::React)
            && requested_conversation_code.is_some()
            && supports_user_input_requests
        {
            messages.push(AgentMessage::system(managed_system_prompt(
                self.prompt_service.as_ref(),
                &workspace.workspace_code,
                "chat.ask-user",
                &locale,
                &empty_runtime_prompt_values,
            )?));
            prompt_prefix_sources.push(Some(managed_system_prompt_source(
                "chat.ask-user",
                &locale,
            )));
        }
        if tool_exposure.tool_exposure_mode == Some(ToolExposureMode::DomainDiscovery)
            && !tool_exposure.visible_domains.is_empty()
        {
            let persistence_mode = tool_exposure
                .expanded_domain_persistence_mode
                .expect("domain discovery mode should include persistence mode");
            let runtime_values = domain_discovery_runtime_values(
                &tool_exposure.visible_domains,
                persistence_mode,
                &locale,
            );
            messages.push(AgentMessage::system(managed_system_prompt(
                self.prompt_service.as_ref(),
                &workspace.workspace_code,
                "agent.domain-discovery",
                &locale,
                &runtime_values,
            )?));
            prompt_prefix_sources.push(Some(managed_system_prompt_source(
                "agent.domain-discovery",
                &locale,
            )));
        }
        let mut prompt_prefix_messages = messages.clone();
        let user_prompt_message = user_prompt_message(prompt.clone(), &requested_attachments);
        messages.push(user_prompt_message.clone());

        let local_directory = execution_scope_override
            .as_ref()
            .and_then(|scope| scope.local_directory.clone())
            .or_else(|| app.as_ref().map(|app| app.local_directory.clone()));
        let branch_name = execution_scope_override
            .as_ref()
            .and_then(|scope| scope.branch_name.clone());
        let repository_url = execution_scope_override
            .as_ref()
            .and_then(|scope| scope.repository_url.clone())
            .or_else(|| app.as_ref().and_then(|app| app.repository_url.clone()));
        let app_code = execution_scope_override
            .as_ref()
            .and_then(|scope| scope.app_code.clone())
            .or_else(|| app.as_ref().map(|app| app.app_code.clone()));
        let task_code = requested_task_code;
        let runtime_selection =
            runtime_selection_for_agent(&resolved_agent, local_directory.clone())?;
        let requested_system_agent_session_id =
            normalize_optional(requested_system_agent_session_id);
        let mut context = AgentRunContext {
            auto_model_route: self.auto_model_route.clone(),
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
            effective_tools,
            tool_exposure,
            execution_scope: execution_scope_override.unwrap_or(AgentExecutionScope {
                applications: Vec::new(),
                project_code: None,
                app_code: app_code.clone(),
                repository_url,
                local_directory,
                branch_name,
                task_code: task_code.clone(),
                allowed_local_directories: Vec::new(),
            }),
            limits,
            messages,
            observations: Vec::new(),
            short_term_memory: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Some(message) =
            crate::application::agent::execution_context_message(&context.execution_scope)
        {
            prompt_prefix_messages.push(message.clone());
            prompt_prefix_sources.push(None);
            context
                .messages
                .insert(context.messages.len().saturating_sub(1), message);
        }
        if context.agent_kind == crate::domain::agent::AgentKind::React {
            if let Some(inspector) = &self.directory_inspector {
                if let Some(message) = crate::application::agent::project_instructions_message(&context.execution_scope, inspector.as_ref())
                    .map_err(AgentInvocationError::RuntimeFailed)? {
                    prompt_prefix_messages.push(message.clone());
                    prompt_prefix_sources.push(None);
                    context.messages.insert(context.messages.len().saturating_sub(1), message);
                }
            }
        }
        activate_short_term_memory(
            self.short_term_memory.as_ref(),
            &mut context,
            &runtime_selection,
            requested_conversation_code.as_deref(),
            &resolved_agent.agent_code,
            &prompt,
            prompt_prefix_messages,
            prompt_prefix_sources,
            Some(user_prompt_message),
            transcript_input_kind.as_deref(),
            transcript_input_metadata,
        )?;

        Ok(PreparedAgentRun {
            run_code,
            conversation_code: context.conversation_code.clone(),
            workspace_code: context.workspace_code.clone(),
            user_code,
            app_code,
            task_code,
            agent_code: resolved_agent.agent_code,
            objective: prompt,
            model_code,
            requested_system_agent_session_id,
            runtime_selection,
            context,
        })
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
