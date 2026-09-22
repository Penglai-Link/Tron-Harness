impl ChatService {
    pub fn with_execution_memory(
        mut self,
        memory: crate::application::short_term_memory::ShortTermMemoryService,
    ) -> Self {
        self.execution_memory = Some(memory);
        self
    }

    pub fn session_execution_scope(
        &self,
        session_code: &str,
    ) -> Result<AgentExecutionScope, ChatError> {
        let session = self.repository().get_session(session_code)?;
        Ok(self.project_execution_scope(&session)?.unwrap_or_default())
    }

    pub(super) fn project_execution_scope(
        &self,
        session: &ChatSession,
    ) -> Result<Option<AgentExecutionScope>, ChatError> {
        let Some(project_code) = session.project_code.as_deref() else {
            return Ok(None);
        };
        let projects = self.projects.as_ref().ok_or(ChatError::ProjectInvalid)?;
        let mut scope = projects
            .execution_scope(
                &session.workspace_code,
                project_code,
                session.task_code.clone(),
            )
            .map_err(|_| ChatError::ProjectInvalid)?;
        if let Some(memory) = &self.execution_memory {
            let detail = memory
                .context_detail(&session.session_code, "applicationContext")
                .map_err(|_| ChatError::StorageFailure)?;
            if let Some(previous) = detail
                .and_then(|detail| detail.get("executionScope").cloned())
                .and_then(|value| serde_json::from_value::<AgentExecutionScope>(value).ok())
            {
                projects.restore_execution_directories(&mut scope, &previous);
            }
        }
        Ok(Some(scope))
    }
}
