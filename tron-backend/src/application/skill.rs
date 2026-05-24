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
};

const DEFAULT_TIMESTAMP: &str = "2026-05-19T00:00:00Z";

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

        let mut items = resolved_bindings
            .into_iter()
            .filter(|(skill_name, _)| matches_workspace_skill_search(skill_name, search.as_deref()))
            .map(|(skill_name, resolved)| {
                build_workspace_skill_list_item(skill_map.get(skill_name.as_str()), &resolved)
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
            Ok(build_workspace_skill_detail(
                Some(&skill),
                &resolved,
                Some(skill_markdown),
                files,
            ))
        } else {
            Ok(build_workspace_skill_detail(
                None,
                &resolved,
                None,
                Vec::new(),
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
            });
        }

        Ok(SkillRegistryMutationResult {
            skill_name,
            workspace_code: None,
            enabled: false,
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
            created_at: DEFAULT_TIMESTAMP.into(),
            updated_at: DEFAULT_TIMESTAMP.into(),
        };
        self.repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .upsert_workspace_binding(&binding)?;
        Ok(SkillBindingMutationResult {
            workspace_code,
            skill_name,
            enabled,
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
                    Some(DEFAULT_TIMESTAMP.into())
                } else {
                    existing_skill.and_then(|skill| skill.last_checked_at.clone())
                },
                last_updated_at: if updated {
                    Some(DEFAULT_TIMESTAMP.into())
                } else {
                    existing_skill.and_then(|skill| skill.last_updated_at.clone())
                },
                last_error: None,
                created_at: existing_skill
                    .map(|skill| skill.created_at.clone())
                    .unwrap_or_else(|| DEFAULT_TIMESTAMP.into()),
                updated_at: DEFAULT_TIMESTAMP.into(),
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

fn read_skills_lock(path: &Path) -> Result<BTreeMap<String, SkillsLockEntry>, SkillError> {
    Ok(read_skills_lock_file(path)?.skills)
}

fn read_skills_lock_file(path: &Path) -> Result<SkillsLockFile, SkillError> {
    if !path.exists() {
        return Ok(SkillsLockFile {
            version: default_lock_version(),
            skills: BTreeMap::new(),
        });
    }
    let body = fs::read_to_string(path).map_err(|_| SkillError::StorageFailure)?;
    serde_json::from_str(&body).map_err(|_| SkillError::StorageFailure)
}

fn write_skills_lock_file(path: &Path, lock_file: &SkillsLockFile) -> Result<(), SkillError> {
    let body = serde_json::to_vec_pretty(lock_file).map_err(|_| SkillError::StorageFailure)?;
    fs::write(path, body).map_err(|_| SkillError::StorageFailure)
}

fn remove_skill_lock_entry(path: &Path, skill_name: &str) -> Result<(), SkillError> {
    let mut lock_file = read_skills_lock_file(path)?;
    lock_file.skills.remove(skill_name);
    if lock_file.skills.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|_| SkillError::StorageFailure)?;
        }
        return Ok(());
    }
    write_skills_lock_file(path, &lock_file)
}

fn default_lock_version() -> u8 {
    1
}

fn list_skill_directories(skills_dir: &Path) -> Result<Vec<PathBuf>, SkillError> {
    if !skills_dir.exists() {
        return Ok(Vec::new());
    }
    let mut dirs = fs::read_dir(skills_dir)
        .map_err(|_| SkillError::StorageFailure)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    Ok(dirs)
}

fn list_skill_files(root: &Path) -> Result<Vec<RegistrySkillFile>, SkillError> {
    let mut files = Vec::new();
    walk_skill_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn walk_skill_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<RegistrySkillFile>,
) -> Result<(), SkillError> {
    for entry in fs::read_dir(current).map_err(|_| SkillError::StorageFailure)? {
        let entry = entry.map_err(|_| SkillError::StorageFailure)?;
        let path = entry.path();
        if path.is_dir() {
            walk_skill_files(root, &path, files)?;
            continue;
        }
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| SkillError::InvalidRegistryPath)?
            .display()
            .to_string();
        files.push(RegistrySkillFile {
            is_markdown: path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("md")),
            path: relative_path,
        });
    }
    Ok(())
}

