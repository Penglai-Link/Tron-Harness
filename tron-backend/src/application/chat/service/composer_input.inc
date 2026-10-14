impl ChatService {
    pub fn with_skill_registry(
        mut self,
        skills: crate::application::skill::SkillRegistryService,
    ) -> Self {
        self.skills = Some(skills);
        self
    }

    pub fn validate_composer_input(
        &self,
        agent: &str,
        content: &str,
        names: &[String],
    ) -> Result<(), ChatError> {
        self.authorize_requested_skills(agent, names)?;
        if super::composer_input::goal_objective(content)?.is_some() {
            self.ensure_goal_agent(agent)?;
        }
        Ok(())
    }

    pub(super) fn authorize_requested_skills(
        &self,
        agent: &str,
        names: &[String],
    ) -> Result<Vec<String>, ChatError> {
        let selected = eliteforge_agent_core::RequestedSkills::default();
        selected
            .extend(names)
            .map_err(|_| ChatError::RequestedSkillsInvalid)?;
        let names = selected.names();
        if names.is_empty() {
            return Ok(names);
        }
        let skills = self
            .skills
            .as_ref()
            .ok_or(ChatError::RequestedSkillForbidden)?;
        let allowed = skills
            .effective_agent_skills(agent)
            .map_err(|_| ChatError::StorageFailure)?;
        if names
            .iter()
            .any(|name| !allowed.iter().any(|skill| skill.skill_name == *name))
        {
            return Err(ChatError::RequestedSkillForbidden);
        }
        Ok(names)
    }

    pub(super) fn ensure_goal_agent(&self, agent: &str) -> Result<(), ChatError> {
        let skills = self
            .skills
            .as_ref()
            .ok_or(ChatError::GoalCommandUnavailable)?;
        if self.execution_memory.is_none()
            || !skills
                .internal_agent_codes()
                .map_err(|_| ChatError::StorageFailure)?
                .iter()
                .any(|code| code == agent)
        {
            return Err(ChatError::GoalCommandUnavailable);
        }
        Ok(())
    }

    pub(super) fn apply_goal_command(
        &self,
        session: &str,
        objective: &str,
        user: Option<String>,
    ) -> Result<(), ChatError> {
        use crate::application::short_term_memory::{
            GoalSessionCommand, SetGoalCommand, UpdateGoalCommand,
        };
        let memory = self
            .execution_memory
            .as_ref()
            .ok_or(ChatError::GoalCommandUnavailable)?;
        let latest = memory
            .get_latest_goal(session)
            .map_err(super::composer_input::goal_error)?;
        if let Some(view) = latest
            .filter(|view| matches!(view.goal.status.as_str(), "ACTIVE" | "PAUSED" | "BLOCKED"))
        {
            let goal_code = view.goal.goal_code;
            memory
                .update_goal(UpdateGoalCommand {
                    session_code: session.into(),
                    goal_code: goal_code.clone(),
                    objective: objective.into(),
                    max_runs: None,
                })
                .map_err(super::composer_input::goal_error)?;
            if view.goal.status != "ACTIVE" {
                memory
                    .resume_goal(GoalSessionCommand {
                        session_code: session.into(),
                        goal_code: Some(goal_code),
                    })
                    .map_err(super::composer_input::goal_error)?;
            }
        } else {
            memory
                .set_goal_with_operator(
                    SetGoalCommand {
                        session_code: session.into(),
                        objective: objective.into(),
                        max_runs: None,
                    },
                    user,
                )
                .map_err(super::composer_input::goal_error)?;
        }
        Ok(())
    }
}
