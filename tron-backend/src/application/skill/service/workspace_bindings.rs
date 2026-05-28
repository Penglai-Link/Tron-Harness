use super::*;

impl SkillRegistryService {
    pub(super) fn active_workspace_detail(
        &self,
        workspace_code: &str,
    ) -> Result<WorkspaceDetail, SkillError> {
        let workspace = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(SkillError::InvalidWorkspace);
        }
        Ok(workspace)
    }

    pub(super) fn ensure_skill_exists(&self, skill_name: &str) -> Result<(), SkillError> {
        let exists = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .any(|skill| skill.skill_name == skill_name);
        if exists {
            Ok(())
        } else {
            Err(SkillError::NotFound)
        }
    }

    pub(super) fn resolve_workspace_bindings(
        &self,
        workspace: &WorkspaceDetail,
    ) -> Result<BTreeMap<String, ResolvedSkillBinding>, SkillError> {
        let lineage = workspace_lineage(workspace);
        let lineage_map: HashMap<_, _> = lineage
            .iter()
            .map(|entry| (entry.code.as_str(), entry.clone()))
            .collect();
        let mut bindings_by_workspace = BTreeMap::<String, Vec<WorkspaceSkillBinding>>::new();
        for binding in self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_workspace_bindings(None)?
        {
            if lineage_map.contains_key(binding.workspace_code.as_str()) {
                bindings_by_workspace
                    .entry(binding.workspace_code.clone())
                    .or_default()
                    .push(binding);
            }
        }

        let current_workspace_code = workspace.workspace_code.as_str();
        let mut resolved = BTreeMap::<String, ResolvedSkillBinding>::new();
        for entry in &lineage {
            let Some(workspace_bindings) = bindings_by_workspace.get_mut(&entry.code) else {
                continue;
            };
            workspace_bindings.sort_by(|left, right| left.skill_name.cmp(&right.skill_name));
            for binding in workspace_bindings.iter().cloned() {
                let previous = resolved.remove(&binding.skill_name);
                let source_kind = match previous {
                    Some(_) if entry.code == current_workspace_code => {
                        SkillWorkspaceSourceKind::Override
                    }
                    Some(_) => SkillWorkspaceSourceKind::Inherited,
                    None if entry.code == current_workspace_code => SkillWorkspaceSourceKind::Local,
                    None => SkillWorkspaceSourceKind::Inherited,
                };
                let (
                    shadowed_workspace_code,
                    shadowed_workspace_name,
                    shadowed_label,
                    shadowed_enabled,
                ) = if entry.code == current_workspace_code {
                    match previous.as_ref() {
                        Some(previous) => (
                            Some(previous.binding.workspace_code.clone()),
                            Some(previous.source_workspace_name.clone()),
                            Some(previous.source_label.clone()),
                            Some(previous.binding.enabled),
                        ),
                        None => (None, None, None, None),
                    }
                } else {
                    (None, None, None, None)
                };
                resolved.insert(
                    binding.skill_name.clone(),
                    ResolvedSkillBinding {
                        source_kind,
                        source_label: entry.label.clone(),
                        source_workspace_name: entry.name.clone(),
                        shadowed_label,
                        shadowed_workspace_code,
                        shadowed_workspace_name,
                        shadowed_enabled,
                        binding,
                    },
                );
            }
        }
        Ok(resolved)
    }
}
