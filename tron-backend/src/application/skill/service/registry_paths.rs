use super::*;

impl SkillRegistryService {
    pub(super) fn ensure_registry_root(&self) -> Result<(), SkillError> {
        fs::create_dir_all(self.skills_dir()).map_err(|_| SkillError::StorageFailure)
    }

    pub(super) fn registry_root(&self) -> &Path {
        &self.registry_root
    }

    pub(super) fn skills_dir(&self) -> PathBuf {
        self.registry_root.join(".agents").join("skills")
    }

    pub(super) fn registry_lock_path(&self) -> PathBuf {
        self.registry_root.join("skills-lock.json")
    }

    pub(super) fn run_skills_command<'a, I>(
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
