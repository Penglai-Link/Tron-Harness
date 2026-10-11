//! Named definitions and per-field platform inheritance.
use super::*;

pub(super) fn default_main_agent_role_prompt() -> String {
    crate::domain::prompt::builtin_prompt_text("agent.role.main", "zh-CN")
        .unwrap_or_default()
        .to_owned()
}

fn builtin_role_prompt(catalog_key: &str) -> String {
    crate::domain::prompt::builtin_prompt_text(catalog_key, "zh-CN")
        .unwrap_or_default()
        .to_owned()
}

#[derive(Debug, Clone)]
pub(super) struct InternalAgentDefinition {
    pub(super) agent_code: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) role_prompt: String,
    pub(super) loop_code: Option<String>,
    pub(super) avatar_id: String,
    pub(super) candidate_model_codes: Vec<String>,
    pub(super) group_name: Option<String>,
    pub(super) tool_assignment_mode: ToolAssignmentMode,
    pub(super) mcp_access_mode: McpAccessMode,
    pub(super) tool_exposure_mode: ToolExposureMode,
    pub(super) expanded_domain_persistence_mode: ExpandedDomainPersistenceMode,
    pub(super) max_steps: u32,
    pub(super) max_tool_calls: u32,
    pub(super) max_parallel_tool_calls: u32,
    pub(super) selected_tool_codes: Vec<String>,
    pub(super) selected_server_codes: Vec<String>,
    pub(super) platform_defined: bool,
    pub(super) built_in: bool,
}

pub(super) fn internal_agents() -> Vec<InternalAgentDefinition> {
    let mut definitions = vec![InternalAgentDefinition {
        agent_code: MAIN_AGENT_CODE.to_string(),
        name: "Main Agent".to_string(),
        description: "Primary workspace internal agent with access to platform business tools."
            .to_string(),
        role_prompt: default_main_agent_role_prompt(),
        avatar_id: "robot".into(),
        candidate_model_codes: Vec::new(),
        group_name: Some("主智能体".into()),
        loop_code: None,
        tool_assignment_mode: ToolAssignmentMode::AllPlatformTools,
        mcp_access_mode: McpAccessMode::AllEnabledServers,
        tool_exposure_mode: ToolExposureMode::DomainDiscovery,
        expanded_domain_persistence_mode: ExpandedDomainPersistenceMode::StickyExpanded,
        max_steps: 100,
        max_tool_calls: 100,
        max_parallel_tool_calls: 4,
        selected_tool_codes: vec![],
        selected_server_codes: vec![],
        platform_defined: true,
        built_in: true,
    }];
    for (agent_code, name, description, catalog_key, group_name, avatar_id) in [
        (
            "explore",
            "探索智能体",
            "只读探索任务使用的内置智能体。",
            "agent.role.explorer",
            "探索",
            "fox",
        ),
        (
            "full-stack",
            "全栈工程师",
            "用于全栈开发任务的内置智能体。",
            "agent.role.full-stack-developer",
            "开发",
            "panda",
        ),
        (
            "product",
            "杨布斯",
            "用于产品分析与设计任务的内置智能体。",
            "agent.role.product-manager",
            "设计",
            "astronaut",
        ),
    ] {
        definitions.push(InternalAgentDefinition {
            agent_code: agent_code.to_owned(),
            name: name.to_owned(),
            description: description.to_owned(),
            role_prompt: builtin_role_prompt(catalog_key),
            avatar_id: avatar_id.into(),
            candidate_model_codes: Vec::new(),
            group_name: Some(group_name.to_owned()),
            loop_code: None,
            tool_assignment_mode: ToolAssignmentMode::AllPlatformTools,
            mcp_access_mode: McpAccessMode::AllEnabledServers,
            tool_exposure_mode: ToolExposureMode::DomainDiscovery,
            expanded_domain_persistence_mode: ExpandedDomainPersistenceMode::StickyExpanded,
            max_steps: 100,
            max_tool_calls: 100,
            max_parallel_tool_calls: 4,
            selected_tool_codes: vec![],
            selected_server_codes: vec![],
            platform_defined: true,
            built_in: true,
        });
    }
    definitions
}

impl AgentManagementService {
    pub(super) fn validate_new_agent_code(
        &self,
        code: &str,
    ) -> Result<String, AgentManagementError> {
        let code = code.trim();
        let valid = !code.is_empty()
            && code.len() <= 64
            && code.as_bytes()[0].is_ascii_lowercase()
            && code.bytes().all(|ch| {
                ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'-' || ch == b'_'
            });
        if !valid {
            return Err(AgentManagementError::InternalAgentCodeInvalid);
        }
        if is_builtin_agent(code)
            || self
                .supported_agents
                .iter()
                .any(|agent| agent.agent_code.eq_ignore_ascii_case(code))
        {
            return Err(AgentManagementError::InternalAgentCodeConflict);
        }
        Ok(code.to_owned())
    }

