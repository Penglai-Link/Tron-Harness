use super::*;

impl SkillRegistryService {
    pub fn bind_capabilities(
        &self,
        resources: crate::application::capability::CapabilityResources,
    ) {
        let _ = self.capabilities.set(resources);
    }
    pub(super) fn skill_available(&self, name: &str) -> bool {
        self.capabilities
            .get()
            .is_none_or(|r| r.available("skill", name))
    }

    pub fn bind_agents(&self, agents: crate::application::agent::AgentManagementService) {
        let _ = self.agents.set(agents);
    }
    pub fn internal_agent_codes(&self) -> Result<Vec<String>, SkillError> {
        self.agents
            .get()
            .ok_or(SkillError::StorageFailure)?
            .platform_internal_agent_codes()
            .map_err(|_| SkillError::StorageFailure)
    }
    pub(super) fn initialize_assignments(&self) -> Result<(), SkillError> {
        if self.agents.get().is_none() {
            return Ok(());
        }
        let agents = self.internal_agent_codes()?;
        let mut repo = self.repository.lock().unwrap();
        repo.initialize_agents_skills(&agents)
    }
    fn validate_agent(&self, agent: &str) -> Result<(), SkillError> {
        if self.internal_agent_codes()?.iter().any(|a| a == agent) {
            Ok(())
        } else {
            Err(SkillError::InvalidAgent)
        }
    }
    pub fn agent_skills(&self, agent: &str) -> Result<Vec<RegistrySkillListItem>, SkillError> {
        self.validate_agent(agent)?;
        self.registry_items(RegistrySkillListQuery { search: None }, Some(agent))
    }

    /// Platform Agents and Dudes receive grants. Legacy workspace-local Agents keep
    /// running with an empty skill set; management APIs still reject unknown IDs.
    pub fn effective_agent_skills(
        &self,
        agent: &str,
    ) -> Result<Vec<RegistrySkillListItem>, SkillError> {
        match self.agent_skills(agent) {
            Ok(items) => Ok(items
                .into_iter()
                .filter(|s| s.assigned && s.available)
                .collect()),
            Err(SkillError::InvalidAgent) => Ok(vec![]),
            Err(error) => Err(error),
        }
    }
    pub fn replace_agent_skills(&self, agent: &str, names: &[String]) -> Result<(), SkillError> {
        let _guard = self.capabilities.get().map(|r| r.assignment_guard());
        self.validate_agent(agent)?;
        self.initialize_assignments()?;
        let mut repo = self.repository.lock().unwrap();
        let skills = repo.list_registry_skills()?;
        for name in names {
            if skills.iter().any(|skill| {
                skill.skill_name == *name
                    && !crate::domain::computer_use::skill_allowed(
                        agent,
                        name,
                        skill.provider_pack_id.as_deref(),
                    )
            }) {
                return Err(SkillError::Unavailable);
            }
            if !skills
                .iter()
                .any(|s| &s.skill_name == name && s.available && self.skill_available(name))
            {
                return Err(SkillError::Unavailable);
            }
        }
        repo.replace_agent_skills(agent, names)
    }
    pub fn skill_agents(&self, name: &str) -> Result<Vec<String>, SkillError> {
        self.initialize_assignments()?;
        let mut repo = self.repository.lock().unwrap();
        if !repo
            .list_registry_skills()?
            .iter()
            .any(|s| s.skill_name == name)
        {
            return Err(SkillError::NotFound);
        }
        Ok(repo
            .list_assignments()?
            .into_iter()
            .filter(|a| a.skill_name == name && a.enabled)
            .filter(|a| crate::domain::computer_use::skill_allowed(&a.agent_code, name, None))
            .map(|a| a.agent_code)
            .collect())
    }
    pub fn replace_skill_agents(&self, name: &str, agents: &[String]) -> Result<(), SkillError> {
        let _guard = self.capabilities.get().map(|r| r.assignment_guard());
        let valid = self
            .internal_agent_codes()?
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        if agents.iter().any(|agent| !valid.contains(agent)) {
            return Err(SkillError::InvalidAgent);
        }
        self.initialize_assignments()?;
        let mut repo = self.repository.lock().unwrap();
        let skill = repo
            .list_registry_skills()?
            .into_iter()
            .find(|s| s.skill_name == name)
            .ok_or(SkillError::NotFound)?;
        if agents.iter().any(|agent| {
            !crate::domain::computer_use::skill_allowed(
                agent,
                name,
                skill.provider_pack_id.as_deref(),
            )
        }) {
            return Err(SkillError::Unavailable);
        }
        if (!skill.available || !self.skill_available(name)) && !agents.is_empty() {
            return Err(SkillError::Unavailable);
        }
        repo.replace_skill_agents(name, agents)
    }
    pub fn register_pack_skill(
        &self,
        pack: &str,
        name: &str,
        path: &Path,
        enabled: bool,
    ) -> Result<(), SkillError> {
        let mut repo = self.repository.lock().unwrap();
        if repo
            .list_registry_skills()?
            .iter()
            .any(|s| s.skill_name == name && s.provider_pack_id.as_deref() != Some(pack))
        {
            return Err(SkillError::DuplicateSkill);
        }
        let now = current_timestamp();
        repo.upsert_registry_skill(&RegistrySkill {
            skill_name: name.into(),
            provider_pack_id: Some(pack.into()),
            available: enabled,
            source_ref: pack.into(),
            source_type: SkillSourceType::BuiltIn,
            registry_path: path.display().to_string(),
            declared_version: None,
            update_status: SkillUpdateStatus::Manual,
            managed_by: SkillManagedBy::BuiltIn,
            last_checked_at: None,
            last_updated_at: None,
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn delete_pack_skills(&self, pack: &str) -> Result<Vec<String>, SkillError> {
        self.repository.lock().unwrap().delete_pack_skills(pack)
    }
}
