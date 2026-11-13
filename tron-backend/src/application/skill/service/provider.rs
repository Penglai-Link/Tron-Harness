use super::*;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_skills::{FileSystemSkillProvider, FileSystemSkillsConfig};
#[derive(Clone)]
pub struct SkillAgentIdentity(pub String);
struct RegistryProvider {
    registry: SkillRegistryService,
    filesystem: FileSystemSkillProvider,
}
impl SkillRegistryService {
    pub fn provider(&self) -> impl fw::SkillProvider + use<> {
        RegistryProvider {
            registry: self.clone(),
            filesystem: FileSystemSkillProvider::new(
                FileSystemSkillsConfig::new()
                    .without_local_directory()
                    .without_system_directory()
                    .with_system_directory(self.skills_dir()),
            ),
        }
    }
    /// Project the skills currently shipped by each capability pack into the registry.
    ///
    /// The pack is authoritative: rows whose skill no longer exists in that pack
    /// are removed, while surviving rows are refreshed in place so Agent
    /// assignments are preserved.
    pub fn reconcile_pack_directories(
        &self,
        packs: &[(String, Vec<PathBuf>, bool)],
    ) -> Result<Vec<String>, SkillError> {
        let enabled = packs
            .iter()
            .map(|(pack, _, active)| (pack.as_str(), *active))
            .collect::<BTreeMap<_, _>>();
        let mut desired = BTreeMap::<String, (String, PathBuf, Option<String>)>::new();
        for (pack, roots, _) in packs {
            for (name, (path, version)) in self.scan_pack_directories(roots)? {
                if desired
                    .insert(name, (pack.clone(), path, version))
                    .is_some()
                {
                    return Err(SkillError::DuplicateSkill);
                }
            }
        }
        let now = current_timestamp();
        let mut repository = self.repository.lock().unwrap();
        let existing = repository.list_registry_skills()?;
        if existing.iter().any(|skill| {
            skill.provider_pack_id.is_none() && desired.contains_key(&skill.skill_name)
        }) {
            return Err(SkillError::DuplicateSkill);
        }
        let mut removed = Vec::new();
        for skill in existing
            .iter()
            .filter(|skill| skill.provider_pack_id.is_some())
        {
            if !desired.contains_key(&skill.skill_name)
                && repository.delete_registry_skill(&skill.skill_name)?
            {
                removed.push(skill.skill_name.clone());
            }
        }
        let entries = desired
            .iter()
            .map(|(name, (pack, path, version))| RegistrySkill {
                skill_name: name.clone(),
                provider_pack_id: Some(pack.clone()),
                available: enabled.get(pack.as_str()).copied().unwrap_or(true),
                source_ref: pack.clone(),
                source_type: SkillSourceType::BuiltIn,
                registry_path: path.display().to_string(),
                declared_version: version.clone(),
                update_status: SkillUpdateStatus::Manual,
                managed_by: SkillManagedBy::BuiltIn,
                last_checked_at: None,
                last_updated_at: None,
                last_error: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .collect::<Vec<_>>();
        repository.upsert_registry_skills(&entries)?;
        let controllers = if enabled
            .iter()
            .any(|(pack, active)| *active && crate::domain::computer_use::is_pack(pack))
        {
            crate::domain::computer_use::CONTROLLERS
                .iter()
                .map(|agent| (*agent).to_owned())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        // A newly installed/upgraded desktop package receives the requested
        // primary-controller defaults once. Later scans preserve deselections.
        for entry in &entries {
            let changed = existing
                .iter()
                .find(|old| old.skill_name == entry.skill_name)
                .is_none_or(|old| old.registry_path != entry.registry_path);
            if !controllers.is_empty() && changed {
                repository.replace_skill_agents(&entry.skill_name, &controllers)?;
            }
        }
        Ok(removed)
    }
    /// Read and validate the complete contribution before changing the registry.
    pub fn validate_pack_directories(
        &self,
        pack: &str,
        roots: &[PathBuf],
    ) -> Result<BTreeMap<String, PathBuf>, SkillError> {
        let skills = self.scan_pack_directories(roots)?;
        let existing = self.repository.lock().unwrap().list_registry_skills()?;
        if existing.iter().any(|skill| {
            skills.contains_key(&skill.skill_name)
                && skill.provider_pack_id.as_deref() != Some(pack)
        }) {
            return Err(SkillError::DuplicateSkill);
        }
        Ok(skills
            .into_iter()
            .map(|(name, (path, _))| (name, path))
            .collect())
    }

    fn scan_pack_directories(
        &self,
        roots: &[PathBuf],
    ) -> Result<BTreeMap<String, (PathBuf, Option<String>)>, SkillError> {
        fn collect(
            root: &Path,
            skills: &mut BTreeMap<String, (PathBuf, Option<String>)>,
        ) -> Result<(), SkillError> {
            if !root.exists() {
                return Ok(());
            }
            if root.join("SKILL.md").is_file() {
                let markdown = read_skill_markdown(root)?;
                let name = parse_skill_name(&markdown)?;
                let version = parse_skill_frontmatter(&markdown).declared_version;
                if skills.insert(name, (root.to_path_buf(), version)).is_some() {
                    return Err(SkillError::DuplicateSkill);
                }
            } else {
                for entry in fs::read_dir(root).map_err(|_| SkillError::StorageFailure)? {
                    let entry = entry.map_err(|_| SkillError::StorageFailure)?;
                    if entry
                        .file_type()
                        .map_err(|_| SkillError::StorageFailure)?
                        .is_dir()
                    {
                        collect(&entry.path(), skills)?;
                    }
                }
            }
            Ok(())
        }
        let mut skills = BTreeMap::new();
        for root in roots {
            collect(root, &mut skills)?;
        }
        Ok(skills)
    }
}
impl RegistryProvider {
    fn allowed_names(&self, run: &fw::RunContext) -> Result<BTreeSet<String>, fw::AgentError> {
        let agent = run
            .extensions
            .get::<SkillAgentIdentity>()
            .ok_or_else(|| fw::AgentError::Run("missing skill Agent identity".into()))?;
        self.registry
            .effective_agent_skill_names(&agent.0)
            .map_err(|e| fw::AgentError::Run(e.to_string()))
    }
    fn allowed(&self, run: &fw::RunContext, name: &str) -> Result<(), fw::AgentError> {
        if self.allowed_names(run)?.contains(name) {
            Ok(())
        } else {
            Err(fw::AgentError::Run(
                "skill is not assigned to this Agent".into(),
            ))
        }
    }
}
#[async_trait::async_trait]
impl fw::SkillProvider for RegistryProvider {
    async fn catalog(&self, run: &fw::RunContext) -> Result<fw::SkillCatalog, fw::AgentError> {
        let mut catalog = self.filesystem.catalog(run).await?;
        let allowed = self.allowed_names(run)?;
        catalog.skills.retain(|s| allowed.contains(&s.name));
        Ok(catalog)
    }
    async fn load(
        &self,
        run: &fw::RunContext,
        name: &str,
    ) -> Result<fw::SkillDetail, fw::AgentError> {
        self.allowed(run, name)?;
        let detail = self.filesystem.load(run, name).await?;
        // File I/O may yield while grants or package availability change.
        self.allowed(run, name)?;
        Ok(detail)
    }
    async fn read_resource(
        &self,
        run: &fw::RunContext,
        name: &str,
        path: &str,
    ) -> Result<fw::SkillResource, fw::AgentError> {
        self.allowed(run, name)?;
        let resource = self.filesystem.read_resource(run, name, path).await?;
        self.allowed(run, name)?;
        Ok(resource)
    }
    fn release(&self, run: &fw::RunContext) {
        self.filesystem.release(run);
    }
}