fn read_skill_markdown(skill_dir: &Path) -> Result<String, SkillError> {
    let canonical = skill_dir.join("SKILL.md");
    if canonical.exists() {
        return fs::read_to_string(canonical).map_err(|_| SkillError::StorageFailure);
    }
    let fallback = skill_dir.join("Skill.md");
    if fallback.exists() {
        return fs::read_to_string(fallback).map_err(|_| SkillError::StorageFailure);
    }
    Err(SkillError::NotFound)
}

fn parse_skill_frontmatter(markdown: &str) -> SkillFrontmatter {
    let mut lines = markdown.lines();
    if lines.next() != Some("---") {
        return SkillFrontmatter::default();
    }

    let mut yaml_lines = Vec::new();
    for line in lines {
        if line == "---" {
            let yaml = yaml_lines.join("\n");
            let value = match serde_yaml::from_str::<serde_yaml::Value>(&yaml) {
                Ok(value) => value,
                Err(_) => return SkillFrontmatter::default(),
            };
            let metadata = value.get("metadata");
            return SkillFrontmatter {
                declared_version: yaml_value_as_string(value.get("version"))
                    .or_else(|| metadata.and_then(|item| yaml_mapping_value(item, "version"))),
                description: yaml_value_as_string(value.get("description"))
                    .or_else(|| metadata.and_then(|item| yaml_mapping_value(item, "description"))),
            };
        }
        yaml_lines.push(line.to_owned());
    }

    SkillFrontmatter::default()
}

fn yaml_value_as_string(value: Option<&serde_yaml::Value>) -> Option<String> {
    match value {
        Some(serde_yaml::Value::String(value)) => Some(value.clone()),
        Some(other) => serde_json::to_string(other).ok(),
        None => None,
    }
}

fn yaml_mapping_value(value: &serde_yaml::Value, key: &str) -> Option<String> {
    match value {
        serde_yaml::Value::Mapping(mapping) => mapping
            .get(serde_yaml::Value::String(key.to_owned()))
            .and_then(|item| yaml_value_as_string(Some(item))),
        _ => None,
    }
}

fn parse_search_output(stdout: &[u8]) -> Vec<SkillSearchCandidate> {
    let normalized = strip_ansi(&String::from_utf8_lossy(stdout));
    let mut candidates = Vec::new();
    let mut pending: Option<SearchLine> = None;

    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("Install with") {
            continue;
        }
        if let Some(detail_url) = line.strip_prefix("└ ") {
            if let Some(pending_line) = pending.take()
                && let Ok((source_ref, skill_name)) = split_package_ref(&pending_line.package_ref)
            {
                candidates.push(SkillSearchCandidate {
                    package_ref: pending_line.package_ref,
                    source_ref,
                    skill_name,
                    description: None,
                    installs_label: pending_line.installs_label,
                    detail_url: Some(detail_url.trim().to_owned()),
                });
            }
            continue;
        }

        if let Some(search_line) = parse_search_line(line) {
            pending = Some(search_line);
        }
    }

    if let Some(pending_line) = pending
        && let Ok((source_ref, skill_name)) = split_package_ref(&pending_line.package_ref)
    {
        candidates.push(SkillSearchCandidate {
            package_ref: pending_line.package_ref,
            source_ref,
            skill_name,
            description: None,
            installs_label: pending_line.installs_label,
            detail_url: None,
        });
    }

    candidates
}

