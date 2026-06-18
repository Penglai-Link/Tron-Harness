use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    application::workspace::WorkspaceService,
    domain::{
        skill::{
            PreparedWorkspaceSkill, PreparedWorkspaceSkills, RegistrySkill, RegistrySkillDetail,
            RegistrySkillFile, RegistrySkillListItem, RegistrySkillListQuery,
            SkillBindingMutationResult, SkillError, SkillManagedBy, SkillRagIndexRecord,
            SkillRegistryMutationResult, SkillRegistryState, SkillRegistrySyncResult,
            SkillRegistryUninstallResult, SkillSearchCandidate, SkillSearchMode, SkillSourceType,
            SkillUpdateStatus, SkillWorkspaceSourceKind, WorkspaceSkillBinding,
            WorkspaceSkillDetail, WorkspaceSkillListItem, WorkspaceSkillListQuery,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::time::current_timestamp,
};

#[cfg(test)]
const DEFAULT_TIMESTAMP: &str = "2026-05-19T00:00:00Z";

mod lock_file;
mod parser;
mod rag_index;
mod rag_index_payload;
mod registry_files;
mod registry_paths;
mod registry_sync;
#[cfg(test)]
mod tests;
mod use_cases;
mod workspace_bindings;
mod workspace_projection;

use self::{lock_file::*, parser::*, registry_files::*, workspace_projection::*};
pub use rag_index::SkillRagIndexService;

#[derive(Debug, Clone, Default)]
pub struct InstallSkillCommand {
    pub package_ref: Option<String>,
    pub source_ref: Option<String>,
    pub skill_name: Option<String>,
    pub enable_for_workspace_code: Option<String>,
}

pub trait SkillRegistryRepository: Send {
    fn list_registry_skills(&mut self) -> Result<Vec<RegistrySkill>, SkillError>;
    fn upsert_registry_skill(&mut self, skill: &RegistrySkill) -> Result<(), SkillError>;
    fn delete_registry_skill(&mut self, skill_name: &str) -> Result<bool, SkillError>;
    fn list_workspace_bindings(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<WorkspaceSkillBinding>, SkillError>;
    fn upsert_workspace_binding(
        &mut self,
        binding: &WorkspaceSkillBinding,
    ) -> Result<(), SkillError>;
    fn delete_workspace_binding(
        &mut self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<bool, SkillError>;
    fn delete_bindings_for_skill(&mut self, skill_name: &str) -> Result<(), SkillError>;
    fn get_rag_index_state(
        &mut self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<Option<SkillRagIndexRecord>, SkillError>;
    fn list_rag_index_states(
        &mut self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<SkillRagIndexRecord>, SkillError>;
    fn upsert_rag_index_state(&mut self, state: &SkillRagIndexRecord) -> Result<(), SkillError>;
}

#[derive(Clone)]
pub struct SkillRegistryService {
    repository: Arc<Mutex<Box<dyn SkillRegistryRepository>>>,
    workspaces: WorkspaceService,
    registry_root: PathBuf,
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

#[derive(Clone)]
struct LineageEntry {
    code: String,
    label: String,
    name: String,
}

#[derive(Clone)]
struct ResolvedSkillBinding {
    source_kind: SkillWorkspaceSourceKind,
    source_label: String,
    source_workspace_name: String,
    shadowed_label: Option<String>,
    shadowed_workspace_code: Option<String>,
    shadowed_workspace_name: Option<String>,
    shadowed_enabled: Option<bool>,
    binding: WorkspaceSkillBinding,
}

impl SkillRegistryService {
    pub fn new(
        repository: impl SkillRegistryRepository + 'static,
        workspaces: WorkspaceService,
        registry_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            workspaces,
            registry_root: registry_root.into(),
        }
    }
}
