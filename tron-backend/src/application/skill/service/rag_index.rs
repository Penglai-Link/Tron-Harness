use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use crate::{
    application::rag::{
        RagDeactivateSourceCommand, RagDiagnosticsRepository, RagDocumentSourceRuntime,
        RagEmbeddingRuntime, RagIngestRepository, RagIngestScope, RagRetryIndexRepository,
        RagSearchRepository, RagSearchRuntime, RagService, RagSourceLifecycleRepository,
        RagStrategyRegistry,
    },
    domain::{
        skill::{
            RegistrySkill, SkillError, SkillRagIndexBatchSummary, SkillRagIndexRecord,
            SkillRagIndexStatus,
        },
        workspace::{WorkspaceListQuery, WorkspaceStatus},
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
const SKILL_VECTOR_INGEST_PROFILE_ID: &str = "skill-ingest-v1";
const SKILL_GRAPH_CORPUS_ID: &str = "skill-graph";
const SKILL_GRAPH_INGEST_PROFILE_ID: &str = "skill-graph-ingest-v1";

#[derive(Clone)]
pub struct SkillRagIndexService<R, S, E> {
    skills: SkillRegistryService,
    rag: RagService<R, S, E>,
}

impl<R, S, E> SkillRagIndexService<R, S, E>
where
    R: RagIngestRepository
        + RagSearchRepository
        + RagDiagnosticsRepository
        + RagRetryIndexRepository
        + RagSourceLifecycleRepository
        + Clone,
    S: RagStrategyRegistry,
    E: RagEmbeddingRuntime + RagSearchRuntime + RagDocumentSourceRuntime,
{
    pub fn new(skills: SkillRegistryService, rag: RagService<R, S, E>) -> Self {
        Self { skills, rag }
    }

    pub async fn index_workspace(
        &self,
        workspace_code: &str,
    ) -> Result<SkillRagIndexBatchSummary, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let workspace = self.skills.active_workspace_detail(&workspace_code)?;
        let resolved = self.skills.resolve_workspace_bindings(&workspace)?;
        let registry_skills = self.registry_skill_map()?;
        let existing_states = self.list_index_states(Some(&workspace_code))?;
        let mut handled = BTreeSet::new();
        let mut batch = SkillRagIndexBatchSummary::default();

        for (skill_name, binding) in resolved {
            handled.insert(skill_name.clone());
            if !binding.binding.enabled {
                let summary = self.disable_skill(&workspace_code, &skill_name).await?;
                merge_summary(&mut batch, &summary);
                continue;
            }
            let Some(skill) = registry_skills.get(skill_name.as_str()) else {
                self.deactivate_sources(&workspace_code, &skill_name).await;
                let state = self.failed_state(
                    &workspace_code,
                    &skill_name,
                    true,
                    None,
                    None,
                    "Skill is no longer present in the local registry",
                );
                self.upsert_state(&state)?;
                let summary = state.summary();
                merge_summary(&mut batch, &summary);
                continue;
            };
            let summary = self.index_skill(&workspace_code, skill).await?;
            merge_summary(&mut batch, &summary);
        }

        for state in existing_states {
            if handled.contains(&state.skill_name) {
                continue;
            }
            let summary = self
                .disable_skill(&workspace_code, &state.skill_name)
                .await?;
            merge_summary(&mut batch, &summary);
        }

        Ok(batch)
    }

    pub async fn index_workspace_skill(
        &self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<crate::domain::skill::SkillRagIndexSummary, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
        let skill_name = normalized_required(skill_name).ok_or(SkillError::SkillNameRequired)?;
        let workspace = self.skills.active_workspace_detail(&workspace_code)?;
        let resolved = self.skills.resolve_workspace_bindings(&workspace)?;
        let Some(binding) = resolved.get(skill_name.as_str()) else {
            return self.disable_skill(&workspace_code, &skill_name).await;
        };

        if !binding.binding.enabled {
            return self.disable_skill(&workspace_code, &skill_name).await;
        }

        let registry_skills = self.registry_skill_map()?;
        let Some(skill) = registry_skills.get(skill_name.as_str()) else {
            self.deactivate_sources(&workspace_code, &skill_name).await;
            let state = self.failed_state(
                &workspace_code,
                &skill_name,
                true,
                None,
                None,
                "Skill is no longer present in the local registry",
            );
            self.upsert_state(&state)?;
            return Ok(state.summary());
        };

        self.index_skill(&workspace_code, skill).await
    }

    pub async fn index_all_active_workspaces(
        &self,
    ) -> Result<SkillRagIndexBatchSummary, SkillError> {
        let workspaces = self
            .skills
            .workspaces
            .list_workspace_options(WorkspaceListQuery {
                include_archived: Some(false),
                search: None,
                status: Some(WorkspaceStatus::Active),
                root_only: None,
            })
            .map_err(super::map_workspace_error)?;
        let mut batch = SkillRagIndexBatchSummary::default();
        for workspace in workspaces {
            match self.index_workspace(&workspace.workspace_code).await {
                Ok(summary) => merge_batch(&mut batch, summary),
                Err(error) => {
                    batch.failed_count += 1;
                    batch.warnings.push(format!(
                        "{} indexing failed: {error}",
                        workspace.workspace_code
                    ));
                }
            }
        }
        Ok(batch)
    }

    pub async fn disable_skill(
        &self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<crate::domain::skill::SkillRagIndexSummary, SkillError> {
        let workspace_code =
            normalized_required(workspace_code).ok_or(SkillError::WorkspaceRequired)?;
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
            workspace_code,
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

    pub fn state_summary(
        &self,
        workspace_code: &str,
        skill_name: &str,
    ) -> Result<Option<crate::domain::skill::SkillRagIndexSummary>, SkillError> {
        Ok(self
            .skills
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .get_rag_index_state(workspace_code, skill_name)?
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
        let text = skill_index_text(&skill.skill_name, &description);
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
                SKILL_VECTOR_INGEST_PROFILE_ID,
                skill_payload(workspace_code, &skill.skill_name, &description, &text, None),
            ))
            .await;
        let vector_status = match vector_result {
            Ok(result) if result.embedding_status == "INDEXED" => SkillRagIndexStatus::Indexed,
            Ok(result) => {
                warnings.push(rag_embedding_warning(
                    "skill vector",
                    &result.embedding_status,
                    "skill vector index completed without vector embedding; fulltext search remains available",
                ));
                SkillRagIndexStatus::Warning
            }
            Err(error) => {
                warnings.push(format!("skill vector index failed: {error}"));
                SkillRagIndexStatus::Failed
            }
        };

        let graph = related_graph_payload(workspace_code, &skill.skill_name, &files);
        let graph_result = self
            .rag
            .ingest(skill_ingest_request(
                workspace_code,
                &skill.skill_name,
                SKILL_GRAPH_CORPUS_ID,
                SKILL_GRAPH_INGEST_PROFILE_ID,
                skill_payload(
                    workspace_code,
                    &skill.skill_name,
                    &description,
                    &text,
                    Some(graph),
                ),
            ))
            .await;
        let graph_status = match graph_result {
            Ok(result) if result.embedding_status == "INDEXED" => SkillRagIndexStatus::Indexed,
            Ok(result) => {
                warnings.push(rag_embedding_warning(
                    "skill graph",
                    &result.embedding_status,
                    "skill graph index completed without vector embedding; graph relationships and fulltext search remain available",
                ));
                SkillRagIndexStatus::Warning
            }
            Err(error) => {
                warnings.push(format!("skill graph index failed: {error}"));
                SkillRagIndexStatus::Failed
            }
        };

        let state = SkillRagIndexRecord {
            workspace_code: workspace_code.to_string(),
            skill_name: skill.skill_name.clone(),
            source_version: SKILL_SOURCE_VERSION.into(),
            desired_enabled: true,
            vector_status,
            graph_status,
            content_hash: Some(content_hash),
            structure_hash: Some(structure_hash),
            last_indexed_at: if vector_status == SkillRagIndexStatus::Indexed
                && graph_status == SkillRagIndexStatus::Indexed
            {
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
        for corpus_id in [SKILL_VECTOR_CORPUS_ID, SKILL_GRAPH_CORPUS_ID] {
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

    fn list_index_states(
        &self,
        workspace_code: Option<&str>,
    ) -> Result<Vec<SkillRagIndexRecord>, SkillError> {
        self.skills
            .repository
            .lock()
            .expect("skill registry repository mutex poisoned")
            .list_rag_index_states(workspace_code)
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
        workspace_code: &str,
        skill_name: &str,
        desired_enabled: bool,
        content_hash: Option<String>,
        structure_hash: Option<String>,
        error: &str,
    ) -> SkillRagIndexRecord {
        SkillRagIndexRecord {
            workspace_code: workspace_code.to_string(),
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

fn rag_embedding_warning(
    index_label: &str,
    embedding_status: &str,
    degraded_message: &str,
) -> String {
    if embedding_status == "FAILED" {
        degraded_message.to_string()
    } else {
        format!("{index_label} index completed with embedding status {embedding_status}")
    }
}

fn merge_summary(
    batch: &mut SkillRagIndexBatchSummary,
    summary: &crate::domain::skill::SkillRagIndexSummary,
) {
    if !summary.warnings.is_empty() {
        batch.warnings.extend(summary.warnings.clone());
    }
    if summary.vector_status == SkillRagIndexStatus::Disabled
        && summary.graph_status == SkillRagIndexStatus::Disabled
    {
        batch.disabled_count += 1;
    } else if summary.vector_status == SkillRagIndexStatus::Indexed
        && summary.graph_status == SkillRagIndexStatus::Indexed
    {
        batch.indexed_count += 1;
    } else if summary.vector_status == SkillRagIndexStatus::Failed
        || summary.graph_status == SkillRagIndexStatus::Failed
    {
        batch.failed_count += 1;
    } else {
        batch.warning_count += 1;
    }
}

fn merge_batch(target: &mut SkillRagIndexBatchSummary, source: SkillRagIndexBatchSummary) {
    target.indexed_count += source.indexed_count;
    target.disabled_count += source.disabled_count;
    target.failed_count += source.failed_count;
    target.warning_count += source.warning_count;
    target.warnings.extend(source.warnings);
}
