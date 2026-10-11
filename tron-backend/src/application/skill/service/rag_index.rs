use std::{collections::BTreeMap, path::Path};

use crate::{
    application::rag::{
        RagDeactivateSourceCommand, RagDiagnosticsRepository, RagEmbeddingRuntime,
        RagIngestRepository, RagIngestScope, RagSearchRepository, RagSearchRuntime, RagService,
        RagSourceLifecycleRepository,
    },
    domain::skill::{
        RegistrySkill, SkillError, SkillRagIndexBatchSummary, SkillRagIndexRecord,
        SkillRagIndexStatus,
    },
    shared::time::current_timestamp,
};

use super::{
    SkillRegistryService, list_skill_files, normalized_required, parse_skill_frontmatter,
    rag_index_payload::{
        SKILL_SOURCE_VERSION, related_graph_payload, skill_index_text, skill_ingest_request,
        skill_payload, source_code, stable_hash, stable_structure_hash,
    },
    read_skill_markdown,
};

const SKILL_VECTOR_CORPUS_ID: &str = "skill";

#[derive(Clone)]
pub struct SkillRagIndexService<R, E> {
    skills: SkillRegistryService,
    rag: RagService<R, E>,
}

impl<R, E> SkillRagIndexService<R, E>
where
    R: Send
        + Sync
        + RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagSourceLifecycleRepository
        + Clone,
    E: RagEmbeddingRuntime + crate::application::rag::RagGraphRuntime + RagSearchRuntime,
{
    pub fn new(skills: SkillRegistryService, rag: RagService<R, E>) -> Self {
        Self { skills, rag }
    }

    pub async fn index_all(&self) -> Result<SkillRagIndexBatchSummary, SkillError> {
        let _guard = self.skills.index_lock.lock().await;
        let skills = self.registry_skill_map()?;
        let states = self.list_index_states()?;
        let mut batch = SkillRagIndexBatchSummary::default();
        for skill in skills.values() {
            let summary = if skill.available {
                self.index_skill("PLATFORM", skill).await?
            } else {
                self.disable_skill_inner(&skill.skill_name).await?
            };
            merge_summary(&mut batch, &summary);
        }
        for state in states {
            if !skills.contains_key(&state.skill_name) {
                let summary = self.disable_skill_inner(&state.skill_name).await?;
                merge_summary(&mut batch, &summary);
                if summary.vector_status != SkillRagIndexStatus::Failed
                    && summary.graph_status != SkillRagIndexStatus::Failed
                {
                    self.skills
                        .repository
                        .lock()
                        .unwrap()
                        .delete_rag_index_state(&state.skill_name)?;
                }
            }
        }
        Ok(batch)
    }
    pub async fn index_registry_skill(
        &self,
        name: &str,
    ) -> Result<crate::domain::skill::SkillRagIndexSummary, SkillError> {
        let _guard = self.skills.index_lock.lock().await;
        let skills = self.registry_skill_map()?;
        let skill = skills.get(name).ok_or(SkillError::NotFound)?;
        if !skill.available {
            return self.disable_skill_inner(name).await;
        }
        self.index_skill("PLATFORM", skill).await
    }

    pub async fn disable_skill(
        &self,
        name: &str,
    ) -> Result<crate::domain::skill::SkillRagIndexSummary, SkillError> {
        let _guard = self.skills.index_lock.lock().await;
        self.disable_skill_inner(name).await
    }
    async fn disable_skill_inner(
        &self,
        skill_name: &str,
    ) -> Result<crate::domain::skill::SkillRagIndexSummary, SkillError> {
        let workspace_code = "PLATFORM";
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        let errors = self
            .deactivate_sources(&workspace_code, &skill_name)
            .await
            .into_iter()
            .collect::<Vec<_>>();
        let status = if errors.is_empty() {
            SkillRagIndexStatus::Disabled
        } else {
            SkillRagIndexStatus::Failed
        };
        let state = SkillRagIndexRecord {
            skill_name,
            source_version: SKILL_SOURCE_VERSION.into(),
            desired_enabled: false,
            vector_status: status,
            graph_status: status,
            content_hash: None,
            structure_hash: None,
            last_indexed_at: None,
            last_error: join_errors(errors),
            updated_at: current_timestamp(),
        };
        self.upsert_state(&state)?;
        Ok(state.summary())
    }

    pub async fn remove_skill_index(&self, name: &str) -> Result<(), SkillError> {
        let _guard = self.skills.index_lock.lock().await;
        self.remove_skill_index_inner(name).await
    }
    /// Delete catalog rows under the indexing lock so a concurrent scan cannot
    /// recreate disabled index state between source cleanup and catalog removal.
    pub async fn remove_pack_skills(&self, pack: &str) -> Result<(), SkillError> {
        let _guard = self.skills.index_lock.lock().await;
        let names = self
            .skills
            .repository
            .lock()
            .unwrap()
            .list_registry_skills()?
            .into_iter()
            .filter(|s| s.provider_pack_id.as_deref() == Some(pack))
            .map(|s| s.skill_name)
            .collect::<Vec<_>>();
        for name in names {
            self.remove_skill_index_inner(&name).await?;
        }
        self.skills.delete_pack_skills(pack)?;
        Ok(())
    }
    async fn remove_skill_index_inner(&self, name: &str) -> Result<(), SkillError> {
        let summary = self.disable_skill_inner(name).await?;
        if summary.vector_status == SkillRagIndexStatus::Failed
            || summary.graph_status == SkillRagIndexStatus::Failed
        {
            return Err(SkillError::StorageFailure);
        }
        self.skills
            .repository
            .lock()
            .unwrap()
            .delete_rag_index_state(name)
    }

    pub fn state_summary(
        &self,
        skill_name: &str,
    ) -> Result<Option<crate::domain::skill::SkillRagIndexSummary>, SkillError> {
        Ok(self
            .skills
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .get_rag_index_state(skill_name)?
            .map(|state| state.summary()))
    }

    async fn index_skill(
        &self,
        workspace_code: &str,
        skill: &RegistrySkill,
    ) -> Result<crate::domain::skill::SkillRagIndexSummary, SkillError> {
        let skill_dir = Path::new(&skill.registry_path);
        let markdown = match read_skill_markdown(skill_dir) {
            Ok(markdown) => markdown,
            Err(error) => {
                self.deactivate_sources(workspace_code, &skill.skill_name)
                    .await;
                let state = self.failed_state(
                    workspace_code,
                    &skill.skill_name,
                    true,
                    None,
                    None,
                    &format!("SKILL.md could not be read: {error}"),
                );
                self.upsert_state(&state)?;
                return Ok(state.summary());
            }
        };
        let frontmatter = parse_skill_frontmatter(&markdown);
        let description = frontmatter.description.unwrap_or_default();
        let files = match list_skill_files(skill_dir) {
            Ok(files) => files,
            Err(error) => {
                self.deactivate_sources(workspace_code, &skill.skill_name)
                    .await;
                let state = self.failed_state(
                    workspace_code,
                    &skill.skill_name,
                    true,
                    None,
                    None,
                    &format!("skill files could not be listed: {error}"),
                );
                self.upsert_state(&state)?;
                return Ok(state.summary());
            }
        };
        let text = format!(
            "{}\n\n{}\n\nFiles:\n{}",
            skill_index_text(&skill.skill_name, &description),
            markdown,
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
        let content_hash = stable_hash(&text);
        let structure_hash = stable_structure_hash(&files);
        let mut warnings = Vec::new();
        if description.trim().is_empty() {
            warnings.push("SKILL.md frontmatter description is empty".to_string());
        }

        let vector_result = self
            .rag
            .ingest(skill_ingest_request(
                workspace_code,
                &skill.skill_name,
                SKILL_VECTOR_CORPUS_ID,
                skill_payload(
                    workspace_code,
                    &skill.skill_name,
                    &description,
                    &text,
                    Some(related_graph_payload(
                        workspace_code,
                        &skill.skill_name,
                        &files,
                    )),
                ),
            ))
            .await;
        let settings = self
            .rag
            .settings_for_corpus("skill")
            .map_err(|_| SkillError::StorageFailure)?;
        let published = vector_result.is_ok();
        if let Err(error) = &vector_result {
            warnings.push(format!("skill index failed: {error}"));
        }
        let vector_status = if !published {
            SkillRagIndexStatus::Failed
        } else if !settings.vector {
            SkillRagIndexStatus::Disabled
        } else if published {
            SkillRagIndexStatus::Indexed
        } else {
            SkillRagIndexStatus::Failed
        };
        let graph_status = if !published {
            SkillRagIndexStatus::Failed
        } else if !settings.graph {
            SkillRagIndexStatus::Disabled
        } else if published {
            SkillRagIndexStatus::Indexed
        } else {
            SkillRagIndexStatus::Failed
        };

        let state = SkillRagIndexRecord {
            skill_name: skill.skill_name.clone(),
            source_version: SKILL_SOURCE_VERSION.into(),
            desired_enabled: true,
            vector_status,
            graph_status,
            content_hash: Some(content_hash),
            structure_hash: Some(structure_hash),
            last_indexed_at: if published {
                Some(current_timestamp())
            } else {
                None
            },
            last_error: join_errors(warnings),
            updated_at: current_timestamp(),
        };
        self.upsert_state(&state)?;
        Ok(state.summary())
    }

    async fn deactivate_sources(&self, workspace_code: &str, skill_name: &str) -> Vec<String> {
        let mut errors = Vec::new();
        for corpus_id in [SKILL_VECTOR_CORPUS_ID] {
            let result = self.rag.deactivate_source(RagDeactivateSourceCommand {
                corpus_id: corpus_id.to_string(),
                source_code: source_code(workspace_code, skill_name),
                source_version: SKILL_SOURCE_VERSION.into(),
                workspace_code: workspace_code.to_string(),
                scope: RagIngestScope {
                    scope_type: "SPACE".into(),
                    scope_code: workspace_code.to_string(),
                },
            });
            if let Err(error) = result {
                errors.push(format!("{corpus_id} deactivate failed: {error}"));
            }
        }
        errors
    }

    fn registry_skill_map(&self) -> Result<BTreeMap<String, RegistrySkill>, SkillError> {
        Ok(self
            .skills
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_registry_skills()?
            .into_iter()
            .map(|skill| (skill.skill_name.clone(), skill))
            .collect())
    }

    fn list_index_states(&self) -> Result<Vec<SkillRagIndexRecord>, SkillError> {
        self.skills
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_rag_index_states()
    }

    fn upsert_state(&self, state: &SkillRagIndexRecord) -> Result<(), SkillError> {
        self.skills
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .upsert_rag_index_state(state)
    }

    fn failed_state(
        &self,
        _workspace_code: &str,
        skill_name: &str,
        desired_enabled: bool,
        content_hash: Option<String>,
        structure_hash: Option<String>,
        error: &str,
    ) -> SkillRagIndexRecord {
        SkillRagIndexRecord {
            skill_name: skill_name.to_string(),
            source_version: SKILL_SOURCE_VERSION.into(),
            desired_enabled,
            vector_status: SkillRagIndexStatus::Failed,
            graph_status: SkillRagIndexStatus::Failed,
            content_hash,
            structure_hash,
            last_indexed_at: None,
            last_error: Some(error.to_string()),
            updated_at: current_timestamp(),
        }
    }
}

fn join_errors(errors: Vec<String>) -> Option<String> {
    if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    }
}

fn merge_summary(
    batch: &mut SkillRagIndexBatchSummary,
    summary: &crate::domain::skill::SkillRagIndexSummary,
) {
    if !summary.warnings.is_empty() {
        batch.warnings.extend(summary.warnings.clone());
    }
    if !summary.desired_enabled {
        batch.disabled_count += 1;
    } else if matches!(
        summary.vector_status,
        SkillRagIndexStatus::Indexed | SkillRagIndexStatus::Disabled
    ) && matches!(
        summary.graph_status,
        SkillRagIndexStatus::Indexed | SkillRagIndexStatus::Disabled
    ) {
        batch.indexed_count += 1;
    } else if summary.vector_status == SkillRagIndexStatus::Failed
        || summary.graph_status == SkillRagIndexStatus::Failed
    {
        batch.failed_count += 1;
    } else {
        batch.warning_count += 1;
    }
}