    pub(super) fn internal_agent_definitions(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<InternalAgentDefinition>, AgentManagementError> {
        let saved = self
            .repository()
            .list_workspace_internal_agent_definitions(workspace_code)?;
        let definitions = self
            .platform_internal_agent_configs()?
            .into_iter()
            .map(|config| InternalAgentDefinition {
                built_in: is_builtin_agent(&config.agent_code),
                agent_code: config.agent_code,
                name: config.name,
                description: "Platform internal agent.".into(),
                role_prompt: config.role_prompt,
                loop_code: config.loop_code,
                avatar_id: config.avatar_id,
                candidate_model_codes: config.candidate_model_codes,
                group_name: config.group_name,
                tool_assignment_mode: config.tool_assignment_mode,
                mcp_access_mode: config.mcp_access_mode,
                tool_exposure_mode: config.tool_exposure_mode,
                expanded_domain_persistence_mode: config.expanded_domain_persistence_mode,
                max_steps: config.max_steps,
                max_tool_calls: config.max_tool_calls,
                max_parallel_tool_calls: config.max_parallel_tool_calls,
                selected_tool_codes: config.selected_tool_codes,
                selected_server_codes: config.selected_server_codes,
                platform_defined: true,
            })
            .collect::<Vec<_>>();
        let mut assigned = Vec::new();
        for definition in definitions {
            if self.agent_assigned(&definition.agent_code, workspace_code)? {
                assigned.push(definition);
            }
        }
        let mut definitions = assigned;
        for record in saved {
            if self
                .platform_internal_agent_configs()?
                .iter()
                .any(|config| config.agent_code == record.agent_code)
                && !self.agent_assigned(&record.agent_code, workspace_code)?
            {
                continue;
            }
            if let Some(definition) = definitions
                .iter_mut()
                .find(|definition| definition.agent_code == record.agent_code)
            {
                if let Some(group_name) = record.group_name {
                    definition.group_name = (!group_name.is_empty()).then_some(group_name);
                }
                if let Some(codes) = record.candidate_model_codes {
                    definition.candidate_model_codes = codes;
                }
                if let Some(avatar_id) = record.avatar_id {
                    definition.avatar_id = avatar_id;
                }
                if let Some(name) = record.name {
                    definition.name = name;
                }
                if let Some(code) = record.loop_code {
                    definition.loop_code = Some(code);
                }
                if let Some(prompt) = record.role_prompt {
                    definition.role_prompt = prompt;
                }
            } else {
                let mut definition = internal_agents().remove(0);
                definition.agent_code = record.agent_code;
                definition.name = record.name.unwrap_or_default();
                definition.description = "Workspace internal agent.".into();
                definition.role_prompt = record.role_prompt.unwrap_or_default();
                definition.loop_code = record.loop_code;
                definition.avatar_id = record.avatar_id.unwrap_or_else(|| "robot".into());
                definition.candidate_model_codes = record.candidate_model_codes.unwrap_or_default();
                definition.group_name = record.group_name.filter(|name| !name.is_empty());
                definition.platform_defined = false;
                definition.built_in = false;
                definitions.push(definition);
            }
        }
        Ok(definitions)
    }

    pub fn create_internal_agent(
        &self,
        workspace_code: &str,
        command: CreateWorkspaceInternalAgentCommand,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        self.active_workspace_detail(workspace_code)?;
        validate_definition(&command.name, &command.role_prompt)?;
        let loop_code = self.validate_loop_selection(command.loop_code.as_deref())?;
        let agent_code = self.validate_new_agent_code(&command.agent_code)?;
        let group_name = self.validate_internal_agent_group(workspace_code, command.group_name)?;
        let now = current_timestamp();
        let definition = WorkspaceInternalAgentDefinition {
            workspace_code: workspace_code.to_owned(),
            agent_code,
            name: Some(command.name.trim().to_owned()),
            role_prompt: Some(command.role_prompt),
            loop_code: Some(loop_code),
            avatar_id: Some(validate_avatar_id(command.avatar_id.as_deref())?),
            candidate_model_codes: Some(normalize_candidate_model_codes(
                command.candidate_model_codes,
            )?),
            group_name,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repository()
            .insert_workspace_internal_agent_definition(&definition)?;
        self.workspace_internal_agent_policy(workspace_code, &definition.agent_code)
    }

    pub fn update_internal_agent_definition(
        &self,
        workspace_code: &str,
        agent_code: &str,
        command: UpdateWorkspaceInternalAgentDefinitionCommand,
    ) -> Result<InternalAgentToolPolicy, AgentManagementError> {
        ensure_editable_definition(agent_code)?;
        if let Some(id) = &command.avatar_id {
            validate_avatar_id(Some(id))?;
        }
        if let Some(code) = &command.loop_code {
            self.validate_loop_selection(Some(code))?;
        }
        let current = self.workspace_internal_agent_policy(workspace_code, agent_code)?;
        let name = command.name.clone().unwrap_or(current.name);
        let role_prompt = command.role_prompt.clone().unwrap_or(current.role_prompt);
        validate_definition(&name, &role_prompt)?;
        let group_name = self.validate_internal_agent_group(workspace_code, command.group_name)?;
        let now = current_timestamp();
        let saved = self
            .repository()
            .list_workspace_internal_agent_definitions(workspace_code)?
            .into_iter()
            .find(|record| record.agent_code == current.agent_code);
        let created_at = saved
            .as_ref()
            .map(|record| record.created_at.clone())
            .unwrap_or_else(|| now.clone());
        self.repository()
            .upsert_workspace_internal_agent_definition(&WorkspaceInternalAgentDefinition {
                workspace_code: workspace_code.to_owned(),
                agent_code: current.agent_code.clone(),
                name: command
                    .name
                    .map(|name| name.trim().to_owned())
                    .or_else(|| saved.as_ref().and_then(|record| record.name.clone())),
                role_prompt: command
                    .role_prompt
                    .or_else(|| saved.as_ref().and_then(|record| record.role_prompt.clone())),
                loop_code: command
                    .loop_code
                    .or_else(|| saved.as_ref().and_then(|record| record.loop_code.clone())),
                avatar_id: command
                    .avatar_id
                    .or_else(|| saved.as_ref().and_then(|record| record.avatar_id.clone())),
                group_name: group_name
                    .or_else(|| saved.as_ref().and_then(|record| record.group_name.clone())),
                candidate_model_codes: command
                    .candidate_model_codes
                    .map(normalize_candidate_model_codes)
                    .transpose()?
                    .or_else(|| {
                        saved
                            .as_ref()
                            .and_then(|record| record.candidate_model_codes.clone())
                    }),
                created_at,
                updated_at: now,
            })?;
        self.workspace_internal_agent_policy(workspace_code, &current.agent_code)
    }

    pub fn current_internal_agent_guidance(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<Option<String>, AgentManagementError> {
        let policy = self.workspace_internal_agent_policy(workspace_code, agent_code)?;
        if !policy.role_prompt.trim().is_empty() {
            return Ok(Some(policy.role_prompt));
        }
        Ok(None)
    }
}

pub(super) fn validate_definition(
    name: &str,
    role_prompt: &str,
) -> Result<(), AgentManagementError> {
    if name.trim().is_empty() || name.chars().count() > 100 || role_prompt.len() > 65_536 {
        return Err(AgentManagementError::InternalAgentDefinitionInvalid);
    }
    Ok(())
}

// Only built-in asset identifiers are stored, never arbitrary URLs or file paths.
pub(super) fn validate_avatar_id(id: Option<&str>) -> Result<String, AgentManagementError> {
    let id = id.unwrap_or("robot");
    if matches!(
        id,
        "robot"
            | "fox"
            | "cat"
            | "owl"
            | "astronaut"
            | "wizard"
            | "panda"
            | "frog"
            | "penguin"
            | "raccoon"
            | "rabbit"
            | "dragon"
    ) {
        Ok(id.to_owned())
    } else {
        Err(AgentManagementError::InternalAgentDefinitionInvalid)
    }
}

/// Identity comes from the authoritative built-in registry, never a naming convention.
pub(super) fn is_builtin_agent(code: &str) -> bool {
    internal_agents()
        .iter()
        .any(|definition| definition.agent_code.eq_ignore_ascii_case(code))
}
pub(super) fn ensure_editable_definition(code: &str) -> Result<(), AgentManagementError> {
    if is_builtin_agent(code) {
        Err(AgentManagementError::BuiltinAgentReadOnly)
    } else {
        Ok(())
    }
}

pub(super) fn normalize_candidate_model_codes(
    codes: Vec<String>,
) -> Result<Vec<String>, AgentManagementError> {
    if codes.len() > 256
        || codes
            .iter()
            .any(|code| code.trim().is_empty() || code.len() > 512)
    {
        return Err(AgentManagementError::InternalAgentDefinitionInvalid);
    }
    let mut codes = codes
        .into_iter()
        .map(|code| code.trim().to_owned())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();
    Ok(codes)
}
