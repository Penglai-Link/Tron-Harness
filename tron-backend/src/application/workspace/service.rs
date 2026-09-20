use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use rand::RngCore;

use crate::{
    application::{role::RoleService, user::UserService},
    domain::{
        auth::AuthenticatedUser,
        role::validate_role_code,
        user::{ListUsersQuery, User, UserError, UserStatus},
        workspace::{
            EffectiveWorkspaceAccessSource, PagedWorkspaceRecords, Workspace,
            WorkspaceAccessListQuery, WorkspaceDetail, WorkspaceEffectiveUser, WorkspaceError,
            WorkspaceLifecycleResult, WorkspaceLineageItem, WorkspaceListQuery, WorkspaceOption,
            WorkspaceRoleAccess, WorkspaceRoleAccessListItem, WorkspaceStatistics, WorkspaceStatus,
            WorkspaceSwitcherData, WorkspaceSwitcherPrimarySpace, WorkspaceSwitcherSecondarySpace,
            WorkspaceTreeNode, WorkspaceUserAccess, WorkspaceUserAccessListItem,
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

mod access_use_cases;
mod events;
mod hierarchy;
mod lifecycle_use_cases;
mod metrics;
mod pagination;
mod query_use_cases;
mod tree_projection;

use hierarchy::{
    build_workspace_path, descendant_codes, find_workspace, generate_workspace_code, subtree_height,
};
use pagination::paginate_records;
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

#[derive(Debug, Clone)]
pub struct ReplaceWorkspaceUserAccessCommand {
    pub users: Vec<WorkspaceUserAccessDraft>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceUserAccessDraft {
    pub user_code: String,
    pub is_owner: bool,
}

#[derive(Debug, Clone)]
pub struct ReplaceWorkspaceRoleAccessCommand {
    pub roles: Vec<WorkspaceRoleAccessDraft>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceRoleAccessDraft {
    pub role_code: String,
}

pub trait WorkspaceRepository: Send {
    fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError>;
    fn next_id(&mut self) -> Result<i64, WorkspaceError>;
    fn create_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError>;
    fn update_workspace(&mut self, workspace: &Workspace) -> Result<(), WorkspaceError>;
    fn list_user_access(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError>;
    fn replace_user_access(
        &mut self,
        workspace_code: &str,
        entries: &[WorkspaceUserAccess],
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError>;
    fn list_role_access(
        &mut self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError>;
    fn replace_role_access(
        &mut self,
        workspace_code: &str,
        entries: &[WorkspaceRoleAccess],
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError>;
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
    users: UserService,
    roles: RoleService,
    event_bus: EventBusHandle,
}

impl WorkspaceService {
    pub fn new(
        repository: impl WorkspaceRepository + 'static,
        users: UserService,
        roles: RoleService,
    ) -> Self {
        Self::with_event_bus(repository, users, roles, EventBusHandle::disabled())
    }

    pub fn with_event_bus(
        repository: impl WorkspaceRepository + 'static,
        users: UserService,
        roles: RoleService,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            users,
            roles,
            event_bus,
        }
    }

    fn ensure_workspace_exists(&self, workspace_code: &str) -> Result<(), WorkspaceError> {
        self.get_workspace_record(workspace_code).map(|_| ())
    }

    fn get_workspace_record(&self, workspace_code: &str) -> Result<Workspace, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        find_workspace(&workspaces, workspace_code).cloned()
    }

    fn all_workspaces(&self) -> Result<Vec<Workspace>, WorkspaceError> {
        let mut repository = self
            .repository
            .lock()
            .expect("workspace repository mutex poisoned");
        repository.list_workspaces()
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
                role_codes: Vec::new(),
                space_codes: Vec::new(),
                status: UserStatus::Active,
                password_reset_required: false,
            },
            CreateWorkspaceCommand {
                name: "默认工作空间".to_owned(),
                description: Some("PersonAI 档案的默认工作空间".to_owned()),
                parent_workspace_code: None,
            },
        )
    }

    fn list_raw_user_access(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
        let mut repository = self
            .repository
            .lock()
            .expect("workspace repository mutex poisoned");
        repository.list_user_access(workspace_code)
    }

    fn list_raw_role_access(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
        let mut repository = self
            .repository
            .lock()
            .expect("workspace repository mutex poisoned");
        repository.list_role_access(workspace_code)
    }

    fn all_active_users(&self) -> Result<Vec<User>, WorkspaceError> {
        let mut current = 1;
        let mut users = Vec::new();
        loop {
            let page = self
                .users
                .list_users(ListUsersQuery {
                    page: None,
                    current: Some(current),
                    size: Some(100),
                    status: Some(UserStatus::Active),
                    search: None,
                    role_code: None,
                    space_code: None,
                    include_archived: Some(false),
                })
                .map_err(map_user_error)?;
            users.extend(page.records);
            if users.len() >= page.total {
                break;
            }
            current += 1;
        }
        Ok(users)
    }

    fn active_user_by_code(&self, user_code: &str) -> Result<User, WorkspaceError> {
        let user = self.users.get_user(user_code).map_err(map_user_error)?;
        if user.status != UserStatus::Active {
            return Err(WorkspaceError::InvalidAccessUser);
        }
        Ok(user)
    }
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn map_user_error(error: UserError) -> WorkspaceError {
    match error {
        UserError::NotFound | UserError::InvalidSpaceCode => WorkspaceError::InvalidAccessUser,
        _ => WorkspaceError::StorageFailure,
    }
}
