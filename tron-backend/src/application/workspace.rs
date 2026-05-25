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

    pub fn list_workspaces(
        &self,
        query: WorkspaceListQuery,
    ) -> Result<Vec<WorkspaceTreeNode>, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        Ok(build_workspace_tree(
            &workspaces,
            &direct_counts,
            &role_counts,
            &effective_counts,
            query,
        ))
    }

    pub fn list_workspace_options(
        &self,
        query: WorkspaceListQuery,
    ) -> Result<Vec<WorkspaceOption>, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        let tree = build_workspace_tree(
            &workspaces,
            &direct_counts,
            &role_counts,
            &effective_counts,
            query,
        );

        Ok(build_workspace_options(&tree))
    }

    pub fn workspace_switcher(&self) -> Result<WorkspaceSwitcherData, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        let tree = build_workspace_tree(
            &workspaces,
            &direct_counts,
            &role_counts,
            &effective_counts,
            WorkspaceListQuery {
                include_archived: None,
                search: None,
                status: Some(WorkspaceStatus::Active),
                root_only: None,
            },
        );

        Ok(build_workspace_switcher(&tree))
    }

    pub fn statistics(&self) -> Result<WorkspaceStatistics, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let mut direct_user_grants = 0;
        let mut role_grants = 0;

        for workspace in &workspaces {
            direct_user_grants += self.list_raw_user_access(&workspace.workspace_code)?.len();
            role_grants += self.list_raw_role_access(&workspace.workspace_code)?.len();
        }

        Ok(WorkspaceStatistics {
            total_workspaces: workspaces.len(),
            active_workspaces: workspaces
                .iter()
                .filter(|workspace| workspace.status == WorkspaceStatus::Active)
                .count(),
            direct_user_grants,
            role_grants,
        })
    }

    pub fn get_workspace(&self, workspace_code: &str) -> Result<WorkspaceDetail, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?.clone();
        let direct_counts = self.direct_user_counts(&workspaces)?;
        let role_counts = self.role_grant_counts(&workspaces)?;
        let effective_counts = self.effective_user_counts(&workspaces)?;
        let path = build_workspace_path(&workspaces, &workspace.workspace_code)?;
        let depth = path.len();
        let children = build_child_summaries(
            &workspaces,
            &workspace.workspace_code,
            depth,
            &direct_counts,
            &role_counts,
            &effective_counts,
        );

        Ok(WorkspaceDetail {
            id: workspace.id.as_decimal_string(),
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            description: workspace.description.clone(),
            parent_workspace_code: workspace.parent_workspace_code.clone(),
            depth,
            status: workspace.status,
            path,
            children,
            direct_user_count: *direct_counts.get(&workspace.workspace_code).unwrap_or(&0),
            role_grant_count: *role_counts.get(&workspace.workspace_code).unwrap_or(&0),
            effective_user_count: *effective_counts
                .get(&workspace.workspace_code)
                .unwrap_or(&0),
            created_at: workspace.created_at.clone(),
            updated_at: workspace.updated_at.clone(),
            archived_at: workspace.archived_at.clone(),
        })
    }

    pub fn create_workspace(
        &self,
        actor: &AuthenticatedUser,
        command: CreateWorkspaceCommand,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        let mut workspaces = self.all_workspaces()?;
        if command.name.trim().is_empty() {
            return Err(WorkspaceError::WorkspaceNameRequired);
        }

        let parent_workspace_code = command
            .parent_workspace_code
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(parent_code) = parent_workspace_code.as_deref() {
            let parent = find_workspace(&workspaces, parent_code)?;
            if parent.status == WorkspaceStatus::Archived {
                return Err(WorkspaceError::ArchivedMutationDenied);
            }
            let parent_depth = build_workspace_path(&workspaces, parent_code)?.len();
            if parent_depth >= 3 {
                return Err(WorkspaceError::InvalidHierarchy);
            }
        }
        let workspace_code = generate_workspace_code(&workspaces)?;

        let id = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.next_id()?
        };
        let workspace = Workspace {
            id: SnowflakeId::new(id),
            workspace_code: workspace_code.clone(),
            name: command.name.trim().to_string(),
            description: optional_trimmed(command.description),
            parent_workspace_code,
            status: WorkspaceStatus::Active,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            archived_at: None,
        };
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.create_workspace(&workspace)?;
        }
        workspaces.push(workspace);
        let detail = self.get_workspace(&workspace_code)?;
        self.publish_workspace_detail_event(
            "workspace.created",
            "Created workspace",
            actor,
            &detail,
        );
        Ok(detail)
    }

    pub fn update_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        command: UpdateWorkspaceCommand,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        let mut workspace = self.get_workspace_record(workspace_code)?;
        if workspace.status == WorkspaceStatus::Archived {
            return Err(WorkspaceError::ArchivedMutationDenied);
        }
        if command.name.trim().is_empty() {
            return Err(WorkspaceError::WorkspaceNameRequired);
        }
        workspace.name = command.name.trim().to_string();
        workspace.description = optional_trimmed(command.description);
        workspace.updated_at = current_timestamp();
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.update_workspace(&workspace)?;
        }
        let detail = self.get_workspace(workspace_code)?;
        self.publish_workspace_detail_event(
            "workspace.updated",
            "Updated workspace",
            actor,
            &detail,
        );
        Ok(detail)
    }

    pub fn move_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        command: MoveWorkspaceCommand,
    ) -> Result<WorkspaceDetail, WorkspaceError> {
        let mut workspaces = self.all_workspaces()?;
        let target_index = workspaces
            .iter()
            .position(|workspace| workspace.workspace_code == workspace_code)
            .ok_or(WorkspaceError::NotFound)?;
        if workspaces[target_index].status == WorkspaceStatus::Archived {
            return Err(WorkspaceError::ArchivedMutationDenied);
        }

        let new_parent = command
            .parent_workspace_code
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if new_parent.as_deref() == Some(workspace_code) {
            return Err(WorkspaceError::InvalidHierarchy);
        }

        let subtree_codes = descendant_codes(&workspaces, workspace_code);
        if let Some(parent_code) = new_parent.as_deref() {
            if subtree_codes.contains(parent_code) {
                return Err(WorkspaceError::InvalidHierarchy);
            }
            let parent = find_workspace(&workspaces, parent_code)?;
            if parent.status == WorkspaceStatus::Archived {
                return Err(WorkspaceError::ArchivedMutationDenied);
            }
            let parent_depth = build_workspace_path(&workspaces, parent_code)?.len();
            let subtree_height = subtree_height(&workspaces, workspace_code, 1);
            if parent_depth + subtree_height > 3 {
                return Err(WorkspaceError::InvalidHierarchy);
            }
        } else if subtree_height(&workspaces, workspace_code, 1) > 3 {
            return Err(WorkspaceError::InvalidHierarchy);
        }

        workspaces[target_index].parent_workspace_code = new_parent;
        workspaces[target_index].updated_at = current_timestamp();
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.update_workspace(&workspaces[target_index])?;
        }
        let detail = self.get_workspace(workspace_code)?;
        self.publish_workspace_detail_event("workspace.moved", "Moved workspace", actor, &detail);
        Ok(detail)
    }

    pub fn archive_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
    ) -> Result<WorkspaceLifecycleResult, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?;
        let subtree_codes = descendant_codes(&workspaces, workspace_code)
            .into_iter()
            .collect::<Vec<_>>();
        let affected_workspace_count = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.archive_subtree(&subtree_codes, &current_timestamp())?
        };
        Ok(WorkspaceLifecycleResult {
            workspace_code: workspace.workspace_code.clone(),
            status: WorkspaceStatus::Archived,
            archived_at: Some(current_timestamp()),
            affected_workspace_count,
        })
        .map(|result| {
            self.publish_workspace_lifecycle_event(
                "workspace.archived",
                "Archived workspace subtree",
                actor,
                &result,
            );
            result
        })
    }

    pub fn restore_workspace(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
    ) -> Result<WorkspaceLifecycleResult, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let workspace = find_workspace(&workspaces, workspace_code)?;
        if let Some(parent_code) = workspace.parent_workspace_code.as_deref() {
            let parent = find_workspace(&workspaces, parent_code)?;
            if parent.status == WorkspaceStatus::Archived {
                return Err(WorkspaceError::ArchivedMutationDenied);
            }
        }
        let subtree_codes = descendant_codes(&workspaces, workspace_code)
            .into_iter()
            .collect::<Vec<_>>();
        let affected_workspace_count = {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.restore_subtree(&subtree_codes, &current_timestamp())?
        };
        Ok(WorkspaceLifecycleResult {
            workspace_code: workspace.workspace_code.clone(),
            status: WorkspaceStatus::Active,
            archived_at: None,
            affected_workspace_count,
        })
        .map(|result| {
            self.publish_workspace_lifecycle_event(
                "workspace.restored",
                "Restored workspace subtree",
                actor,
                &result,
            );
            result
        })
    }

    pub fn list_user_access(
        &self,
        workspace_code: &str,
        query: WorkspaceAccessListQuery,
    ) -> Result<PagedWorkspaceRecords<WorkspaceUserAccessListItem>, WorkspaceError> {
        self.ensure_workspace_exists(workspace_code)?;
        let records = self
            .list_raw_user_access(workspace_code)?
            .into_iter()
            .map(|entry| {
                let user = self.active_user_by_code(&entry.user_code)?;
                Ok(WorkspaceUserAccessListItem {
                    user_code: user.user_code,
                    display_name: user.display_name,
                    email: user.email,
                    is_owner: entry.is_owner,
                    granted_at: entry.granted_at,
                })
            })
            .collect::<Result<Vec<_>, WorkspaceError>>()?;
        Ok(paginate_records(records, query))
    }

    pub fn replace_user_access(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        command: ReplaceWorkspaceUserAccessCommand,
    ) -> Result<PagedWorkspaceRecords<WorkspaceUserAccessListItem>, WorkspaceError> {
        let workspace = self.get_workspace_record(workspace_code)?;
        if workspace.status == WorkspaceStatus::Archived {
            return Err(WorkspaceError::ArchivedMutationDenied);
        }
        let mut unique = BTreeSet::new();
        let entries = command
            .users
            .into_iter()
            .map(|draft| {
                let user_code = draft.user_code.trim().to_string();
                if user_code.is_empty() || !unique.insert(user_code.clone()) {
                    return Err(WorkspaceError::DuplicateAccessGrant);
                }
                self.active_user_by_code(&user_code)?;
                Ok(WorkspaceUserAccess {
                    user_code,
                    is_owner: draft.is_owner,
                    granted_at: current_timestamp(),
                })
            })
            .collect::<Result<Vec<_>, WorkspaceError>>()?;
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.replace_user_access(workspace_code, &entries)?;
        }
        let result = self.list_user_access(
            workspace_code,
            WorkspaceAccessListQuery {
                current: Some(1),
                size: Some(100),
                search: None,
            },
        )?;
        self.publish_workspace_access_event(
            "workspace.user_access_replaced",
            "Replaced workspace user access grants",
            actor,
            workspace_code,
            result.records.len(),
        );
        Ok(result)
    }

    pub fn list_role_access(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<WorkspaceRoleAccessListItem>, WorkspaceError> {
        self.ensure_workspace_exists(workspace_code)?;
        let role_catalog = self
            .roles
            .list_roles()
            .map_err(|_| WorkspaceError::StorageFailure)?;
        let active_users = self.all_active_users()?;
        let role_grants = self.list_raw_role_access(workspace_code)?;

        role_grants
            .into_iter()
            .map(|entry| {
                let role = role_catalog
                    .iter()
                    .find(|role| role.code == entry.role_code)
                    .ok_or(WorkspaceError::InvalidAccessRole)?;
                let matched_user_count = active_users
                    .iter()
                    .filter(|user| user.role_codes.iter().any(|code| code == &entry.role_code))
                    .count();
                Ok(WorkspaceRoleAccessListItem {
                    role_code: entry.role_code,
                    role_name: role.name.clone(),
                    matched_user_count,
                    granted_at: entry.granted_at,
                })
            })
            .collect()
    }

    pub fn replace_role_access(
        &self,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        command: ReplaceWorkspaceRoleAccessCommand,
    ) -> Result<Vec<WorkspaceRoleAccessListItem>, WorkspaceError> {
        let workspace = self.get_workspace_record(workspace_code)?;
        if workspace.status == WorkspaceStatus::Archived {
            return Err(WorkspaceError::ArchivedMutationDenied);
        }
        let role_catalog = self
            .roles
            .list_roles()
            .map_err(|_| WorkspaceError::StorageFailure)?;
        let mut unique = BTreeSet::new();
        let entries = command
            .roles
            .into_iter()
            .map(|draft| {
                let role_code = draft.role_code.trim().to_string();
                validate_role_code(&role_code).map_err(|_| WorkspaceError::InvalidAccessRole)?;
                if !unique.insert(role_code.clone()) {
                    return Err(WorkspaceError::DuplicateAccessGrant);
                }
                if !role_catalog.iter().any(|role| role.code == role_code) {
                    return Err(WorkspaceError::InvalidAccessRole);
                }
                Ok(WorkspaceRoleAccess {
                    role_code,
                    granted_at: current_timestamp(),
                })
            })
            .collect::<Result<Vec<_>, WorkspaceError>>()?;
        {
            let mut repository = self
                .repository
                .lock()
                .expect("workspace repository mutex poisoned");
            repository.replace_role_access(workspace_code, &entries)?;
        }
        let result = self.list_role_access(workspace_code)?;
        self.publish_workspace_access_event(
            "workspace.role_access_replaced",
            "Replaced workspace role access grants",
            actor,
            workspace_code,
            result.len(),
        );
        Ok(result)
    }

    pub fn list_effective_users(
        &self,
        workspace_code: &str,
        query: WorkspaceAccessListQuery,
    ) -> Result<PagedWorkspaceRecords<WorkspaceEffectiveUser>, WorkspaceError> {
        self.ensure_workspace_exists(workspace_code)?;
        let active_users = self.all_active_users()?;
        let direct_users = self.list_raw_user_access(workspace_code)?;
        let role_grants = self.list_raw_role_access(workspace_code)?;

        let direct_user_map = direct_users
            .into_iter()
            .map(|entry| (entry.user_code.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        let granted_role_codes = role_grants
            .into_iter()
            .map(|entry| entry.role_code)
            .collect::<BTreeSet<_>>();

        let mut effective = active_users
            .into_iter()
            .filter_map(|user| {
                let direct = direct_user_map.get(&user.user_code);
                let matched_role_codes = user
                    .role_codes
                    .iter()
                    .filter(|code| granted_role_codes.contains(*code))
                    .cloned()
                    .collect::<Vec<_>>();
                if direct.is_none() && matched_role_codes.is_empty() {
                    return None;
                }
                let source = match (direct.is_some(), matched_role_codes.is_empty()) {
                    (true, false) => EffectiveWorkspaceAccessSource::DirectAndRole,
                    (true, true) => EffectiveWorkspaceAccessSource::Direct,
                    (false, false) => EffectiveWorkspaceAccessSource::Role,
                    (false, true) => return None,
                };
                Some(WorkspaceEffectiveUser {
                    user_code: user.user_code,
                    display_name: user.display_name,
                    email: user.email,
                    source,
                    matched_role_codes,
                    is_owner: direct.is_some_and(|entry| entry.is_owner),
                })
            })
            .collect::<Vec<_>>();
        effective.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        Ok(paginate_records(effective, query))
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

    fn direct_user_counts(
        &self,
        workspaces: &[Workspace],
    ) -> Result<BTreeMap<String, usize>, WorkspaceError> {
        workspaces
            .iter()
            .map(|workspace| {
                Ok((
                    workspace.workspace_code.clone(),
                    self.list_raw_user_access(&workspace.workspace_code)?.len(),
                ))
            })
            .collect()
    }

    fn role_grant_counts(
        &self,
        workspaces: &[Workspace],
    ) -> Result<BTreeMap<String, usize>, WorkspaceError> {
        workspaces
            .iter()
            .map(|workspace| {
                Ok((
                    workspace.workspace_code.clone(),
                    self.list_raw_role_access(&workspace.workspace_code)?.len(),
                ))
            })
            .collect()
    }

    fn effective_user_counts(
        &self,
        workspaces: &[Workspace],
    ) -> Result<BTreeMap<String, usize>, WorkspaceError> {
        let active_users = self.all_active_users()?;
        let mut counts = BTreeMap::new();
        for workspace in workspaces {
            let direct_user_codes = self
                .list_raw_user_access(&workspace.workspace_code)?
                .into_iter()
                .map(|entry| entry.user_code)
                .collect::<BTreeSet<_>>();
            let granted_roles = self
                .list_raw_role_access(&workspace.workspace_code)?
                .into_iter()
                .map(|entry| entry.role_code)
                .collect::<BTreeSet<_>>();
            let count = active_users
                .iter()
                .filter(|user| {
                    direct_user_codes.contains(&user.user_code)
                        || user
                            .role_codes
                            .iter()
                            .any(|code| granted_roles.contains(code))
                })
                .count();
            counts.insert(workspace.workspace_code.clone(), count);
        }
        Ok(counts)
    }

    fn publish_workspace_detail_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        detail: &WorkspaceDetail,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "workspace_service")
                .subject("workspace", detail.workspace_code.clone())
                .actor(actor)
                .summary(summary)
                .attr("status", format!("{:?}", detail.status))
                .attr("depth", detail.depth.to_string())
                .attr(
                    "parentWorkspaceCode",
                    detail
                        .parent_workspace_code
                        .clone()
                        .unwrap_or_else(|| "ROOT".into()),
                )
                .attr("directUserCount", detail.direct_user_count.to_string())
                .attr("roleGrantCount", detail.role_grant_count.to_string())
                .attr(
                    "effectiveUserCount",
                    detail.effective_user_count.to_string(),
                ),
        );
    }

    fn publish_workspace_lifecycle_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        result: &WorkspaceLifecycleResult,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "workspace_service")
                .subject("workspace", result.workspace_code.clone())
                .actor(actor)
                .summary(summary)
                .attr("status", format!("{:?}", result.status))
                .attr(
                    "affectedWorkspaceCount",
                    result.affected_workspace_count.to_string(),
                ),
        );
    }

    fn publish_workspace_access_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        grant_count: usize,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "workspace_service")
                .subject("workspace", workspace_code.to_string())
                .actor(actor)
                .summary(summary)
                .attr("grantCount", grant_count.to_string()),
        );
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

