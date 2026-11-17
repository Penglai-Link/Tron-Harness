use super::{
    artifacts::{artifact_hash, materialize_artifact, read_artifact, validate_artifact},
    service::*,
};
use crate::{
    application::skill::SkillRegistryService,
    domain::{assistant_memory::*, skill::RegistrySkillListQuery},
};

#[derive(Debug, Clone)]
pub struct SkillAuthoringContext {
    pub candidate: SkillCandidate,
    pub creator_markdown: Option<String>,
    pub creator_files: Vec<SkillArtifactFile>,
}
impl AssistantMemoryService {
    pub fn begin_skill_authoring(
        &self,
        user: &str,
        batch_code: &str,
        skill_name: &str,
        session: &str,
    ) -> Result<SkillAuthoringContext, AssistantMemoryError> {
        let batch = self.get_batch(user, batch_code)?;
        let sources = batch
            .sources
            .iter()
            .map(|source| source.source_code.clone())
            .collect::<Vec<_>>();
        self.begin_skill_authoring_for_sources(user, batch_code, skill_name, session, &sources)
    }
    pub fn begin_skill_authoring_for_sources(
        &self,
        user: &str,
        batch_code: &str,
        skill_name: &str,
        session: &str,
        source_codes: &[String],
    ) -> Result<SkillAuthoringContext, AssistantMemoryError> {
        if skill_name == "skill-creator"
            || skill_name.is_empty()
            || skill_name.len() >= 64
            || !skill_name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(AssistantMemoryError::Invalid(
                "invalid generated skill name".into(),
            ));
        }
        let mut repository = self.repository.lock().unwrap();
        let batch = repository.get_batch(batch_code)?;
        if batch.user_code != user {
            return Err(AssistantMemoryError::NotFound);
        }
        if let Some(candidate) = repository.list_candidates(user)?.into_iter().find(|c| {
            c.batch_code == batch_code
                && c.skill_name == skill_name
                && !matches!(
                    c.status,
                    SkillCandidateStatus::RevisionRequested
                        | SkillCandidateStatus::WaitingDependency
                        | SkillCandidateStatus::Authoring
                )
        }) {
            return Ok(SkillAuthoringContext {
                candidate,
                creator_markdown: None,
                creator_files: Vec::new(),
            });
        }
        if source_codes.is_empty()
            || source_codes.iter().any(|code| {
                !batch
                    .sources
                    .iter()
                    .any(|source| source.source_code == *code)
            })
        {
            return Err(AssistantMemoryError::Invalid(
                "Skill evidence must identify sources in the current batch".into(),
            ));
        }
        // Only this experience's evidence participates in rejection deduplication. Unrelated
        // new conversations in the batch cannot make a rejected experience appear new.
        let evidence: Vec<SkillEvidence> = batch
            .sources
            .iter()
            .filter(|source| source_codes.contains(&source.source_code))
            .map(SkillEvidence::from)
            .collect();
        if let Some(candidate) = repository.list_candidates(user)?.into_iter().find(|c| {
            c.skill_name == skill_name
                && c.status == SkillCandidateStatus::Rejected
                && c.evidence == evidence
        }) {
            return Ok(SkillAuthoringContext {
                candidate,
                creator_markdown: None,
                creator_files: Vec::new(),
            });
        }
        let installed_files = installed_artifact(&self.skills, skill_name)?;
        let base_artifact_hash = (!installed_files.is_empty())
            .then(|| artifact_hash(&installed_files))
            .transpose()?;
        let previous = repository
            .list_candidates(user)?
            .into_iter()
            .find(|c| c.batch_code == batch_code && c.skill_name == skill_name);
        let mut candidate = previous.clone().unwrap_or_else(|| SkillCandidate {
            candidate_code: new_code("skill-candidate"),
            assistant_code: batch.assistant_code,
            batch_code: batch_code.into(),
            user_code: user.into(),
            skill_name: skill_name.into(),
            operation: if base_artifact_hash.is_some() {
                CandidateOperation::Update
            } else {
                CandidateOperation::Create
            },
            revision: 0,
            artifact_hash: String::new(),
            status: SkillCandidateStatus::Authoring,
            base_artifact_hash,
            creator: None,
            author_session_code: session.into(),
            files: Vec::new(),
            installed_files,
            evidence,
            validation: None,
            approved_hash: None,
            reason: None,
            feedback: None,
            last_error: None,
            created_at: now(),
            updated_at: now(),
        });
        candidate.author_session_code = session.into();
        candidate.updated_at = now();
        let creator = load_creator(&self.skills);
        let (markdown, creator_files) = match creator {
            Ok((receipt, markdown, files)) => {
                candidate.creator = Some(receipt);
                candidate.status = SkillCandidateStatus::Authoring;
                candidate.last_error = None;
                (Some(markdown), files)
            }
            Err(error) => {
                candidate.status = SkillCandidateStatus::WaitingDependency;
                candidate.last_error = Some(error.to_string());
                (None, Vec::new())
            }
        };
        repository.save_candidate(previous.as_ref(), &candidate)?;
        drop(repository);
        if candidate.status == SkillCandidateStatus::WaitingDependency {
            self.notify(
                &candidate,
                "SkillDependencyMissing",
                "Install and enable skill-creator to prepare experience Skills".into(),
            )?;
        }
        Ok(SkillAuthoringContext {
            candidate,
            creator_markdown: markdown,
            creator_files,
        })
    }
    /// This host tool must be called from the author session that received the installed skill-creator instructions.
    pub fn submit_candidate(
        &self,
        user: &str,
        code: &str,
        session: &str,
        files: Vec<SkillArtifactFile>,
        behavior: Vec<BehaviorCheck>,
        auto_approve: bool,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        let previous = self.get_candidate(user, code)?;
        if previous.author_session_code != session
            || previous.status != SkillCandidateStatus::Authoring
        {
            return Err(AssistantMemoryError::Conflict);
        }
        let (creator, _, _) = load_creator(&self.skills)?;
        if previous.creator.as_ref() != Some(&creator) {
            return Err(AssistantMemoryError::Dependency(
                "skill-creator changed; reload it before authoring".into(),
            ));
        }
        let hash = artifact_hash(&files)?;
        let validation = validate_artifact(&previous.skill_name, &files, &behavior)?;
        let mut candidate = previous.clone();
        candidate.revision += 1;
        candidate.artifact_hash = hash;
        candidate.files = files;
        candidate.validation = Some(validation.clone());
        candidate.approved_hash = None;
        candidate.updated_at = now();
        candidate.last_error = None;
        candidate.status = if validation.static_passed && validation.behavior_passed {
            SkillCandidateStatus::PendingApproval
        } else {
            SkillCandidateStatus::ValidationFailed
        };
        std::fs::create_dir_all(&self.staging_root)
            .map_err(|e| AssistantMemoryError::Storage(e.to_string()))?;
        let stage = self.staging_root.join(format!(
            "{}-{}-{}",
            candidate.candidate_code, candidate.revision, candidate.artifact_hash
        ));
        // Publish a complete stage atomically. A crash before the record CAS may leave the
        // same immutable stage behind; retry reuses it only when its bytes still match.
        if stage.exists() {
            if artifact_hash(&read_artifact(&stage)?)? != candidate.artifact_hash {
                return Err(AssistantMemoryError::Conflict);
            }
        } else {
            let temporary = tempfile::tempdir_in(&self.staging_root)
                .map_err(|e| AssistantMemoryError::Storage(e.to_string()))?;
            let artifact = temporary.path().join("artifact");
            materialize_artifact(&artifact, &candidate.files)?;
            if let Err(error) = std::fs::rename(&artifact, &stage) {
                if !stage.exists()
                    || artifact_hash(&read_artifact(&stage)?)? != candidate.artifact_hash
                {
                    return Err(AssistantMemoryError::Storage(error.to_string()));
                }
            }
        }
        self.repository
            .lock()
            .unwrap()
            .save_candidate(Some(&previous), &candidate)?;
        if candidate.status == SkillCandidateStatus::PendingApproval {
            if auto_approve {
                return self.approve(user, code, candidate.revision, &candidate.artifact_hash);
            }
            self.notify(
                &candidate,
                "SkillPendingApproval",
                format!(
                    "Skill {} is ready for approval and installation",
                    candidate.skill_name
                ),
            )?;
        } else {
            self.notify(
                &candidate,
                "SkillValidationFailed",
                format!(
                    "Skill {} needs revision before approval",
                    candidate.skill_name
                ),
            )?;
        }
        Ok(candidate)
    }
    /// Installation and retrieval indexing are distinct recoverable steps. The Skill center
    /// owns the index state; the candidate only keeps a retry marker and user-visible error.
    pub async fn index_installed(
        &self,
        candidate: &SkillCandidate,
        rag: crate::app::AppRagService,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        if candidate.status != SkillCandidateStatus::Installed {
            return Ok(candidate.clone());
        }
        let index = crate::application::skill::SkillRagIndexService::new(self.skills.clone(), rag)
            .index_registry_skill(&candidate.skill_name)
            .await;
        let failure = match index {
            Err(error) => Some(error.to_string()),
            Ok(summary)
                if summary.vector_status == crate::domain::skill::SkillRagIndexStatus::Failed
                    || summary.graph_status
                        == crate::domain::skill::SkillRagIndexStatus::Failed =>
            {
                Some(
                    summary
                        .last_error
                        .unwrap_or_else(|| "Skill retrieval index failed".into()),
                )
            }
            Ok(_) => None,
        };
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_candidate(&candidate.candidate_code)?;
        check_request(
            &previous,
            &candidate.user_code,
            candidate.revision,
            &candidate.artifact_hash,
        )?;
        if previous.status != SkillCandidateStatus::Installed {
            return Err(AssistantMemoryError::Conflict);
        }
        let mut next = previous.clone();
        next.last_error = failure.map(|error| format!("Skill index pending: {error}"));
        if next.last_error != previous.last_error {
            next.updated_at = now();
            repository.save_candidate(Some(&previous), &next)?;
            drop(repository);
            if let Some(error) = &next.last_error {
                self.notify(
                    &next,
                    "SkillIndexFailed",
                    format!(
                        "Skill {} is installed; retrieval indexing will retry: {error}",
                        next.skill_name
                    ),
                )?;
            } else if previous.last_error.is_some() {
                self.notify(
                    &next,
                    "SkillIndexRecovered",
                    format!("Skill {} retrieval index is ready", next.skill_name),
                )?;
            }
        }
        Ok(next)
    }
    pub fn approve(
        &self,
        user: &str,
        code: &str,
        revision: u32,
        hash: &str,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_candidate(code)?;
        check_request(&previous, user, revision, hash)?;
        if previous.status == SkillCandidateStatus::Installed
            && previous.approved_hash.as_deref() == Some(hash)
        {
            return Ok(previous);
        }
        if previous.status != SkillCandidateStatus::PendingApproval {
            return Err(AssistantMemoryError::Conflict);
        }
        if !previous
            .validation
            .as_ref()
            .is_some_and(|v| v.static_passed && v.behavior_passed && v.artifact_hash == hash)
            || artifact_hash(&previous.files)? != hash
        {
            return Err(AssistantMemoryError::Invalid(
                "the exact artifact must pass structural and behavioral validation".into(),
            ));
        }
        let mut candidate = previous.clone();
        candidate.status = SkillCandidateStatus::Approved;
        candidate.approved_hash = Some(hash.into());
        candidate.updated_at = now();
        repository.save_candidate(Some(&previous), &candidate)?;
        drop(repository);
        self.retry_install(user, code, revision, hash)
    }
    pub fn reject(
        &self,
        user: &str,
        code: &str,
        revision: u32,
        hash: &str,
        reason: String,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        self.decide(user, code, revision, hash, reason, false)
    }
    pub fn request_revision(
        &self,
        user: &str,
        code: &str,
        revision: u32,
        hash: &str,
        feedback: String,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        let candidate = self.decide(user, code, revision, hash, feedback, true)?;
        self.notify(
            &candidate,
            "SkillRevisionRequested",
            format!(
                "Revise Skill {} using the user's feedback",
                candidate.skill_name
            ),
        )?;
        Ok(candidate)
    }
    fn decide(
        &self,
        user: &str,
        code: &str,
        revision: u32,
        hash: &str,
        text: String,
        revise: bool,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        if text.trim().is_empty() {
            return Err(AssistantMemoryError::Invalid(
                "a reason or revision feedback is required".into(),
            ));
        }
        let mut repository = self.repository.lock().unwrap();
        let previous = repository.get_candidate(code)?;
        check_request(&previous, user, revision, hash)?;
        if !matches!(
            previous.status,
            SkillCandidateStatus::PendingApproval | SkillCandidateStatus::ValidationFailed
        ) && !(previous.status == SkillCandidateStatus::RevisionRequested && !revise)
        {
            return Err(AssistantMemoryError::Conflict);
        }
        let mut candidate = previous.clone();
        candidate.status = if revise {
            SkillCandidateStatus::RevisionRequested
        } else {
            SkillCandidateStatus::Rejected
        };
        candidate.approved_hash = None;
        candidate.updated_at = now();
        if revise {
            candidate.feedback = Some(text);
        } else {
            candidate.reason = Some(text);
        }
        repository.save_candidate(Some(&previous), &candidate)?;
        drop(repository);
        if !revise {
            self.notify(
                &candidate,
                "SkillRejected",
                format!("Skill {} candidate was rejected", candidate.skill_name),
            )?;
        }
        Ok(candidate)
    }
    pub fn retry_install(
        &self,
        user: &str,
        code: &str,
        revision: u32,
        hash: &str,
    ) -> Result<SkillCandidate, AssistantMemoryError> {
        let _guard = self.installation_lock.lock().unwrap();
        let previous = self.get_candidate(user, code)?;
        check_request(&previous, user, revision, hash)?;
        if previous.status == SkillCandidateStatus::Installed {
            return Ok(previous);
        }
        if !matches!(
            previous.status,
            SkillCandidateStatus::Approved
                | SkillCandidateStatus::InstallFailed
                | SkillCandidateStatus::Installing
        ) || previous.approved_hash.as_deref() != Some(hash)
            || artifact_hash(&previous.files)? != hash
        {
            return Err(AssistantMemoryError::Conflict);
        }
        let mut candidate = previous.clone();
        candidate.status = SkillCandidateStatus::Installing;
        candidate.updated_at = now();
        self.repository
            .lock()
            .unwrap()
            .save_candidate(Some(&previous), &candidate)?;
        let installed = self.skills.install_approved_artifact(
            &candidate.skill_name,
            &candidate.files,
            candidate.base_artifact_hash.as_deref(),
            hash,
        );
        let old = candidate.clone();
        candidate.updated_at = now();
        match installed {
            Ok(()) => {
                candidate.status = SkillCandidateStatus::Installed;
                candidate.last_error = Some(
                    "Skill index pending: Installation committed; retrieval index not confirmed"
                        .into(),
                );
            }
            Err(error) => {
                candidate.status = SkillCandidateStatus::InstallFailed;
                candidate.last_error = Some(error.to_string());
            }
        }
        self.repository
            .lock()
            .unwrap()
            .save_candidate(Some(&old), &candidate)?;
        if candidate.status == SkillCandidateStatus::InstallFailed {
            self.notify(
                &candidate,
                "SkillInstallFailed",
                format!(
                    "Skill {} was approved but installation failed: {}",
                    candidate.skill_name,
                    candidate.last_error.as_deref().unwrap_or("")
                ),
            )?;
        } else {
            self.notify(
                &candidate,
                "SkillInstalled",
                format!(
                    "Skill {} is installed in the Skill center",
                    candidate.skill_name
                ),
            )?;
        }
        Ok(candidate)
    }
}
fn check_request(
    candidate: &SkillCandidate,
    user: &str,
    revision: u32,
    hash: &str,
) -> Result<(), AssistantMemoryError> {
    if candidate.user_code != user {
        return Err(AssistantMemoryError::NotFound);
    }
    if candidate.revision != revision || candidate.artifact_hash != hash {
        return Err(AssistantMemoryError::Conflict);
    }
    Ok(())
}
fn load_creator(
    skills: &SkillRegistryService,
) -> Result<(CreatorReceipt, String, Vec<SkillArtifactFile>), AssistantMemoryError> {
    let creator = skills
        .get_registry_skill("skill-creator")
        .map_err(|_| AssistantMemoryError::Dependency("skill-creator is not installed".into()))?;
    if !creator.available {
        return Err(AssistantMemoryError::Dependency(
            "skill-creator is disabled".into(),
        ));
    }
    let files = read_artifact(std::path::Path::new(&creator.registry_path))?;
    let content_hash = artifact_hash(&files)?;
    // Instructions and receipt must come from the same filesystem snapshot. Registry
    // display metadata may lag behind a user's direct file edit until the next scan.
    let markdown = files
        .iter()
        .find(|file| file.path == "SKILL.md")
        .and_then(|file| super::artifacts::artifact_bytes(file).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| {
            AssistantMemoryError::Dependency("skill-creator has no readable SKILL.md".into())
        })?;
    let frontmatter = markdown
        .strip_prefix("---\n")
        .and_then(|text| text.split_once("\n---"))
        .and_then(|(yaml, _)| serde_yaml::from_str::<serde_yaml::Value>(yaml).ok())
        .ok_or_else(|| {
            AssistantMemoryError::Dependency("skill-creator frontmatter is invalid".into())
        })?;
    if frontmatter["name"].as_str() != Some("skill-creator") {
        return Err(AssistantMemoryError::Dependency(
            "loaded skill-creator artifact has a different name".into(),
        ));
    }
    let version = frontmatter
        .get("version")
        .or_else(|| {
            frontmatter
                .get("metadata")
                .and_then(|metadata| metadata.get("version"))
        })
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_i64().map(|value| value.to_string()))
        })
        .unwrap_or_else(|| content_hash.clone());
    Ok((
        CreatorReceipt {
            skill_name: "skill-creator".into(),
            version,
            content_hash,
        },
        markdown,
        files,
    ))
}
fn installed_artifact(
    skills: &SkillRegistryService,
    name: &str,
) -> Result<Vec<SkillArtifactFile>, AssistantMemoryError> {
    let existing = skills
        .list_registry(RegistrySkillListQuery { search: None })
        .map_err(|e| AssistantMemoryError::Storage(e.to_string()))?
        .into_iter()
        .find(|s| s.skill_name == name);
    match existing {
        None => Ok(Vec::new()),
        Some(skill) if skill.provider_pack_id.is_none() && skill.available => {
            read_artifact(std::path::Path::new(&skill.registry_path))
        }
        Some(_) => Err(AssistantMemoryError::Invalid(
            "built-in or unavailable skills cannot be replaced by experience candidates".into(),
        )),
    }
}
