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
            SkillBindingMutationResult, SkillError, SkillManagedBy, SkillRegistryMutationResult,
            SkillRegistryState, SkillRegistrySyncResult, SkillRegistryUninstallResult,
            SkillSearchCandidate, SkillSearchMode, SkillSourceType, SkillUpdateStatus,
            SkillWorkspaceSourceKind, WorkspaceSkillBinding, WorkspaceSkillDetail,
            WorkspaceSkillListItem, WorkspaceSkillListQuery,
        },
        workspace::{WorkspaceDetail, WorkspaceError, WorkspaceStatus},
    },
    shared::time::current_timestamp,
};

#[cfg(test)]
const DEFAULT_TIMESTAMP: &str = "2026-05-19T00:00:00Z";

mod lock_file;
mod parser;
mod registry_files;
#[cfg(test)]
mod tests;
mod use_cases;
mod workspace_projection;

use self::{lock_file::*, parser::*, registry_files::*, workspace_projection::*};

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

    fn sync_registry(
        &self,
        checked: bool,
        updated: bool,
    ) -> Result<SkillRegistrySyncResult, SkillError> {
        self.ensure_registry_root()?;
        let existing = self
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .map(|skill| (skill.skill_name.clone(), skill))
            .collect::<HashMap<_, _>>();
        let lock_entries = read_skills_lock(&self.registry_lock_path())?;
        let skill_dirs = list_skill_directories(&self.skills_dir())?;
        let mut seen = BTreeSet::new();

        for skill_dir in &skill_dirs {
            let Some(skill_name) = skill_dir.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            seen.insert(skill_name.to_owned());
            let lock_entry = lock_entries.get(skill_name);
            let existing_skill = existing.get(skill_name);
            let skill_markdown = read_skill_markdown(skill_dir)?;
            let frontmatter = parse_skill_frontmatter(&skill_markdown);
            let managed_by = if lock_entry.is_some() {
                SkillManagedBy::Cli
            } else {
                SkillManagedBy::Manual
            };
            let source_ref = lock_entry
                .map(|entry| entry.source.clone())
                .or_else(|| existing_skill.map(|skill| skill.source_ref.clone()))
                .unwrap_or_else(|| skill_dir.display().to_string());
            let source_type = lock_entry
                .and_then(|entry| entry.source_type.as_deref())
                .and_then(parse_source_type)
                .unwrap_or(if managed_by == SkillManagedBy::Manual {
                    SkillSourceType::Manual
                } else {
                    SkillSourceType::GitHub
                });
            let update_status = if managed_by == SkillManagedBy::Manual {
                SkillUpdateStatus::Manual
            } else if updated {
                SkillUpdateStatus::UpToDate
            } else {
                existing_skill
                    .map(|skill| skill.update_status)
                    .unwrap_or(SkillUpdateStatus::UpToDate)
            };
            let skill = RegistrySkill {
                skill_name: skill_name.to_owned(),
                source_ref,
                source_type,
                registry_path: skill_dir.display().to_string(),
                declared_version: frontmatter.declared_version,
                update_status,
                managed_by,
                last_checked_at: if checked || updated {
                    Some(current_timestamp())
                } else {
                    existing_skill.and_then(|skill| skill.last_checked_at.clone())
                },
                last_updated_at: if updated {
                    Some(current_timestamp())
                } else {
                    existing_skill.and_then(|skill| skill.last_updated_at.clone())
                },
                last_error: None,
                created_at: existing_skill
                    .map(|skill| skill.created_at.clone())
                    .unwrap_or_else(current_timestamp),
                updated_at: current_timestamp(),
            };
            self.repository
                .lock()
                .expect("skill registry repository mutex poisoned")
                .upsert_registry_skill(&skill)?;
        }

        let mut removed_count = 0;
        for skill_name in existing.keys() {
            if seen.contains(skill_name) {
                continue;
            }
            removed_count += usize::from(
                self.repository
                    .lock()
                    .expect("skill registry repository mutex poisoned")
                    .delete_registry_skill(skill_name)?,
            );
        }

        Ok(SkillRegistrySyncResult {
            registry_root: self.registry_root.display().to_string(),
            discovered_count: seen.len(),
            removed_count,
            checked,
            updated,
        })
    }

    fn ensure_registry_root(&self) -> Result<(), SkillError> {
        fs::create_dir_all(self.skills_dir()).map_err(|_| SkillError::StorageFailure)
    }

    fn active_workspace_detail(&self, workspace_code: &str) -> Result<WorkspaceDetail, SkillError> {
        let workspace = self
            .workspaces
            .get_workspace(workspace_code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(SkillError::InvalidWorkspace);
        }
        Ok(workspace)
    }

    fn ensure_skill_exists(&self, skill_name: &str) -> Result<(), SkillError> {
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

    fn resolve_workspace_bindings(
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

    fn registry_root(&self) -> &Path {
        &self.registry_root
    }

    fn skills_dir(&self) -> PathBuf {
        self.registry_root.join(".agents").join("skills")
    }

    fn registry_lock_path(&self) -> PathBuf {
        self.registry_root.join("skills-lock.json")
    }

    fn run_skills_command<'a, I>(
        &self,
        args: I,
        current_dir: &Path,
    ) -> Result<std::process::Output, SkillError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut command = Command::new("npx");
        command.arg("-y").arg("skills@latest");
        for arg in args {
            command.arg(arg);
        }
        command.current_dir(current_dir);
        command.output().map_err(|_| SkillError::CommandFailed)
    }
}
