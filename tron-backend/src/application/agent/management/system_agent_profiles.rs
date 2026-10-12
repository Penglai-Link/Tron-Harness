use super::*;

impl AgentManagementService {
    /// A logical system Agent shares an installed CLI but pins its own model.
    pub fn create_system_agent_profile(
        &self,
        code: &str,
        name: &str,
        base: &str,
        model: &str,
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
        entry.base_agent_code = Some(entry.agent_code.clone());
        entry.agent_code = code;
        entry.name = name.trim().into();
        entry.model_id = Some(model.trim().into());
        entry.enabled = true;
        self.repository().insert_system_agent(&entry)?;
        self.profile_record(&entry)
    }

    pub fn update_system_agent_profile(
        &self,
        code: &str,
        name: &str,
        model: &str,
    ) -> Result<ManagedSystemAgentRecord, AgentManagementError> {
        let mut entry = self.required_system_agent(code)?;
        if entry.base_agent_code.is_none() {
            return Err(AgentManagementError::BuiltinAgentReadOnly);
        }
        validate_profile(name, model, entry.runtime_type)?;
        entry.name = name.trim().into();
        entry.model_id = Some(model.trim().into());
        self.repository().upsert_system_agent(&entry)?;
        self.profile_record(&entry)
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
            agent_code: entry.agent_code,
            system_model_id: entry.model_id,
            runtime_type: entry.runtime_type,
            command_name: Some(entry.command_name),
            name: entry.name,
        })
    }
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
        || (runtime == AgentRuntimeType::OpenCodeRun
            && !model
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

#[cfg(test)]
#[path = "system_agent_profile_tests.rs"]
mod tests;
