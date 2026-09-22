use crate::domain::{
    agent::{AgentExecutionScope, ApplicationDirectoryContext, ApplicationDirectoryStatus},
    project::ProjectError,
};

use super::ProjectService;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInstructionFile {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

/// Read-only local directory inspection; implementations must never change HEAD or cwd.
pub trait ApplicationDirectoryInspector: Send + Sync {
    fn inspect(&self, directory: &str) -> ApplicationDirectoryContext;
    fn instruction_files(&self, _directory: &str) -> Result<Vec<ProjectInstructionFile>, String> {
        Ok(Vec::new())
    }
    fn list_worktrees(&self, _directory: &str) -> Vec<crate::domain::agent::GitWorktreeContext> {
        Vec::new()
    }
}

impl ProjectService {
    pub fn with_directory_inspector(
        mut self,
        inspector: std::sync::Arc<dyn ApplicationDirectoryInspector>,
    ) -> Self {
        self.directory_inspector = Some(inspector);
        self
    }

    pub fn execution_scope(
        &self,
        workspace_code: &str,
        project_code: &str,
        task_code: Option<String>,
    ) -> Result<AgentExecutionScope, ProjectError> {
        let project = self.resolve_execution_context(workspace_code, project_code)?;
        let applications = project
            .linked_apps
            .iter()
            .map(|app| {
                let mut directory = self
                    .directory_inspector
                    .as_ref()
                    .map(|inspector| inspector.inspect(&app.local_directory))
                    .unwrap_or(ApplicationDirectoryContext {
                        app_code: String::new(),
                        app_name: String::new(),
                        is_primary: false,
                        configured_directory: app.local_directory.clone(),
                        working_directory: None,
                        repository_root: None,
                        branch_name: None,
                        head_commit: None,
                        worktree_path: None,
                        is_worktree: false,
                        status: ApplicationDirectoryStatus::Unavailable,
                    });
                directory.app_code = app.app_code.clone();
                directory.app_name = app.name.clone();
                directory.is_primary = project
                    .primary_app
                    .as_ref()
                    .is_some_and(|primary| primary.app_code == app.app_code);
                directory
            })
            .collect::<Vec<_>>();
        let primary = applications.iter().find(|app| app.is_primary);
        Ok(AgentExecutionScope {
            project_code: Some(project.project_code),
            app_code: project.primary_app.as_ref().map(|app| app.app_code.clone()),
            repository_url: project
                .primary_app
                .as_ref()
                .and_then(|app| app.repository_url.clone()),
            // Preserve an unavailable configured path so tools fail there instead of silently
            // falling back to the backend process directory.
            local_directory: primary.map(|app| {
                app.working_directory
                    .clone()
                    .unwrap_or_else(|| app.configured_directory.clone())
            }),
            branch_name: primary.and_then(|app| app.branch_name.clone()),
            task_code,
            allowed_local_directories: applications
                .iter()
                .map(|app| {
                    app.working_directory
                        .clone()
                        .unwrap_or_else(|| app.configured_directory.clone())
                })
                .collect(),
            applications,
        })
    }
}

/// Only select directories belonging to a worktree of the configured repository.
pub fn inspect_current_application(
    inspector: &dyn ApplicationDirectoryInspector,
    app: &ApplicationDirectoryContext,
    requested_path: Option<&str>,
) -> ApplicationDirectoryContext {
    let selected = requested_path
        .or(app.working_directory.as_deref())
        .unwrap_or(&app.configured_directory);
    let path = std::path::Path::new(selected);
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let configured = std::path::Path::new(&app.configured_directory)
        .canonicalize()
        .unwrap_or_else(|_| app.configured_directory.clone().into());
    let worktrees = inspector.list_worktrees(&app.configured_directory);
    let registered = worktrees
        .iter()
        .filter(|tree| canonical.starts_with(&tree.path))
        .max_by_key(|tree| tree.path.len());
    let mut fresh = if let Some(tree) = registered {
        inspector.inspect(&tree.path)
    } else if canonical == configured {
        inspector.inspect(&app.configured_directory)
    } else {
        let mut unavailable = inspector.inspect(selected);
        if unavailable.status != ApplicationDirectoryStatus::MissingDirectory {
            unavailable.status = ApplicationDirectoryStatus::Unavailable;
        }
        unavailable.branch_name = None;
        unavailable.head_commit = None;
        unavailable
    };
    // Keep the selected path even when it disappeared; falling back would run tools
    // in the configured checkout without an explicit selection.
    if fresh.working_directory.is_none() {
        fresh.working_directory = Some(
            registered
                .map(|tree| tree.path.clone())
                .unwrap_or_else(|| selected.into()),
        );
    }
    fresh.app_code = app.app_code.clone();
    fresh.app_name = app.app_name.clone();
    fresh.is_primary = app.is_primary;
    fresh.configured_directory = app.configured_directory.clone();
    fresh
}

impl ProjectService {
    pub fn restore_execution_directories(
        &self,
        scope: &mut AgentExecutionScope,
        previous: &AgentExecutionScope,
    ) {
        let Some(inspector) = self.directory_inspector.as_ref() else {
            return;
        };
        for app in &mut scope.applications {
            if let Some(saved) = previous.applications.iter().find(|saved| {
                saved.app_code == app.app_code
                    && saved.configured_directory == app.configured_directory
            }) {
                *app = inspect_current_application(
                    inspector.as_ref(),
                    app,
                    saved.working_directory.as_deref(),
                );
            }
        }
        if let Some(primary) = scope.applications.iter().find(|app| app.is_primary) {
            scope.local_directory = Some(
                primary
                    .working_directory
                    .clone()
                    .unwrap_or_else(|| primary.configured_directory.clone()),
            );
            scope.branch_name = primary.branch_name.clone();
            scope.allowed_local_directories.extend(
                scope
                    .applications
                    .iter()
                    .filter_map(|app| app.working_directory.clone()),
            );
        }
    }
}
