use super::*;

impl WorkspaceService {
    pub(super) fn direct_user_counts(
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

    pub(super) fn role_grant_counts(
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

    pub(super) fn effective_user_counts(
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
}
