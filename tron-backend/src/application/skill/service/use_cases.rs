use super::*;

impl SkillRegistryService {
    pub fn list_registry(
        &self,
        query: RegistrySkillListQuery,
    ) -> Result<Vec<RegistrySkillListItem>, SkillError> {
        let workspace_code = optional_trimmed(query.workspace_code);
        let resolved_bindings = if let Some(code) = workspace_code.as_deref() {
            let workspace = self.active_workspace_detail(code)?;
            self.resolve_workspace_bindings(&workspace)?
        } else {
            BTreeMap::new()
        };

        let search = normalized_search(query.search);
        let skills = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?;
        let bindings = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_workspace_bindings(None)?;
        let enabled_counts = binding_counts(&bindings);

        let mut items = skills
            .into_iter()
            .filter(|skill| matches_skill_search(skill, search.as_deref()))
            .map(|skill| {
                let resolved = resolved_bindings.get(skill.skill_name.as_str());
                let description = read_skill_markdown(Path::new(&skill.registry_path))
                    .ok()
                    .and_then(|markdown| parse_skill_frontmatter(&markdown).description);
                RegistrySkillListItem {
                    enabled: resolved.is_some_and(|binding| binding.binding.enabled),
                    enabled_workspace_count: enabled_counts
                        .get(skill.skill_name.as_str())
                        .copied()
                        .unwrap_or(0),
                    skill_name: skill.skill_name,
                    source_ref: skill.source_ref,
                    description,
                    source_type: skill.source_type,
                    registry_path: skill.registry_path,
                    declared_version: skill.declared_version,
                    update_status: skill.update_status,
                    managed_by: skill.managed_by,
                    last_checked_at: skill.last_checked_at,
                    last_updated_at: skill.last_updated_at,
                    last_error: skill.last_error,
                    source_kind: resolved
                        .map(|binding| binding.source_kind)
                        .unwrap_or(SkillWorkspaceSourceKind::Registry),
                    source_workspace_code: resolved
                        .map(|binding| binding.binding.workspace_code.clone()),
                    source_workspace_name: resolved
                        .map(|binding| binding.source_workspace_name.clone()),
                    source_label: resolved.map(|binding| binding.source_label.clone()),
                    shadowed_workspace_code: resolved
                        .and_then(|binding| binding.shadowed_workspace_code.clone()),
                    shadowed_workspace_name: resolved
                        .and_then(|binding| binding.shadowed_workspace_name.clone()),
                    shadowed_label: resolved.and_then(|binding| binding.shadowed_label.clone()),
                    shadowed_enabled: resolved.and_then(|binding| binding.shadowed_enabled),
                }
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.skill_name.cmp(&right.skill_name));
        Ok(items)
    }

    pub fn get_registry_skill(
        &self,
        skill_name: &str,
        workspace_code: Option<&str>,
    ) -> Result<RegistrySkillDetail, SkillError> {
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        let skill = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .find(|record| record.skill_name == skill_name)
            .ok_or(SkillError::NotFound)?;
        let registry_path = PathBuf::from(&skill.registry_path);
        let skill_markdown = read_skill_markdown(&registry_path)?;
        let files = list_skill_files(&registry_path)?;
        let enabled_workspace_count = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_workspace_bindings(None)?
            .into_iter()
            .filter(|binding| binding.skill_name == skill.skill_name && binding.enabled)
            .count();
        let resolved = if let Some(workspace_code) = workspace_code.and_then(normalized_required) {
            let workspace = self.active_workspace_detail(&workspace_code)?;
            self.resolve_workspace_bindings(&workspace)?
                .remove(skill.skill_name.as_str())
        } else {
            None
        };
        Ok(RegistrySkillDetail {
            skill_name: skill.skill_name,
            source_ref: skill.source_ref,
            source_type: skill.source_type,
            registry_path: skill.registry_path,
            declared_version: skill.declared_version,
            update_status: skill.update_status,
            managed_by: skill.managed_by,
            last_checked_at: skill.last_checked_at,
            last_updated_at: skill.last_updated_at,
            last_error: skill.last_error,
            skill_markdown,
            files,
            enabled: resolved
                .as_ref()
                .is_some_and(|binding| binding.binding.enabled),
            source_kind: resolved
                .as_ref()
                .map(|binding| binding.source_kind)
                .unwrap_or(SkillWorkspaceSourceKind::Registry),
            source_workspace_code: resolved
                .as_ref()
                .map(|binding| binding.binding.workspace_code.clone()),
            source_workspace_name: resolved
                .as_ref()
                .map(|binding| binding.source_workspace_name.clone()),
            source_label: resolved
                .as_ref()
                .map(|binding| binding.source_label.clone()),
            shadowed_workspace_code: resolved
                .as_ref()
                .and_then(|binding| binding.shadowed_workspace_code.clone()),
            shadowed_workspace_name: resolved
                .as_ref()
                .and_then(|binding| binding.shadowed_workspace_name.clone()),
            shadowed_label: resolved
                .as_ref()
                .and_then(|binding| binding.shadowed_label.clone()),
            shadowed_enabled: resolved
                .as_ref()
                .and_then(|binding| binding.shadowed_enabled),
            enabled_workspace_count,
        })
    }

    pub fn list_workspace_skills(
        &self,
        workspace_code: &str,
        query: WorkspaceSkillListQuery,
    ) -> Result<Vec<WorkspaceSkillListItem>, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let workspace = self.active_workspace_detail(&workspace_code)?;
        let resolved_bindings = self.resolve_workspace_bindings(&workspace)?;
        let search = normalized_search(query.search);
        let skill_map = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .map(|skill| (skill.skill_name.clone(), skill))
            .collect::<HashMap<_, _>>();
        let rag_index_states = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_rag_index_states(Some(&workspace_code))?
            .into_iter()
            .map(|state| (state.skill_name.clone(), state))
            .collect::<HashMap<_, _>>();

        let mut items = resolved_bindings
            .into_iter()
            .filter(|(skill_name, _)| matches_workspace_skill_search(skill_name, search.as_deref()))
            .map(|(skill_name, resolved)| {
                build_workspace_skill_list_item(
                    skill_map.get(skill_name.as_str()),
                    &resolved,
                    rag_index_states.get(skill_name.as_str()),
                )
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.skill_name.cmp(&right.skill_name));
        Ok(items)
    }

    pub fn get_workspace_skill(
        &self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<WorkspaceSkillDetail, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        let workspace = self.active_workspace_detail(&workspace_code)?;
        let resolved = self
            .resolve_workspace_bindings(&workspace)?
            .remove(skill_name.as_str())
            .ok_or(SkillError::NotFound)?;

        let skill = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .find(|record| record.skill_name == skill_name);

        if let Some(skill) = skill {
            let registry_path = PathBuf::from(&skill.registry_path);
            let skill_markdown = read_skill_markdown(&registry_path)?;
            let files = list_skill_files(&registry_path)?;
            let rag_index = self
                .repository
                .lock()
                .expect("skill registry repository mutex poisoned")
                .get_rag_index_state(&workspace_code, &skill_name)?;
            Ok(build_workspace_skill_detail(
                Some(&skill),
                &resolved,
                Some(skill_markdown),
                files,
                rag_index.as_ref(),
            ))
        } else {
            let rag_index = self
                .repository
                .lock()
                .expect("skill registry repository mutex poisoned")
                .get_rag_index_state(&workspace_code, &skill_name)?;
            Ok(build_workspace_skill_detail(
                None,
                &resolved,
                None,
                Vec::new(),
                rag_index.as_ref(),
            ))
        }
    }

    pub fn search_marketplace(
        &self,
        query: &str,
        mode: SkillSearchMode,
    ) -> Result<Vec<SkillSearchCandidate>, SkillError> {
        let query = normalized_required(query).ok_or(SkillError::SearchQueryRequired)?;
        self.ensure_registry_root()?;
        let output = match mode {
            SkillSearchMode::Find => {
                self.run_skills_command(["find", query.as_str()], self.registry_root())?
            }
            SkillSearchMode::Repo => {
                self.run_skills_command(["add", query.as_str(), "-l"], self.registry_root())?
            }
        };
        if !output.status.success() {
            return Err(SkillError::CommandFailed);
        }
        Ok(match mode {
            SkillSearchMode::Find => parse_search_output(&output.stdout),
            SkillSearchMode::Repo => parse_repo_listing_output(&output.stdout, &query),
        })
    }

    pub fn install_skill(
        &self,
        command: InstallSkillCommand,
    ) -> Result<SkillRegistryMutationResult, SkillError> {
        self.ensure_registry_root()?;
        let package_ref = optional_trimmed(command.package_ref)
            .or_else(|| build_package_ref(command.source_ref.clone(), command.skill_name.clone()))
            .ok_or(SkillError::SourceRefRequired)?;
        let (_, skill_name) = split_package_ref(&package_ref)?;

        let output = self.run_skills_command(
            [
                "add",
                package_ref.as_str(),
                "--copy",
                "-a",
                "universal",
                "-y",
            ],
            self.registry_root(),
        )?;
        if !output.status.success() {
            return Err(SkillError::CommandFailed);
        }

        self.sync_registry(false, false)?;

        if let Some(workspace_code) = optional_trimmed(command.enable_for_workspace_code) {
            self.bind_workspace_skill(&workspace_code, &skill_name, true)?;
            return Ok(SkillRegistryMutationResult {
                skill_name,
                workspace_code: Some(workspace_code),
                enabled: true,
                rag_index: None,
            });
        }

        Ok(SkillRegistryMutationResult {
            skill_name,
            workspace_code: None,
            enabled: false,
            rag_index: None,
        })
    }

    pub fn rescan_registry(&self) -> Result<SkillRegistrySyncResult, SkillError> {
        self.sync_registry(false, false)
    }

    pub fn uninstall_skill(
        &self,
        skill_name: &str,
        remove_bindings: bool,
    ) -> Result<SkillRegistryUninstallResult, SkillError> {
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        let registry_skill = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .find(|skill| skill.skill_name == skill_name)
            .ok_or(SkillError::NotFound)?;

        let registry_path = PathBuf::from(&registry_skill.registry_path);
        if !registry_path.starts_with(self.skills_dir()) {
            return Err(SkillError::InvalidRegistryPath);
        }

        let bindings = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_workspace_bindings(None)?;
        let kept_bindings_count = bindings
            .iter()
            .filter(|binding| binding.skill_name == skill_name)
            .count();

        if registry_path.exists() {
            fs::remove_dir_all(&registry_path).map_err(|_| SkillError::StorageFailure)?;
        }
        remove_skill_lock_entry(&self.registry_lock_path(), &skill_name)?;

        self.repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .delete_registry_skill(&skill_name)?;

        let removed_bindings_count = if remove_bindings {
            self.repository
                .lock()
                .expect("skill registry repository mutex poisoned")
                .delete_bindings_for_skill(&skill_name)?;
            kept_bindings_count
        } else {
            0
        };

        Ok(SkillRegistryUninstallResult {
            skill_name,
            removed_bindings_count,
            kept_bindings_count: if remove_bindings {
                0
            } else {
                kept_bindings_count
            },
            rag_index: None,
        })
    }

    pub fn check_updates(&self) -> Result<SkillRegistrySyncResult, SkillError> {
        self.ensure_registry_root()?;
        let output = self.run_skills_command(["ls", "--json"], self.registry_root())?;
        if !output.status.success() {
            return Err(SkillError::CommandFailed);
        }
        self.sync_registry(true, false)
    }

    pub fn update_registry(&self) -> Result<SkillRegistrySyncResult, SkillError> {
        self.ensure_registry_root()?;
        let output = self.run_skills_command(["update", "-p", "-y"], self.registry_root())?;
        if !output.status.success() {
            return Err(SkillError::CommandFailed);
        }
        self.sync_registry(true, true)
    }

    pub fn refresh_workspace_skills(
        &self,
        workspace_code: &str,
    ) -> Result<SkillRegistrySyncResult, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        self.active_workspace_detail(&workspace_code)?;
        self.sync_registry(false, false)
    }

