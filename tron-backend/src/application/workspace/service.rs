use std::sync::{Arc, Mutex};

use rand::RngCore;

use crate::{
    domain::{
        auth::AuthenticatedUser,
        workspace::{
            Workspace, WorkspaceDetail, WorkspaceError, WorkspaceLifecycleResult,
            WorkspaceLineageItem, WorkspaceListQuery, WorkspaceOption, WorkspaceStatus,
            WorkspaceSwitcherData, WorkspaceSwitcherPrimarySpace, WorkspaceSwitcherSecondarySpace,
            WorkspaceTreeNode,
        },
    },
    shared::{
        event_bus::{BusEventDraft, EventBusHandle},
        id::SnowflakeId,
        time::current_timestamp,
    },
};

const GENERATED_WORKSPACE_CODE_PREFIX: &str = "WS-";
const WORKSPACE_CODE_GENERATION_ATTEMPTS: usize = 16;

mod events;
mod hierarchy;
mod lifecycle_use_cases;
mod query_use_cases;
mod tree_projection;

use hierarchy::{
    build_workspace_path, descendant_codes, find_workspace, generate_workspace_code, subtree_height,
};
use tree_projection::{
    build_child_summaries, build_workspace_options, build_workspace_switcher, build_workspace_tree,
};

#[derive(Debug, Clone)]
pub struct CreateWorkspaceCommand {
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateWorkspaceCommand {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoveWorkspaceCommand {
    pub parent_workspace_code: Option<String>,
}

pub trait WorkspaceRepository: Send {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError>;
    fn next_id(&mut self) -> Result<i64, WorkspaceError>;
    fn create_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError>;
    fn update_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError>;
    fn archive_subtree(
        &mut self,
        workspace_codes: &[String],
        archived_at: &str,
    ) -> Result<usize, WorkspaceError>;
    fn restore_subtree(
        &mut self,
        workspace_codes: &[String],
        restored_at: &str,
    ) -> Result<usize, WorkspaceError>;
}

#[derive(Clone)]
pub struct WorkspaceService {
    repository: Arc<Mutex<Box<dyn WorkspaceRepository>>>,
    event_bus: EventBusHandle,
}

impl WorkspaceService {
    pub fn new(repository: impl WorkspaceRepository + 'static) -> Self {
        Self::with_event_bus(repository, EventBusHandle::disabled())
    }

    pub fn with_event_bus(
        repository: impl WorkspaceRepository + 'static,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            event_bus,
        }
    }

    fn get_workspace_record(&self, workspace_code: &str) -> Result<Workspace, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        find_workspace(&workspaces, workspace_code).cloned()
    }

    fn all_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceError> {
        self.repository
            .lock()
            .expect("workspace repository mutex poisoned")
            .list_workspaces()
    }

    pub fn workspace_is_active(&self, workspace_code: &str) -> Result<bool, WorkspaceError> {
        Ok(self.get_workspace_record(workspace_code)?.status == WorkspaceStatus::Active)
    }

    pub fn ensure_default_workspace(
        &self,
        person_ai_user_id: &str,
        person_ai_username: &str,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        if let Some(workspace) = self
            .all_workspaces()?
            .into_iter()
            .find(|workspace| workspace.status == WorkspaceStatus::Active)
        {
            return self.get_workspace(&workspace.workspace_code);
        }
        self.create_workspace(
            &AuthenticatedUser {
                avatar_url: None,
                user_code: person_ai_user_id.trim().to_owned(),
                username: person_ai_username.trim().to_owned(),
                display_name: person_ai_username.trim().to_owned(),
                email: String::new(),
            },
            CreateWorkspaceCommand {
                name: "默认工作空间".to_owned(),
                description: Some("PersonAI 档案的默认工作空间".to_owned()),
                parent_workspace_code: None,
            },
        )
    }

    pub fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?.clone();
        let path = build_workspace_path(&workspaces, &workspace.workspace_code)?;
        let children = build_child_summaries(&workspaces, &workspace.workspace_code, path.len());
        Ok(WorkspaceDetail {
            id: workspace.id.as_decimal_string(),
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            description: workspace.description.clone(),
            parent_workspace_code: workspace.parent_workspace_code.clone(),
            depth: path.len(),
            status: workspace.status,
            path,
            children,
            created_at: workspace.created_at.clone(),
            updated_at: workspace.updated_at.clone(),
            archived_at: workspace.archived_at.clone(),
        })
    }
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
