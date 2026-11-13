use super::*;

impl AgentManagementService {
    pub fn create_system_agent_profile(
        &self,
        code: &str,
        name: &str,
        base: &str,
        model: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        self.create_system_agent_profile_with_settings(
            code, name, base, model, None, None, false, None,
        )
    }
    pub fn update_system_agent_profile(
        &self,
        code: &str,
        name: &str,
        model: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        self.update_system_agent_profile_with_settings(code, name, model, None, None, false, None)
    }
    /// A logical system Agent shares an installed CLI but pins its own model.
    pub fn create_system_agent_profile_with_settings(
        &self,
        code: &str,
        name: &str,
        base: &str,
        model: &str,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
        avatar_id: Option<String>,
        fast_mode: bool,
        description: Option<String>,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        let code = self.validate_new_agent_code(code)?;
        if self
            .platform_internal_agent_configs()?
            .iter()
            .any(|entry| entry.agent_code == code)
        {
            return Err(AgentManagementError::InternalAgentCodeConflict);
        }
        let mut entry = self.required_system_agent(base)?;
        if entry.base_agent_code.is_some() {
            return Err(AgentManagementError::InternalAgentDefinitionInvalid);
        }
        validate_profile(name, model, entry.runtime_type)?;
        validate_fast_mode(entry.runtime_type, fast_mode)?;
        self.validate_system_effort(&entry, model, reasoning_effort)?;
        if let Some(id) = avatar_id.as_deref() {
            super::internal_definitions::validate_avatar_id(Some(id))?;
        }
        entry.reasoning_effort = reasoning_effort;
        entry.avatar_id = avatar_id;
        entry.fast_mode = fast_mode;
        entry.base_agent_code = Some(entry.agent_code.clone());
        entry.agent_code = code;
        entry.name = name.trim().into();
        entry.description = super::internal_definitions::validate_description(
            description.as_deref().unwrap_or(""),
        )?;
        entry.model_id = Some(model.trim().into());
        entry.enabled = true;
        self.repository().insert_system_agent(&entry)?;
        self.profile_record(&entry)
    }

    pub fn update_system_agent_profile_with_settings(
        &self,
        code: &str,
        name: &str,
        model: &str,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
        avatar_id: Option<String>,
        fast_mode: bool,
        description: Option<String>,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        let mut entry = self.required_system_agent(code)?;
        validate_fast_mode(entry.runtime_type, fast_mode)?;
        if let Some(id) = avatar_id.as_deref() {
            super::internal_definitions::validate_avatar_id(Some(id))?;
        }
        if entry.base_agent_code.is_none() {
            if avatar_id.is_none() {
                return Err(AgentManagementError::BuiltinAgentReadOnly);
            }
        } else {
            validate_profile(name, model, entry.runtime_type)?;
            self.validate_system_effort(&entry, model, reasoning_effort)?;
            entry.name = name.trim().into();
            entry.model_id = Some(model.trim().into());
            entry.reasoning_effort = reasoning_effort;
        }
        entry.avatar_id = avatar_id;
        entry.fast_mode = fast_mode;
        if let Some(description) = description {
            entry.description = super::internal_definitions::validate_description(&description)?;
        }
        self.repository().upsert_system_agent(&entry)?;
        self.profile_record(&entry)
    }

    fn validate_system_effort(
        &self,
        entry: &SystemAgentRegistryEntry,
        model: &str,
        effort: Option<crate::domain::reasoning::ReasoningEffort>,
    ) -> Result<(), AgentManagementError> {
        let Some(effort) = effort else {
            return Ok(());
        };
        let base = entry
            .base_agent_code
            .as_deref()
            .unwrap_or(&entry.agent_code);
        let models = self.system_agent_models(base);
        if entry.runtime_type == AgentRuntimeType::DshHeadless
            && !models.iter().any(|record| {
                record.model_id == model.trim()
                    && record.supported_reasoning_efforts.contains(&effort)
            })
        {
            return Err(AgentManagementError::InternalAgentDefinitionInvalid);
        }
        if let Some(record) = models.iter().find(|record| record.model_id == model.trim()) {
            if !record.supported_reasoning_efforts.is_empty()
                && !record.supported_reasoning_efforts.contains(&effort)
            {
                return Err(AgentManagementError::InternalAgentDefinitionInvalid);
            }
        }
        Ok(())
    }

    pub fn delete_system_agent_profile(&self, code: &str) -> Result<(), AgentManagementError> {
        let entry = self.required_system_agent(code)?;
        if entry.base_agent_code.is_none() {
            return Err(AgentManagementError::BuiltinAgentReadOnly);
        }
        self.repository().delete_system_agent(&entry.agent_code)?;
        Ok(())
    }

    fn profile_record(
        &self,
        entry: &SystemAgentRegistryEntry,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        self.list_platform_system_agents()?
            .into_iter()
            .find(|record| record.agent_code == entry.agent_code)
            .ok_or(AgentManagementError::SystemAgentNotFound)
    }

    pub(super) fn resolve_platform_system_runtime(
        &self,
        code: &str,
    ) -> Result<ResolvedAgentRuntime, AgentManagementError> {
        let entry = self.required_system_agent(code)?;
        if !entry.enabled || !entry.available || entry.command_name.trim().is_empty() {
            return Err(AgentManagementError::SystemAgentUnavailable);
        }
        Ok(ResolvedAgentRuntime {
            system_fast_mode: entry.fast_mode,
            system_reasoning_effort: entry.reasoning_effort,
            agent_code: entry.agent_code,
            system_model_id: entry.model_id,
            runtime_type: entry.runtime_type,
            command_name: Some(entry.command_name),
            name: entry.name,
        })
    }
}

fn validate_fast_mode(
    runtime: AgentRuntimeType,
    fast_mode: bool,
) -> Result<(), AgentManagementError> {
    if fast_mode
        && !matches!(
            runtime,
            AgentRuntimeType::CodexExec | AgentRuntimeType::CodexAppServer
        )
    {
        return Err(AgentManagementError::InternalAgentDefinitionInvalid);
    }
    Ok(())
}

fn validate_profile(
    name: &str,
    model: &str,
    runtime: AgentRuntimeType,
) -> Result<(), AgentManagementError> {
    if name.trim().is_empty()
        || name.chars().count() > 128
        || model.trim().is_empty()
        || model.len() > 256
        || model.chars().any(char::is_control)
        || (matches!(
            runtime,
            AgentRuntimeType::OpenCodeRun | AgentRuntimeType::DshHeadless
        ) && !model
            .trim()
            .split_once('/')
            .is_some_and(|(provider, model)| {
                !provider.trim().is_empty() && !model.trim().is_empty()
            }))
    {
        return Err(AgentManagementError::InternalAgentDefinitionInvalid);
    }
    Ok(())
}
