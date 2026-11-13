use super::{CapabilityContentInfo, content};
use eliteforge_agent_capability_runtime::{CapabilityPackManifest, safe_relative};
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn skill_contents(
    package: &Path,
    manifest: &CapabilityPackManifest,
) -> Result<Vec<CapabilityContentInfo>, String> {
    let package = package.canonicalize().map_err(|error| error.to_string())?;
    let mut skills = BTreeMap::new();
    for root in &manifest.skills {
        let root = package.join(safe_relative(root).map_err(|error| error.to_string())?);
        collect(&package, &root, 0, &mut skills)?;
    }
    Ok(skills.into_values().collect())
}

fn collect(
    package: &Path,
    directory: &Path,
    depth: usize,
    skills: &mut BTreeMap<String, CapabilityContentInfo>,
) -> Result<(), String> {
    if depth > 16 {
        return Err("capability skill directory nesting exceeds 16 levels".into());
    }
    let directory = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !directory.starts_with(package) {
        return Err("capability skill directory escapes its package".into());
    }
    let markdown = directory.join("SKILL.md");
    if markdown.is_file() {
        let metadata = fs::symlink_metadata(&markdown).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || metadata.len() > 512 * 1024 {
            return Err("invalid capability skill markdown file".into());
        }
        let markdown = fs::read_to_string(&markdown).map_err(|error| error.to_string())?;
        let (name, description) = crate::application::skill::skill_metadata(&markdown)
            .map_err(|error| format!("invalid skill in {}: {error}", directory.display()))?;
        skills.insert(name.clone(), content("Skill", name, description));
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            collect(package, &entry.path(), depth + 1, skills)?;
        }
    }
    Ok(())
}
