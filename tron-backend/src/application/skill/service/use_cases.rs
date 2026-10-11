use super::*;

impl SkillRegistryService {
    pub fn list_registry(
        &self,
        query: RegistrySkillListQuery,
    ) -> Result<Vec<RegistrySkillListItem>, SkillError> {
        self.initialize_assignments()?;
        let search = normalized_search(query.search);
        let mut repo = self.repository.lock().unwrap();
        let assignments = repo.list_assignments()?;
        Ok(repo
            .list_registry_skills()?
            .into_iter()
            .filter(|s| matches_skill_search(s, search.as_deref()))
            .map(|mut s| {
                s.available &= self.skill_available(&s.skill_name);
                let count = assignments
                    .iter()
                    .filter(|a| a.skill_name == s.skill_name && a.enabled && s.available)
                    .count();
                let description = read_skill_markdown(Path::new(&s.registry_path))
                    .ok()
                    .and_then(|md| parse_skill_frontmatter(&md).description);
                RegistrySkillListItem {
                    disabled_reason: disabled_reason(&s),
                    assigned_agent_count: count,
                    assigned: false,
                    skill_name: s.skill_name,
                    provider_pack_id: s.provider_pack_id,
                    available: s.available,
                    source_ref: s.source_ref,
                    description,
                    source_type: s.source_type,
                    registry_path: s.registry_path,
                    declared_version: s.declared_version,
                    update_status: s.update_status,
                    managed_by: s.managed_by,
                    last_checked_at: s.last_checked_at,
                    last_updated_at: s.last_updated_at,
                    last_error: s.last_error,
                }
            })
            .collect())
    }

    pub fn get_registry_skill(&self, name: &str) -> Result<RegistrySkillDetail, SkillError> {
        let mut s = self
            .repository
            .lock()
            .unwrap()
            .list_registry_skills()?
            .into_iter()
            .find(|s| s.skill_name == name)
            .ok_or(SkillError::NotFound)?;
        s.available &= self.skill_available(&s.skill_name);
        let skill_markdown = read_skill_markdown(Path::new(&s.registry_path))?;
        let files = list_skill_files(Path::new(&s.registry_path))?;
        let count = self
            .repository
            .lock()
            .unwrap()
            .list_assignments()?
            .iter()
            .filter(|a| a.skill_name == name && a.enabled && s.available)
            .count();
        Ok(RegistrySkillDetail {
            disabled_reason: disabled_reason(&s),
            assigned_agent_count: count,
            skill_name: s.skill_name,
            provider_pack_id: s.provider_pack_id,
            available: s.available,
            source_ref: s.source_ref,
            source_type: s.source_type,
            registry_path: s.registry_path,
            declared_version: s.declared_version,
            update_status: s.update_status,
            managed_by: s.managed_by,
            last_checked_at: s.last_checked_at,
            last_updated_at: s.last_updated_at,
            last_error: s.last_error,
            skill_markdown,
            files,
        })
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
        if self
            .repository
            .lock()
            .unwrap()
            .list_registry_skills()?
            .iter()
            .any(|s| s.skill_name == skill_name)
        {
            return Err(SkillError::DuplicateSkill);
        }

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

        Ok(SkillRegistryMutationResult {
            skill_name,
            rag_index: None,
        })
    }

    pub fn rescan_registry(&self) -> Result<SkillRegistrySyncResult, SkillError> {
        self.sync_registry(false, false)
    }

    pub fn uninstall_skill(
        &self,
        skill_name: &str,
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

        let removed_assignments_count = self
            .repository
            .lock()
            .unwrap()
            .list_assignments()?
            .iter()
            .filter(|a| a.skill_name == skill_name)
            .count();

        if registry_path.exists() {
            fs::remove_dir_all(&registry_path).map_err(|_| SkillError::StorageFailure)?;
        }
        remove_skill_lock_entry(&self.registry_lock_path(), &skill_name)?;

        self.repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .delete_registry_skill(&skill_name)?;

        Ok(SkillRegistryUninstallResult {
            skill_name,
            removed_assignments_count,
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
}

fn disabled_reason(s: &RegistrySkill) -> Option<String> {
    (!s.available).then(|| {
        format!(
            "能力包 {} 已禁用",
            s.provider_pack_id.as_deref().unwrap_or("")
        )
    })
}
