use super::*;

impl WorkspaceService {
    pub fn user_has_effective_access(
        &self,
        workspace_code: &str,
        user_code: &str,
    ) -> Result<bool, WorkspaceError> {
        let workspaces = self.all_workspaces()?;
        let user = self.active_user_by_code(user_code)?;
        let workspace_path = build_workspace_path(&workspaces, workspace_code)?;

        // Access grants are inherited down the workspace tree. A user may be
        // granted directly, or through one of their roles, on any ancestor of
        // the requested workspace. This keeps child-workspace resource APIs
        // consistent with the workspace switcher and inherited resources.
        for workspace in workspace_path {
            let direct = self
                .list_raw_user_access(&workspace.workspace_code)?
                .iter()
                .any(|entry| entry.user_code == user.user_code);
            if direct {
                return Ok(true);
            }

            let granted_roles = self
                .list_raw_role_access(&workspace.workspace_code)?
                .into_iter()
                .map(|entry| entry.role_code)
                .collect::<BTreeSet<_>>();
            if user
                .role_codes
                .iter()
                .any(|role_code| granted_roles.contains(role_code))
            {
                return Ok(true);
            }
        }

        Ok(false)
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
}
