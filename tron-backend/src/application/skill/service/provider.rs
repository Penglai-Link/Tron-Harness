use super::*;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_skills::{FileSystemSkillProvider, FileSystemSkillsConfig};
#[derive(Clone)]
pub struct SkillAgentIdentity(pub String);
struct RegistryProvider(SkillRegistryService);
impl SkillRegistryService {
    pub fn provider(&self) -> impl fw::SkillProvider + use<> {
        RegistryProvider(self.clone())
    }
    pub fn register_pack_directories(
        &self,
        pack: &str,
        roots: &[PathBuf],
        enabled: bool,
    ) -> Result<(), SkillError> {
        for (name, path) in self.validate_pack_directories(pack, roots)? {
            self.register_pack_skill(pack, &name, &path, enabled)?;
        }
        Ok(())
    }
    /// Read and validate the complete contribution before changing the registry.
    pub fn validate_pack_directories(
        &self,
        pack: &str,
        roots: &[PathBuf],
    ) -> Result<BTreeMap<String, PathBuf>, SkillError> {
        fn collect(root: &Path, skills: &mut BTreeMap<String, PathBuf>) -> Result<(), SkillError> {
            if !root.exists() {
                return Ok(());
            }
            if root.join("SKILL.md").is_file() {
                let name = parse_skill_name(&read_skill_markdown(root)?)?;
                if skills.insert(name, root.to_path_buf()).is_some() {
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
        let existing = self.repository.lock().unwrap().list_registry_skills()?;
        if existing.iter().any(|skill| {
            skills.contains_key(&skill.skill_name)
                && skill.provider_pack_id.as_deref() != Some(pack)
        }) {
            return Err(SkillError::DuplicateSkill);
        }
        Ok(skills)
    }
}
impl RegistryProvider {
    fn filesystem(&self) -> FileSystemSkillProvider {
        FileSystemSkillProvider::new(
            FileSystemSkillsConfig::new()
                .without_local_directory()
                .without_system_directory()
                .with_system_directory(self.0.skills_dir()),
        )
    }
    fn allowed(&self, run: &fw::RunContext, name: &str) -> Result<(), fw::AgentError> {
        let agent = run
            .extensions
            .get::<SkillAgentIdentity>()
            .ok_or_else(|| fw::AgentError::Run("missing skill Agent identity".into()))?;
        if self
            .0
            .effective_agent_skills(&agent.0)
            .map_err(|e| fw::AgentError::Run(e.to_string()))?
            .iter()
            .any(|s| s.skill_name == name && s.assigned && s.available)
        {
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
        let mut catalog = self.filesystem().catalog(run).await?;
        catalog
            .skills
            .retain(|s| self.allowed(run, &s.name).is_ok());
        Ok(catalog)
    }
    async fn load(
        &self,
        run: &fw::RunContext,
        name: &str,
    ) -> Result<fw::SkillDetail, fw::AgentError> {
        self.allowed(run, name)?;
        self.filesystem().load(run, name).await
    }
    async fn read_resource(
        &self,
        run: &fw::RunContext,
        name: &str,
        path: &str,
    ) -> Result<fw::SkillResource, fw::AgentError> {
        self.allowed(run, name)?;
        self.filesystem().read_resource(run, name, path).await
    }
}