    pub fn bind_workspace_skill(
        &self,
        workspace_code: &str,
        skill_name: &str,
        enabled: bool,
    ) -> Result<SkillBindingMutationResult, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        self.active_workspace_detail(&workspace_code)?;
        self.ensure_skill_exists(&skill_name)?;
        let binding = WorkspaceSkillBinding {
            workspace_code: workspace_code.clone(),
            skill_name: skill_name.clone(),
            enabled,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };
        self.repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .upsert_workspace_binding(&binding)?;
        Ok(SkillBindingMutationResult {
            workspace_code,
            skill_name,
            enabled,
            rag_index: None,
        })
    }

    pub fn unbind_workspace_skill(
        &self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<SkillBindingMutationResult, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        self.active_workspace_detail(&workspace_code)?;
        self.repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .delete_workspace_binding(&workspace_code, &skill_name)?;
        Ok(SkillBindingMutationResult {
            workspace_code,
            skill_name,
            enabled: false,
            rag_index: None,
        })
    }

    pub fn prepare_workspace_skills(
        &self,
        workspace_code: &str,
        worktree_path: &Path,
    ) -> Result<PreparedWorkspaceSkills, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let workspace = self.active_workspace_detail(&workspace_code)?;
        let bindings = self.resolve_workspace_bindings(&workspace)?;
        let skill_map = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .map(|skill| (skill.skill_name.clone(), skill))
            .collect::<HashMap<_, _>>();

        let destination_root = worktree_path.join(".agents").join("skills");
        if destination_root.exists() {
            fs::remove_dir_all(&destination_root).map_err(|_| SkillError::StorageFailure)?;
        }
        fs::create_dir_all(&destination_root).map_err(|_| SkillError::StorageFailure)?;

        let mut prepared = Vec::new();
        for binding in bindings
            .into_values()
            .filter(|binding| binding.binding.enabled)
        {
            let Some(skill) = skill_map.get(&binding.binding.skill_name) else {
                continue;
            };
            let source_path = PathBuf::from(&skill.registry_path);
            let target_path = destination_root.join(&binding.binding.skill_name);
            copy_dir_all(&source_path, &target_path)?;
            prepared.push(PreparedWorkspaceSkill {
                skill_name: binding.binding.skill_name,
                source_path: source_path.display().to_string(),
                target_path: target_path.display().to_string(),
            });
        }

        let manifest_path = destination_root.join(".tron-injected.json");
        let manifest = PreparedWorkspaceSkills {
            workspace_code,
            worktree_path: worktree_path.display().to_string(),
            skills: prepared,
        };
        let manifest_body =
            serde_json::to_vec_pretty(&manifest).map_err(|_| SkillError::StorageFailure)?;
        fs::write(&manifest_path, manifest_body).map_err(|_| SkillError::StorageFailure)?;
        Ok(manifest)
    }
}