fn parse_repo_listing_output(stdout: &[u8], source_ref: &str) -> Vec<SkillSearchCandidate> {
    let normalized = strip_ansi(&String::from_utf8_lossy(stdout));
    let source_ref = source_ref.trim();
    let mut candidates = Vec::new();
    let mut in_skills_section = false;
    let mut current_skill: Option<String> = None;
    let mut description_lines: Vec<String> = Vec::new();

    fn flush_candidate(
        candidates: &mut Vec<SkillSearchCandidate>,
        source_ref: &str,
        current_skill: &mut Option<String>,
        description_lines: &mut Vec<String>,
    ) {
        let Some(skill_name) = current_skill.take() else {
            return;
        };
        let description = join_description_lines(description_lines);
        candidates.push(SkillSearchCandidate {
            package_ref: format!("{source_ref}@{skill_name}"),
            source_ref: source_ref.to_owned(),
            skill_name,
            description,
            installs_label: None,
            detail_url: None,
        });
        description_lines.clear();
    }

    for raw_line in normalized.lines() {
        let content = trim_cli_line(raw_line);
        if content.is_empty() {
            continue;
        }
        if !in_skills_section {
            if content == "Available Skills" {
                in_skills_section = true;
            }
            continue;
        }
        if content.starts_with("Use --skill ") {
            break;
        }
        if is_repo_skill_name(content) {
            flush_candidate(
                &mut candidates,
                source_ref,
                &mut current_skill,
                &mut description_lines,
            );
            current_skill = Some(content.to_owned());
            continue;
        }
        if current_skill.is_some() {
            description_lines.push(content.to_owned());
        }
    }

    flush_candidate(
        &mut candidates,
        source_ref,
        &mut current_skill,
        &mut description_lines,
    );

    candidates
}

fn parse_search_line(line: &str) -> Option<SearchLine> {
    if let Some(prefix) = line.strip_suffix(" installs")
        && let Some((package_ref, install_count)) = prefix.rsplit_once(' ')
    {
        return Some(SearchLine {
            package_ref: package_ref.trim().to_owned(),
            installs_label: Some(format!("{install_count} installs")),
        });
    }

    normalized_required(line).map(|package_ref| SearchLine {
        package_ref,
        installs_label: None,
    })
}

fn trim_cli_line(line: &str) -> &str {
    line.trim()
        .trim_start_matches(['│', '└', '◇', '●', '◒', '◐', '◓', '◑', ' '])
        .trim()
}

fn is_repo_skill_name(line: &str) -> bool {
    !line.is_empty()
        && !line.contains(char::is_whitespace)
        && line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn join_description_lines(lines: &mut Vec<String>) -> Option<String> {
    if lines.is_empty() {
        return None;
    }
    let joined = lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    lines.clear();
    normalized_required(joined)
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek().is_some_and(|next| *next == '[') {
                let _ = chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        output.push(ch);
    }

    output
}

fn split_package_ref(package_ref: &str) -> Result<(String, String), SkillError> {
    let package_ref = normalized_required(package_ref).ok_or(SkillError::SourceRefRequired)?;
    let (source_ref, skill_name) = package_ref
        .rsplit_once('@')
        .ok_or(SkillError::SkillNameRequired)?;
    let source_ref = normalized_required(source_ref).ok_or(SkillError::SourceRefRequired)?;
    let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
    Ok((source_ref, skill_name))
}

fn build_package_ref(source_ref: Option<String>, skill_name: Option<String>) -> Option<String> {
    let source_ref = optional_trimmed(source_ref)?;
    let skill_name = optional_trimmed(skill_name)?;
    Some(format!("{source_ref}@{skill_name}"))
}

fn normalized_required(value: impl Into<String>) -> Option<String> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(normalized_required)
}

fn normalized_search(value: Option<String>) -> Option<String> {
    optional_trimmed(value).map(|search| search.to_lowercase())
}

fn workspace_lineage(workspace: &WorkspaceDetail) -> Vec<LineageEntry> {
    let mut labels = Vec::new();
    workspace
        .lineage
        .iter()
        .map(|item| {
            labels.push(item.name.clone());
            LineageEntry {
                code: item.workspace_code.clone(),
                label: labels.join(" / "),
                name: item.name.clone(),
            }
        })
        .collect()
}

