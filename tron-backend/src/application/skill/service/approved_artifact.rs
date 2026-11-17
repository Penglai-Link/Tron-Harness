use super::*;
use crate::{
    application::assistant_memory::{artifact_hash, materialize_artifact},
    domain::assistant_memory::{AssistantMemoryError, SkillArtifactFile},
};

impl SkillRegistryService {
    /// The Skill center owns the only installation path for approved generated artifacts.
    /// Existing assignments are retained and newly created Skills are not automatically granted.
    pub fn install_approved_artifact(
        &self,
        name: &str,
        files: &[SkillArtifactFile],
        expected_base: Option<&str>,
        approved_hash: &str,
    ) -> Result<(), AssistantMemoryError> {
        let _guard = self.mutation_lock.lock().unwrap();
        if name.is_empty()
            || name.len() >= 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || name == "skill-creator"
        {
            return Err(AssistantMemoryError::Invalid(
                "invalid generated skill name".into(),
            ));
        }
        if artifact_hash(files)? != approved_hash {
            return Err(AssistantMemoryError::Conflict);
        }
        self.ensure_registry_root()
            .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        let records = self
            .repository
            .lock()
            .unwrap()
            .list_registry_skills()
            .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        if records.iter().any(|record| {
            record.skill_name == name && (record.provider_pack_id.is_some() || !record.available)
        }) {
            return Err(AssistantMemoryError::Conflict);
        }
        let destination = self.skills_dir().join(name);
        // The durable journal survives process death between the two filesystem renames.
        let journal = self
            .registry_root()
            .join(".approved-artifacts")
            .join(name)
            .join(approved_hash);
        let stage = journal.join("artifact");
        let backup = journal.join("previous");
        if !destination.exists() && backup.exists() {
            fs::rename(&backup, &destination)
                .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        }
        let current_hash = if destination.exists() {
            Some(artifact_hash(
                &crate::application::assistant_memory::read_artifact(&destination)?,
            )?)
        } else {
            None
        };
        // A crash after filesystem commit but before recording Installed is safe to retry.
        if current_hash.as_deref() == Some(approved_hash) {
            self.sync_registry(false, false)
                .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
            if journal.exists() {
                fs::remove_dir_all(&journal)
                    .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
            }
            return Ok(());
        }
        if current_hash.as_deref() != expected_base {
            return Err(AssistantMemoryError::Conflict);
        }
        fs::create_dir_all(&journal)
            .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        if stage.exists() {
            fs::remove_dir_all(&stage)
                .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        }
        materialize_artifact(&stage, files)?;
        if backup.exists() {
            return Err(AssistantMemoryError::Conflict);
        }
        if destination.exists() {
            fs::rename(&destination, &backup)
                .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        }
        if let Err(error) = fs::rename(&stage, &destination) {
            if backup.exists() {
                let _ = fs::rename(&backup, &destination);
            }
            return Err(AssistantMemoryError::Installation(error.to_string()));
        }
        if let Err(error) = self.sync_registry(false, false) {
            // Preserve previous working installation if indexing the registry failed.
            let _ = fs::remove_dir_all(&destination);
            if backup.exists() {
                let _ = fs::rename(&backup, &destination);
            }
            return Err(AssistantMemoryError::Installation(error.to_string()));
        }
        let actual = artifact_hash(&crate::application::assistant_memory::read_artifact(
            &destination,
        )?)?;
        if actual != approved_hash {
            return Err(AssistantMemoryError::Conflict);
        }
        fs::remove_dir_all(&journal)
            .map_err(|e| AssistantMemoryError::Installation(e.to_string()))?;
        Ok(())
    }
}
