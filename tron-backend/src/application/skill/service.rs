use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{domain::skill::*, shared::time::current_timestamp};

mod assignments;
mod lock_file;
mod parser;
mod provider;
mod rag_index;
mod rag_index_payload;
mod registry_files;
mod registry_paths;
mod registry_sync;
#[cfg(test)]
mod tests;
mod use_cases;
mod approved_artifact;
pub use provider::SkillAgentIdentity;
mod projection;

use self::{lock_file::*, parser::*, projection::*, registry_files::*};
pub use rag_index::SkillRagIndexService;

#[derive(Debug, Clone, Default)]
pub struct InstallSkillCommand {
    pub package_ref: Option<String>,
    pub source_ref: Option<String>,
    pub skill_name: Option<String>,
}

pub trait SkillRegistryRepository: Send {
    fn list_registry_skills(&mut self) -> Result<Vec<RegistrySkill>, SkillError>;
    fn upsert_registry_skill(&mut self, skill: &RegistrySkill) -> Result<(), SkillError>;
    fn delete_registry_skill(&mut self, skill_name: &str) -> Result<bool, SkillError>;
    fn list_assignments(&mut self) -> Result<Vec<AgentSkillAssignment>, SkillError>;
    fn replace_agent_skills(&mut self, agent: &str, names: &[String]) -> Result<(), SkillError>;
    fn replace_skill_agents(&mut self, name: &str, agents: &[String]) -> Result<(), SkillError>;
    fn initialize_agent_skills(&mut self, agent: &str) -> Result<(), SkillError>;
    fn delete_pack_skills(&mut self, pack: &str) -> Result<Vec<String>, SkillError>;
    fn get_rag_index_state(
        &mut self,
        skill_name: &str,
    ) -> Result<Option<SkillRagIndexRecord>, SkillError>;
    fn list_rag_index_states(&mut self) -> Result<Vec<SkillRagIndexRecord>, SkillError>;
    fn delete_rag_index_state(&mut self, name: &str) -> Result<(), SkillError>;
    fn upsert_rag_index_state(&mut self, state: &SkillRagIndexRecord) -> Result<(), SkillError>;
}

#[derive(Clone)]
pub struct SkillRegistryService {
    repository: Arc<Mutex<Box<dyn SkillRegistryRepository>>>,
    agents: Arc<std::sync::OnceLock<crate::application::agent::AgentManagementService>>,
    capabilities: Arc<std::sync::OnceLock<crate::application::capability::CapabilityResources>>,
    index_lock: Arc<tokio::sync::Mutex<()>>,
    registry_root: PathBuf,
    mutation_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SkillsLockFile {
    #[serde(default = "default_lock_version")]
    version: u8,
    #[serde(default)]
    skills: BTreeMap<String, SkillsLockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillsLockEntry {
    source: String,
    source_type: Option<String>,
}

#[derive(Debug, Clone)]
struct SearchLine {
    package_ref: String,
    installs_label: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SkillFrontmatter {
    declared_version: Option<String>,
    description: Option<String>,
}

impl SkillRegistryService {
    pub fn new(
        repository: impl SkillRegistryRepository + 'static,
        registry_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            agents: Arc::new(std::sync::OnceLock::new()),
            capabilities: Default::default(),
            index_lock: Default::default(),
            registry_root: registry_root.into(),
            mutation_lock: Default::default(),
        }
    }
}