fn find_workspace<'a>(
    workspaces: &'a [Workspace],
    workspace_code: &str,
) -> Result<&'a Workspace, WorkspaceError> {
    workspaces
        .iter()
        .find(|workspace| workspace.workspace_code == workspace_code)
        .ok_or(WorkspaceError::NotFound)
}

fn build_workspace_path(
    workspaces: &[Workspace],
    workspace_code: &str,
) -> Result<Vec<WorkspaceLineageItem>, WorkspaceError> {
    let mut path = Vec::new();
    let mut current = Some(workspace_code.to_string());
    while let Some(code) = current {
        let workspace = find_workspace(workspaces, &code)?;
        path.push(WorkspaceLineageItem {
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            depth: 0,
        });
        current = workspace.parent_workspace_code.clone();
    }
    path.reverse();
    for (index, item) in path.iter_mut().enumerate() {
        item.depth = index + 1;
    }
    Ok(path)
}

fn generate_workspace_code(workspaces: &[Workspace]) -> Result<String, WorkspaceError> {
    let existing_codes = workspaces
        .iter()
        .map(|workspace| workspace.workspace_code.as_str())
        .collect::<BTreeSet<_>>();
    let mut rng = rand::rng();

    for _ in 0..WORKSPACE_CODE_GENERATION_ATTEMPTS {
        let candidate = format!("{GENERATED_WORKSPACE_CODE_PREFIX}{:016X}", rng.next_u64());
        if !existing_codes.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }

    Err(WorkspaceError::DuplicateWorkspaceCode)
}