fn matches_skill_search(skill: &RegistrySkill, search: Option<&str>) -> bool {
    let Some(search) = search else {
        return true;
    };
    let haystack = [
        skill.skill_name.as_str(),
        skill.source_ref.as_str(),
        skill.registry_path.as_str(),
        skill.declared_version.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase();
    haystack.contains(search)
}

fn matches_workspace_skill_search(skill_name: &str, search: Option<&str>) -> bool {
    let Some(search) = search else {
        return true;
    };
    skill_name.to_lowercase().contains(search)
}

fn build_workspace_skill_list_item(
    skill: Option<&RegistrySkill>,
    resolved: &ResolvedSkillBinding,
) -> WorkspaceSkillListItem {
    WorkspaceSkillListItem {
        skill_name: resolved.binding.skill_name.clone(),
        source_ref: skill.map(|entry| entry.source_ref.clone()),
        source_type: skill.map(|entry| entry.source_type),
        registry_path: skill.map(|entry| entry.registry_path.clone()),
        declared_version: skill.and_then(|entry| entry.declared_version.clone()),
        update_status: skill.map(|entry| entry.update_status),
        managed_by: skill.map(|entry| entry.managed_by),
        last_checked_at: skill.and_then(|entry| entry.last_checked_at.clone()),
        last_updated_at: skill.and_then(|entry| entry.last_updated_at.clone()),
        last_error: skill
            .and_then(|entry| entry.last_error.clone())
            .or_else(|| match skill {
                Some(_) => None,
                None => Some("Skill is no longer present in ~/.tron/skills-registry".into()),
            }),
        enabled: resolved.binding.enabled,
        source_kind: resolved.source_kind,
        source_workspace_code: Some(resolved.binding.workspace_code.clone()),
        source_workspace_name: Some(resolved.source_workspace_name.clone()),
        source_label: Some(resolved.source_label.clone()),
        shadowed_workspace_code: resolved.shadowed_workspace_code.clone(),
        shadowed_workspace_name: resolved.shadowed_workspace_name.clone(),
        shadowed_label: resolved.shadowed_label.clone(),
        shadowed_enabled: resolved.shadowed_enabled,
        registry_state: if skill.is_some() {
            SkillRegistryState::Present
        } else {
            SkillRegistryState::Deleted
        },
    }
}

fn build_workspace_skill_detail(
    skill: Option<&RegistrySkill>,
    resolved: &ResolvedSkillBinding,
    skill_markdown: Option<String>,
    files: Vec<RegistrySkillFile>,
) -> WorkspaceSkillDetail {
    WorkspaceSkillDetail {
        skill_name: resolved.binding.skill_name.clone(),
        source_ref: skill.map(|entry| entry.source_ref.clone()),
        source_type: skill.map(|entry| entry.source_type),
        registry_path: skill.map(|entry| entry.registry_path.clone()),
        declared_version: skill.and_then(|entry| entry.declared_version.clone()),
        update_status: skill.map(|entry| entry.update_status),
        managed_by: skill.map(|entry| entry.managed_by),
        last_checked_at: skill.and_then(|entry| entry.last_checked_at.clone()),
        last_updated_at: skill.and_then(|entry| entry.last_updated_at.clone()),
        last_error: skill
            .and_then(|entry| entry.last_error.clone())
            .or_else(|| match skill {
                Some(_) => None,
                None => Some("Skill is no longer present in ~/.tron/skills-registry".into()),
            }),
        skill_markdown,
        files,
        enabled: resolved.binding.enabled,
        source_kind: resolved.source_kind,
        source_workspace_code: Some(resolved.binding.workspace_code.clone()),
        source_workspace_name: Some(resolved.source_workspace_name.clone()),
        source_label: Some(resolved.source_label.clone()),
        shadowed_workspace_code: resolved.shadowed_workspace_code.clone(),
        shadowed_workspace_name: resolved.shadowed_workspace_name.clone(),
        shadowed_label: resolved.shadowed_label.clone(),
        shadowed_enabled: resolved.shadowed_enabled,
        registry_state: if skill.is_some() {
            SkillRegistryState::Present
        } else {
            SkillRegistryState::Deleted
        },
    }
}

fn binding_counts(bindings: &[WorkspaceSkillBinding]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for binding in bindings {
        if binding.enabled {
            *counts.entry(binding.skill_name.as_str()).or_insert(0) += 1;
        }
    }
    counts
}

fn parse_source_type(value: &str) -> Option<SkillSourceType> {
    match value {
        "marketplace" => Some(SkillSourceType::Marketplace),
        "github" => Some(SkillSourceType::GitHub),
        "git" => Some(SkillSourceType::Git),
        "url" => Some(SkillSourceType::Url),
        "local" => Some(SkillSourceType::Local),
        "manual" => Some(SkillSourceType::Manual),
        _ => None,
    }
}

fn map_workspace_error(error: WorkspaceError) -> SkillError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::InvalidWorkspaceCode => {
            SkillError::InvalidWorkspace
        }
        _ => SkillError::StorageFailure,
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), SkillError> {
    fs::create_dir_all(destination).map_err(|_| SkillError::StorageFailure)?;
    for entry in fs::read_dir(source).map_err(|_| SkillError::StorageFailure)? {
        let entry = entry.map_err(|_| SkillError::StorageFailure)?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|_| SkillError::StorageFailure)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{collections::BTreeMap, fs};

    use crate::{
        application::{
            role::{RoleRepository, RoleService},
            user::{PasswordHasher, UserRepository, UserService},
            workspace::{WorkspaceRepository, WorkspaceService},
        },
        config::AppConfig,
        domain::{
            auth::{AuthError, OAuth2IdentityLink, RefreshSession},
            role::{Role, RoleError},
            skill::{
                RegistrySkill, SkillManagedBy, SkillSourceType, SkillUpdateStatus,
                WorkspaceSkillBinding,
            },
            user::{ListUsersQuery, PagedUsers, User, UserError, UserStatistics},
            workspace::{Workspace, WorkspaceError, WorkspaceRoleAccess, WorkspaceUserAccess},
        },
    };

    use super::{
        DEFAULT_TIMESTAMP, SkillRegistryRepository, SkillRegistryService, build_package_ref,
        parse_repo_listing_output, parse_search_output, parse_skill_frontmatter, read_skills_lock,
        strip_ansi,
    };

    #[derive(Default)]
    struct TestSkillRegistryRepository {
        registry_skills: BTreeMap<String, RegistrySkill>,
        workspace_bindings: Vec<WorkspaceSkillBinding>,
    }

    impl SkillRegistryRepository for TestSkillRegistryRepository {
        fn list_registry_skills(
            &mut self,
        ) -> Result<Vec<RegistrySkill>, crate::domain::skill::SkillError> {
            Ok(self.registry_skills.values().cloned().collect())
        }

        fn upsert_registry_skill(
            &mut self,
            skill: &RegistrySkill,
        ) -> Result<(), crate::domain::skill::SkillError> {
            self.registry_skills
                .insert(skill.skill_name.clone(), skill.clone());
            Ok(())
        }

        fn delete_registry_skill(
            &mut self,
            skill_name: &str,
        ) -> Result<bool, crate::domain::skill::SkillError> {
            Ok(self.registry_skills.remove(skill_name).is_some())
        }

        fn list_workspace_bindings(
            &mut self,
            workspace_code: Option<&str>,
        ) -> Result<Vec<WorkspaceSkillBinding>, crate::domain::skill::SkillError> {
            Ok(self
                .workspace_bindings
                .iter()
                .filter(|binding| workspace_code.is_none_or(|code| binding.workspace_code == code))
                .cloned()
                .collect())
        }

        fn upsert_workspace_binding(
            &mut self,
            binding: &WorkspaceSkillBinding,
        ) -> Result<(), crate::domain::skill::SkillError> {
            if let Some(existing) = self.workspace_bindings.iter_mut().find(|existing| {
                existing.workspace_code == binding.workspace_code
                    && existing.skill_name == binding.skill_name
            }) {
                *existing = binding.clone();
            } else {
                self.workspace_bindings.push(binding.clone());
            }
            Ok(())
        }

        fn delete_workspace_binding(
            &mut self,
            workspace_code: &str,
            skill_name: &str,
        ) -> Result<bool, crate::domain::skill::SkillError> {
            let original_len = self.workspace_bindings.len();
            self.workspace_bindings.retain(|binding| {
                !(binding.workspace_code == workspace_code && binding.skill_name == skill_name)
            });
            Ok(self.workspace_bindings.len() != original_len)
        }

        fn delete_bindings_for_skill(
            &mut self,
            skill_name: &str,
        ) -> Result<(), crate::domain::skill::SkillError> {
            self.workspace_bindings
                .retain(|binding| binding.skill_name != skill_name);
            Ok(())
        }
    }

    struct DummyWorkspaceRepository;

    impl WorkspaceRepository for DummyWorkspaceRepository {
        fn list_workspaces(&mut self) -> Result<Vec<Workspace>, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn next_id(&mut self) -> Result<i64, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn create_workspace(&mut self, _workspace: &Workspace) -> Result<(), WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn update_workspace(&mut self, _workspace: &Workspace) -> Result<(), WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn list_user_access(
            &mut self,
            _workspace_code: &str,
        ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn replace_user_access(
            &mut self,
            _workspace_code: &str,
            _entries: &[WorkspaceUserAccess],
        ) -> Result<Vec<WorkspaceUserAccess>, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn list_role_access(
            &mut self,
            _workspace_code: &str,
        ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn replace_role_access(
            &mut self,
            _workspace_code: &str,
            _entries: &[WorkspaceRoleAccess],
        ) -> Result<Vec<WorkspaceRoleAccess>, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn archive_subtree(
            &mut self,
            _workspace_codes: &[String],
            _archived_at: &str,
        ) -> Result<usize, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }

        fn restore_subtree(
            &mut self,
            _workspace_codes: &[String],
            _restored_at: &str,
        ) -> Result<usize, WorkspaceError> {
            unreachable!("workspace repository should not be used in sync_registry tests")
        }
    }

    struct DummyUserRepository;

    impl UserRepository for DummyUserRepository {
        fn list_users(&mut self, _query: ListUsersQuery) -> Result<PagedUsers, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn get_user(&mut self, _user_code: &str) -> Result<User, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn statistics(&mut self) -> Result<UserStatistics, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn next_id(&mut self) -> Result<i64, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn next_user_code(&mut self, _username: &str) -> Result<String, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn user_code_exists(
            &mut self,
            _user_code: &str,
            _except_user_code: Option<&str>,
        ) -> Result<bool, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn username_exists(
            &mut self,
            _username: &str,
            _except_user_code: Option<&str>,
        ) -> Result<bool, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn email_exists(
            &mut self,
            _email: &str,
            _except_user_code: Option<&str>,
        ) -> Result<bool, UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn insert_user(&mut self, _user: &User) -> Result<(), UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn update_user(&mut self, _user: &User) -> Result<(), UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn replace_role_codes(&mut self, _user: &User) -> Result<(), UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn replace_space_codes(&mut self, _user: &User) -> Result<(), UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn record_password_reset(&mut self, _user: &User, _reset_id: i64) -> Result<(), UserError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn find_user_by_login(&mut self, _login: &str) -> Result<Option<User>, AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn find_active_user_by_email(&mut self, _email: &str) -> Result<Option<User>, AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn find_user_by_provider_subject(
            &mut self,
            _provider_code: &str,
            _provider_subject: &str,
        ) -> Result<Option<User>, AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn create_refresh_session(&mut self, _session: &RefreshSession) -> Result<(), AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn find_refresh_session(
            &mut self,
            _session_code: &str,
        ) -> Result<Option<RefreshSession>, AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn replace_refresh_session(&mut self, _session: &RefreshSession) -> Result<(), AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn revoke_refresh_session(
            &mut self,
            _session_code: &str,
            _revoked_at: i64,
        ) -> Result<(), AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }

        fn upsert_oauth_identity(
            &mut self,
            _identity: &OAuth2IdentityLink,
        ) -> Result<(), AuthError> {
            unreachable!("user repository should not be used in sync_registry tests")
        }
    }

    struct DummyPasswordHasher;

    impl PasswordHasher for DummyPasswordHasher {
        fn hash_password(&self, _value: &str) -> Result<String, UserError> {
            unreachable!("password hasher should not be used in sync_registry tests")
        }

        fn verify_password(&self, _value: &str, _hash: &str) -> Result<bool, AuthError> {
            unreachable!("password hasher should not be used in sync_registry tests")
        }
    }

    struct DummyRoleRepository;

    impl RoleRepository for DummyRoleRepository {
        fn list_roles(&mut self) -> Result<Vec<Role>, RoleError> {
            unreachable!("role repository should not be used in sync_registry tests")
        }

        fn next_id(&mut self) -> Result<i64, RoleError> {
            unreachable!("role repository should not be used in sync_registry tests")
        }

        fn replace_roles(&mut self, _roles: &[Role]) -> Result<Vec<Role>, RoleError> {
            unreachable!("role repository should not be used in sync_registry tests")
        }
    }

    fn build_test_workspace_service() -> WorkspaceService {
        WorkspaceService::new(
            DummyWorkspaceRepository,
            UserService::new(
                DummyUserRepository,
                DummyPasswordHasher,
                AppConfig::default().auth,
            ),
            RoleService::new(DummyRoleRepository),
        )
    }

    #[test]
    fn parses_search_output_candidates() {
        let output = "\u{1b}[38;5;102mInstall with\u{1b}[0m npx skills add <owner/repo@skill>\n\
wshobson/agents@typescript-advanced-types 42K installs\n\
└ https://skills.sh/wshobson/agents/typescript-advanced-types\n\
github/awesome-copilot@javascript-typescript-jest 10.5K installs\n\
└ https://skills.sh/github/awesome-copilot/javascript-typescript-jest\n";
        let candidates = parse_search_output(output.as_bytes());
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].source_ref, "wshobson/agents");
        assert_eq!(candidates[0].skill_name, "typescript-advanced-types");
        assert_eq!(candidates[0].description, None);
        assert_eq!(
            candidates[0].installs_label.as_deref(),
            Some("42K installs")
        );
        assert_eq!(
            candidates[1].detail_url.as_deref(),
            Some("https://skills.sh/github/awesome-copilot/javascript-typescript-jest")
        );
    }

    #[test]
    fn parses_repo_listing_output_candidates() {
        let output = "◇  Available Skills\n\
│\n\
│    eliteforge-agent-doctor\n\
│\n\
│      检查开发环境是否配置完成\n\
│      缺少命令时自动补齐\n\
│\n\
│    eliteforge-be-fe-consensus-spec\n\
│\n\
│      约束前后端接口共识\n\
│\n\
└  Use --skill <name> to install specific skills\n";
        let candidates = parse_repo_listing_output(output.as_bytes(), "cloudsen/eliteforge-skills");
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].package_ref,
            "cloudsen/eliteforge-skills@eliteforge-agent-doctor"
        );
        assert_eq!(
            candidates[0].description.as_deref(),
            Some("检查开发环境是否配置完成 缺少命令时自动补齐")
        );
        assert_eq!(candidates[1].skill_name, "eliteforge-be-fe-consensus-spec");
        assert_eq!(candidates[1].installs_label, None);
        assert_eq!(candidates[1].detail_url, None);
    }

    #[test]
    fn strips_ansi_sequences() {
        let output = strip_ansi("\u{1b}[38;5;102mhello\u{1b}[0m");
        assert_eq!(output, "hello");
    }

    #[test]
    fn parses_skill_frontmatter_fields() {
        let markdown = "---\nversion: 1.2.3\ndescription: Example skill\n---\n# Skill";
        let frontmatter = parse_skill_frontmatter(markdown);
        assert_eq!(frontmatter.declared_version.as_deref(), Some("1.2.3"));
        assert_eq!(frontmatter.description.as_deref(), Some("Example skill"));
    }

    #[test]
    fn parses_skill_frontmatter_metadata_fields() {
        let markdown =
            "---\nmetadata:\n  version: 2.3.4\n  description: Metadata skill\n---\n# Skill";
        let frontmatter = parse_skill_frontmatter(markdown);
        assert_eq!(frontmatter.declared_version.as_deref(), Some("2.3.4"));
        assert_eq!(frontmatter.description.as_deref(), Some("Metadata skill"));
    }

    #[test]
    fn builds_package_ref_from_source_and_skill_name() {
        let package_ref = build_package_ref(
            Some("wshobson/agents".into()),
            Some("typescript-advanced-types".into()),
        );
        assert_eq!(
            package_ref.as_deref(),
            Some("wshobson/agents@typescript-advanced-types")
        );
    }

    #[test]
    fn reads_skills_lock_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("tron-skill-lock-{unique}"));
        fs::create_dir_all(&temp_dir).expect("temp skill dir should exist");
        let lock_path = temp_dir.join("skills-lock.json");
        fs::write(
            &lock_path,
            r#"{
  "version": 1,
  "skills": {
    "typescript-advanced-types": {
      "source": "wshobson/agents",
      "sourceType": "github"
    }
  }
}"#,
        )
        .expect("skills lock should be written");

        let skills = read_skills_lock(&lock_path).expect("lock file should parse");
        let entry = skills
            .get("typescript-advanced-types")
            .expect("skill should exist");
        assert_eq!(entry.source, "wshobson/agents");
        assert_eq!(entry.source_type.as_deref(), Some("github"));

        fs::remove_dir_all(&temp_dir).expect("temp skill dir should be removed");
    }

    #[test]
    fn enums_are_copyable_for_registry_models() {
        let source_type = SkillSourceType::GitHub;
        let update_status = SkillUpdateStatus::Unknown;
        let managed_by = SkillManagedBy::Cli;
        assert!(matches!(source_type, SkillSourceType::GitHub));
        assert!(matches!(update_status, SkillUpdateStatus::Unknown));
        assert!(matches!(managed_by, SkillManagedBy::Cli));
    }

    #[test]
    fn sync_registry_keeps_workspace_bindings_when_registry_skill_disappears() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let registry_root = std::env::temp_dir().join(format!("tron-skill-registry-sync-{unique}"));
        let skills_dir = registry_root.join(".agents/skills");
        fs::create_dir_all(&skills_dir).expect("skills directory should exist");

        let skill_name = "eliteforge-agent-doctor".to_string();
        let repository = TestSkillRegistryRepository {
            registry_skills: BTreeMap::from([(
                skill_name.clone(),
                RegistrySkill {
                    skill_name: skill_name.clone(),
                    source_ref: "cloudsen/eliteforge-skills".into(),
                    source_type: SkillSourceType::GitHub,
                    registry_path: skills_dir.join(&skill_name).display().to_string(),
                    declared_version: Some("1.0.0".into()),
                    update_status: SkillUpdateStatus::UpToDate,
                    managed_by: SkillManagedBy::Cli,
                    last_checked_at: None,
                    last_updated_at: None,
                    last_error: None,
                    created_at: DEFAULT_TIMESTAMP.into(),
                    updated_at: DEFAULT_TIMESTAMP.into(),
                },
            )]),
            workspace_bindings: vec![WorkspaceSkillBinding {
                workspace_code: "TRON_CORE".into(),
                skill_name: skill_name.clone(),
                enabled: true,
                created_at: DEFAULT_TIMESTAMP.into(),
                updated_at: DEFAULT_TIMESTAMP.into(),
            }],
        };

        let service =
            SkillRegistryService::new(repository, build_test_workspace_service(), &registry_root);

        service
            .sync_registry(false, false)
            .expect("registry sync should succeed");

        let remaining_registry_skills = service
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()
            .expect("registry skills should be listed");
        assert!(remaining_registry_skills.is_empty());

        let remaining_bindings = service
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_workspace_bindings(None)
            .expect("workspace bindings should be listed");
        assert_eq!(remaining_bindings.len(), 1);
        assert_eq!(remaining_bindings[0].skill_name, skill_name);

        fs::remove_dir_all(&registry_root).expect("temp skill registry should be removed");
    }
}
