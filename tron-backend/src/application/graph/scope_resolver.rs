use crate::{
    application::workspace::WorkspaceService,
    domain::{
        graph::{GraphScope, GraphScopeCommand, GraphScopeError, ResolvedGraphScope},
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
};

pub trait WorkspaceDetailProvider {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError>;
}

impl WorkspaceDetailProvider for WorkspaceService {
    fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        WorkspaceService::get_workspace(self, workspace_code)
    }
}

#[derive(Debug, Clone)]
pub struct ScopeResolver<W = WorkspaceService> {
    workspaces: W,
}

impl ScopeResolver<WorkspaceService> {
    pub fn new(workspaces: WorkspaceService) -> Self {
        Self { workspaces }
    }
}

impl<W> ScopeResolver<W>
where
    W: WorkspaceDetailProvider,
{
    pub fn with_workspace_provider(workspaces: W) -> Self {
        Self { workspaces }
    }

    pub fn resolve(
        &self,
        command: GraphScopeCommand,
    ) -> Result<ResolvedGraphScope, GraphScopeError> {
        let scope = GraphScope::from_command(command)?;
        let workspace = self
            .workspaces
            .get_workspace(&scope.workspace_code)
            .map_err(|_| GraphScopeError::WorkspaceResolutionFailed)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(GraphScopeError::ArchivedWorkspace);
        }

        crate::domain::graph::resolve_storage_scope(scope, &workspace.path)
    }
}