fn build_workspace_tree(
    workspaces: &[Workspace],
    direct_counts: &BTreeMap<String, usize>,
    role_counts: &BTreeMap<String, usize>,
    effective_counts: &BTreeMap<String, usize>,
    query: WorkspaceListQuery,
) -> Vec<WorkspaceTreeNode> {
    let visible_status = visible_workspace_status(query.status, query.include_archived);
    let included_codes = search_included_codes(workspaces, query.search.as_deref(), visible_status);
    let mut roots = workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.is_none())
        .filter(|workspace| workspace_matches_status(workspace, visible_status))
        .filter(|workspace| {
            included_codes
                .as_ref()
                .is_none_or(|codes| codes.contains(&workspace.workspace_code))
        })
        .map(|workspace| {
            build_workspace_node(
                workspaces,
                workspace,
                1,
                visible_status,
                included_codes.as_ref(),
                direct_counts,
                role_counts,
                effective_counts,
                query.root_only.unwrap_or(false),
            )
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.name.cmp(&right.name));
    roots
}

fn build_workspace_options(tree: &[WorkspaceTreeNode]) -> Vec<WorkspaceOption> {
    let mut options = Vec::new();
    for node in tree {
        collect_workspace_options(node, &mut Vec::new(), &mut options);
    }
    options
}

