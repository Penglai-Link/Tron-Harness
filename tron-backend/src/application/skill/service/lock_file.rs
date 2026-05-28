use super::*;

pub(super) fn read_skills_lock(
    path: &Path,
) -> Result<BTreeMap<String, SkillsLockEntry>, SkillError> {
    Ok(read_skills_lock_file(path)?.skills)
}

pub(super) fn read_skills_lock_file(path: &Path) -> Result<SkillsLockFile, SkillError> {
    if !path.exists() {
        return Ok(SkillsLockFile {
            version: default_lock_version(),
            skills: BTreeMap::new(),
        });
    }
    let body = fs::read_to_string(path).map_err(|_| SkillError::StorageFailure)?;
    serde_json::from_str(&body).map_err(|_| SkillError::StorageFailure)
}

pub(super) fn write_skills_lock_file(
    path: &Path,
    lock_file: &SkillsLockFile,
) -> Result<(), SkillError> {
    let body = serde_json::to_vec_pretty(lock_file).map_err(|_| SkillError::StorageFailure)?;
    fs::write(path, body).map_err(|_| SkillError::StorageFailure)
}

pub(super) fn remove_skill_lock_entry(path: &Path, skill_name: &str) -> Result<(), SkillError> {
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

pub(super) fn default_lock_version() -> u8 {
    1
}
