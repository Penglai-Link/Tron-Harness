use super::{KnowledgeBase, KnowledgeBaseError, KnowledgeBaseService};

impl KnowledgeBaseService {
    fn validate_grant_agent(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
    ) -> Result<(), KnowledgeBaseError> {
        self.workspace(user, workspace)?;
        let agents = self
            .agents
            .list_workspace_internal_agents(workspace)
            .map_err(|error| KnowledgeBaseError::Storage(format!("{error:?}")))?;
        if !agents.iter().any(|record| record.agent_code == agent) {
            return Err(KnowledgeBaseError::Invalid(
                "只支持当前空间的内部 Agent".into(),
            ));
        }
        Ok(())
    }

    pub fn list_agent_knowledge_libraries(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
    ) -> Result<Vec<KnowledgeBase>, KnowledgeBaseError> {
        self.validate_grant_agent(user, workspace, agent)?;
        self.list(user, workspace)
    }

    pub fn set_agent_knowledge_libraries(
        &self,
        user: &str,
        workspace: &str,
        agent: &str,
        library_ids: &[String],
    ) -> Result<(), KnowledgeBaseError> {
        self.validate_grant_agent(user, workspace, agent)?;
        self.repository
            .set_agent_libraries(user, workspace, agent, library_ids)
    }
}