fn collect_workspace_options(
    node: &WorkspaceTreeNode,
    lineage_names: &mut Vec<String>,
    acc: &mut Vec<WorkspaceOption>,
) {
    lineage_names.push(node.name.clone());
    acc.push(WorkspaceOption {
        workspace_code: node.workspace_code.clone(),
        label: lineage_names.join(" / "),
        name: node.name.clone(),
        description: node.description.clone(),
        parent_workspace_code: node.parent_workspace_code.clone(),
        depth: node.depth,
        status: node.status,
    });

    for child in &node.children {
        collect_workspace_options(child, lineage_names, acc);
    }

    lineage_names.pop();
}

fn build_workspace_switcher(tree: &[WorkspaceTreeNode]) -> WorkspaceSwitcherData {
    let primary_spaces = tree
        .iter()
        .map(|node| WorkspaceSwitcherPrimarySpace {
            workspace_code: node.workspace_code.clone(),
            name: node.name.clone(),
            description: node.description.clone(),
            status: node.status,
            child_count: node.child_count,
        })
        .collect();

    let mut secondary_spaces = Vec::new();
    for node in tree {
        collect_workspace_switcher_secondary(
            node,
            &node.workspace_code,
            &mut Vec::new(),
            &mut secondary_spaces,
        );
    }

    WorkspaceSwitcherData {
        primary_spaces,
        secondary_spaces,
    }
}

