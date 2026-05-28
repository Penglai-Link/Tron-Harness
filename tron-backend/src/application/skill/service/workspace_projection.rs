use super::*;

pub(super) fn workspace_lineage(workspace: &WorkspaceDetail) -> Vec<LineageEntry> {
    let mut labels = Vec::new();
    workspace
        .path
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

pub(super) fn matches_skill_search(skill: &RegistrySkill, search: Option<&str>) -> bool {
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

pub(super) fn matches_workspace_skill_search(skill_name: &str, search: Option<&str>) -> bool {
    let Some(search) = search else {
        return true;
    };
    skill_name.to_lowercase().contains(search)
}

pub(super) fn build_workspace_skill_list_item(
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

pub(super) fn build_workspace_skill_detail(
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

pub(super) fn binding_counts(bindings: &[WorkspaceSkillBinding]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for binding in bindings {
        if binding.enabled {
            *counts.entry(binding.skill_name.as_str()).or_insert(0) += 1;
        }
    }
    counts
}

pub(super) fn parse_source_type(value: &str) -> Option<SkillSourceType> {
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

pub(super) fn map_workspace_error(error: WorkspaceError) -> SkillError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::InvalidWorkspaceCode => {
            SkillError::InvalidWorkspace
        }
        _ => SkillError::StorageFailure,
    }
}

pub(super) fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), SkillError> {
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
