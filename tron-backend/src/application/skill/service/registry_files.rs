use super::*;

pub(super) fn list_skill_directories(skills_dir: &Path) -> Result<Vec<PathBuf>, SkillError> {
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

pub(super) fn list_skill_files(root: &Path) -> Result<Vec<RegistrySkillFile>, SkillError> {
    let mut files = Vec::new();
    walk_skill_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub(super) fn walk_skill_files(
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

pub(super) fn read_skill_markdown(skill_dir: &Path) -> Result<String, SkillError> {
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