fn collect_workspace_switcher_secondary(
    node: &WorkspaceTreeNode,
    primary_workspace_code: &str,
    lineage_names: &mut Vec<String>,
    acc: &mut Vec<WorkspaceSwitcherSecondarySpace>,
) {
    lineage_names.push(node.name.clone());
    acc.push(WorkspaceSwitcherSecondarySpace {
        workspace_code: node.workspace_code.clone(),
        primary_workspace_code: primary_workspace_code.to_string(),
        label: lineage_names.join(" / "),
        name: node.name.clone(),
        description: node.description.clone(),
        status: node.status,
        depth: node.depth,
        child_count: node.child_count,
        effective_user_count: node.effective_user_count,
    });

    for child in &node.children {
        collect_workspace_switcher_secondary(child, primary_workspace_code, lineage_names, acc);
    }

    lineage_names.pop();
}

fn build_workspace_node(
    workspaces: &[Workspace],
    workspace: &Workspace,
    depth: usize,
    visible_status: Option<WorkspaceStatus>,
    included_codes: Option<&BTreeSet<String>>,
    direct_counts: &BTreeMap<String, usize>,
    role_counts: &BTreeMap<String, usize>,
    effective_counts: &BTreeMap<String, usize>,
    root_only: bool,
) -> WorkspaceTreeNode {
    let mut children = if root_only {
        Vec::new()
    } else {
        workspaces
            .iter()
            .filter(|candidate| {
                candidate.parent_workspace_code.as_deref() == Some(&workspace.workspace_code)
            })
            .filter(|candidate| workspace_matches_status(candidate, visible_status))
            .filter(|candidate| {
                included_codes.is_none_or(|codes| codes.contains(&candidate.workspace_code))
            })
            .map(|candidate| {
                build_workspace_node(
                    workspaces,
                    candidate,
                    depth + 1,
                    visible_status,
                    included_codes,
                    direct_counts,
                    role_counts,
                    effective_counts,
                    false,
                )
            })
            .collect::<Vec<_>>()
    };
    children.sort_by(|left, right| left.name.cmp(&right.name));
    WorkspaceTreeNode {
        id: workspace.id.as_decimal_string(),
        workspace_code: workspace.workspace_code.clone(),
        name: workspace.name.clone(),
        description: workspace.description.clone(),
        parent_workspace_code: workspace.parent_workspace_code.clone(),
        depth,
        status: workspace.status,
        child_count: children.len(),
        direct_user_count: *direct_counts.get(&workspace.workspace_code).unwrap_or(&0),
        role_grant_count: *role_counts.get(&workspace.workspace_code).unwrap_or(&0),
        effective_user_count: *effective_counts
            .get(&workspace.workspace_code)
            .unwrap_or(&0),
        children,
    }
}

