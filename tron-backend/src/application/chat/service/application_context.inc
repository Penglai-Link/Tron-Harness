impl ChatService {
    pub fn ensure_session_directory_available(&self, session_code: &str) -> Result<(), ChatError> {
        let session = self.repository().get_session(session_code)?;
        let Some(code) = session.directory_code.as_deref().or(session.working_directory_code.as_deref()) else {
            return Ok(());
        };
        let directory = self.directories.as_ref().ok_or(ChatError::DirectoryInvalid)?
            .resolve(&session.workspace_code, code).map_err(|_| ChatError::DirectoryInvalid)?;
        if !std::path::Path::new(&directory.canonical_path).is_dir() {
            return Err(ChatError::DirectoryInvalid);
        }
        Ok(())
    }

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
        Ok(self.invocation_execution_scope(&session, None, true)?.unwrap_or_default())
    }

    pub(super) fn invocation_execution_scope(
        &self,
        session: &ChatSession,
        requested_workspace: Option<&str>,
        continuation: bool,
    ) -> Result<Option<AgentExecutionScope>, ChatError> {
        if !session.source_code.as_deref().is_some_and(|source| source.starts_with("__assistant:")) {
            return self.project_execution_scope(session);
        }
        // User submissions always capture a fresh workspace. Only continuation
        // of an existing run may recover its persisted execution context.
        let scope = if continuation {
            self.execution_memory.as_ref()
                .map(|memory| memory.context_detail(&session.session_code, "applicationContext"))
                .transpose().map_err(|_| ChatError::StorageFailure)?
                .flatten()
                .and_then(|detail| detail.get("executionScope").cloned())
                .map(serde_json::from_value::<AgentExecutionScope>)
                .transpose().map_err(|_| ChatError::StorageFailure)?
                .unwrap_or_default()
        } else {
            AgentExecutionScope {
                workspace_code: requested_workspace.map(str::trim).filter(|code| !code.is_empty()).map(str::to_owned),
                task_code: session.task_code.clone(),
                ..Default::default()
            }
        };
        Ok(Some(scope))
    }

    pub(super) fn project_execution_scope(
        &self,
        session: &ChatSession,
    ) -> Result<Option<AgentExecutionScope>, ChatError> {
        let working = session.directory_code.as_deref().or(session.working_directory_code.as_deref());
        let mut scope = if let Some(project_code) = session.project_code.as_deref() {
            let projects = self.projects.as_ref().ok_or(ChatError::ProjectInvalid)?;
            let mut scope = projects.execution_scope(&session.workspace_code, project_code, session.task_code.clone())
                .map_err(|_| ChatError::ProjectInvalid)?;
            if let Some(memory) = &self.execution_memory {
                let detail = memory.context_detail(&session.session_code, "applicationContext").map_err(|_| ChatError::StorageFailure)?;
                if let Some(previous) = detail.and_then(|detail| detail.get("executionScope").cloned())
                    .and_then(|value| serde_json::from_value::<AgentExecutionScope>(value).ok()) {
                    projects.restore_execution_directories(&mut scope, &previous);
                }
            }
            if let Some(topics) = &self.topics {
                let memo = topics.get(session.user_code.as_deref().unwrap_or(""), project_code).map_err(|_| ChatError::ProjectInvalid)?;
                for reference in memo.references {
                    if let Some(path) = reference.directory_path {
                        if !scope.allowed_local_directories.contains(&path) { scope.allowed_local_directories.push(path); }
                    }
                }
            }
            scope
        } else if working.is_some() {
            AgentExecutionScope { task_code: session.task_code.clone(), ..Default::default() }
        } else {
            return Ok(None);
        };
        if let Some(code) = working {
            let directory = self.directories.as_ref().ok_or(ChatError::DirectoryInvalid)?
                .resolve(&session.workspace_code, code).map_err(|_| ChatError::DirectoryInvalid)?;
            let path = directory.canonical_path;
            if !scope.allowed_local_directories.contains(&path) { scope.allowed_local_directories.push(path.clone()); }
            if !scope.applications.iter().any(|app| app.working_directory.as_deref() == Some(path.as_str())) {
                scope.app_code = None;
                scope.repository_url = None;
                scope.branch_name = None;
            }
            scope.local_directory = Some(path);
        }
        Ok(Some(scope))
    }
}