fn build_child_summaries(
    workspaces: &[Workspace],
    workspace_code: &str,
    parent_depth: usize,
    direct_counts: &BTreeMap<String, usize>,
    role_counts: &BTreeMap<String, usize>,
    effective_counts: &BTreeMap<String, usize>,
) -> Vec<WorkspaceTreeNode> {
    let mut children = workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.as_deref() == Some(workspace_code))
        .map(|workspace| WorkspaceTreeNode {
            id: workspace.id.as_decimal_string(),
            workspace_code: workspace.workspace_code.clone(),
            name: workspace.name.clone(),
            description: workspace.description.clone(),
            parent_workspace_code: workspace.parent_workspace_code.clone(),
            depth: parent_depth + 1,
            status: workspace.status,
            child_count: workspaces
                .iter()
                .filter(|candidate| {
                    candidate.parent_workspace_code.as_deref() == Some(&workspace.workspace_code)
                })
                .count(),
            direct_user_count: *direct_counts.get(&workspace.workspace_code).unwrap_or(&0),
            role_grant_count: *role_counts.get(&workspace.workspace_code).unwrap_or(&0),
            effective_user_count: *effective_counts
                .get(&workspace.workspace_code)
                .unwrap_or(&0),
            children: Vec::new(),
        })
        .collect::<Vec<_>>();
    children.sort_by(|left, right| left.name.cmp(&right.name));
    children
}

fn search_included_codes(
    workspaces: &[Workspace],
    search: Option<&str>,
    visible_status: Option<WorkspaceStatus>,
) -> Option<BTreeSet<String>> {
    let search = search.map(str::trim).filter(|value| !value.is_empty())?;
    let search = search.to_ascii_lowercase();
    let mut included = workspaces
        .iter()
        .filter(|workspace| workspace_matches_status(workspace, visible_status))
        .filter(|workspace| {
            workspace
                .workspace_code
                .to_ascii_lowercase()
                .contains(&search)
                || workspace.name.to_ascii_lowercase().contains(&search)
                || workspace
                    .description
                    .as_deref()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains(&search)
        })
        .map(|workspace| workspace.workspace_code.clone())
        .collect::<BTreeSet<_>>();
    if included.is_empty() {
        return Some(included);
    }
    let matched_codes = included.iter().cloned().collect::<Vec<_>>();
    for code in matched_codes {
        let mut current = find_workspace(workspaces, &code)
            .ok()
            .and_then(|workspace| workspace.parent_workspace_code.clone());
        while let Some(parent_code) = current {
            let Some(parent) = workspaces.iter().find(|workspace| {
                workspace.workspace_code == parent_code
                    && workspace_matches_status(workspace, visible_status)
            }) else {
                break;
            };
            included.insert(parent.workspace_code.clone());
            current = parent.parent_workspace_code.clone();
        }
    }
    Some(included)
}

fn visible_workspace_status(
    status: Option<WorkspaceStatus>,
    include_archived: Option<bool>,
) -> Option<WorkspaceStatus> {
    match (status, include_archived.unwrap_or(false)) {
        (None, true) => None,
        (Some(status), _) => Some(status),
        (None, false) => Some(WorkspaceStatus::Active),
    }
}

fn workspace_matches_status(
    workspace: &Workspace,
    visible_status: Option<WorkspaceStatus>,
) -> bool {
    visible_status.is_none_or(|status| workspace.status == status)
}

fn descendant_codes(workspaces: &[Workspace], workspace_code: &str) -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    collect_descendants(workspaces, workspace_code, &mut codes);
    codes
}

fn collect_descendants(workspaces: &[Workspace], workspace_code: &str, acc: &mut BTreeSet<String>) {
    if !acc.insert(workspace_code.to_string()) {
        return;
    }
    for child in workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.as_deref() == Some(workspace_code))
    {
        collect_descendants(workspaces, &child.workspace_code, acc);
    }
}

fn subtree_height(workspaces: &[Workspace], workspace_code: &str, depth: usize) -> usize {
    let max_child_depth = workspaces
        .iter()
        .filter(|workspace| workspace.parent_workspace_code.as_deref() == Some(workspace_code))
        .map(|workspace| subtree_height(workspaces, &workspace.workspace_code, depth + 1))
        .max()
        .unwrap_or(depth);
    max_child_depth
}

fn paginate_records<T: Clone + SearchableRecord>(
    records: Vec<T>,
    query: WorkspaceAccessListQuery,
) -> PagedWorkspaceRecords<T> {
    let current = query.current.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);
    let search = query.search.map(|value| value.to_ascii_lowercase());
    let mut filtered = records
        .into_iter()
        .filter(|record| {
            search
                .as_ref()
                .is_none_or(|search| record.matches_search(search))
        })
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    let total = filtered.len();
    let start = ((current - 1) * size).min(total);
    let end = (start + size).min(total);

    PagedWorkspaceRecords {
        current,
        size,
        total,
        records: filtered[start..end].to_vec(),
    }
}

trait SearchableRecord {
    fn matches_search(&self, search: &str) -> bool;
    fn sort_key(&self) -> String;
}

impl SearchableRecord for WorkspaceUserAccessListItem {
    fn matches_search(&self, search: &str) -> bool {
        self.user_code.to_ascii_lowercase().contains(search)
            || self.display_name.to_ascii_lowercase().contains(search)
            || self.email.to_ascii_lowercase().contains(search)
    }

    fn sort_key(&self) -> String {
        self.display_name.clone()
    }
}

impl SearchableRecord for WorkspaceEffectiveUser {
    fn matches_search(&self, search: &str) -> bool {
        self.user_code.to_ascii_lowercase().contains(search)
            || self.display_name.to_ascii_lowercase().contains(search)
            || self.email.to_ascii_lowercase().contains(search)
            || self
                .matched_role_codes
                .iter()
                .any(|code| code.to_ascii_lowercase().contains(search))
    }

    fn sort_key(&self) -> String {
        self.display_name.clone()
    }
}
