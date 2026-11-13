//! Business authorization supplies a source snapshot; the RAG service owns execution.
use super::*;
use crate::domain::rag::RagError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct RagIdentity {
    pub user_code: String,
    pub workspace_code: String,
    pub agent_code: String,
}
pub trait RagAccessResolver: Send + Sync {
    fn resolve(&self, identity: &RagIdentity) -> Result<Vec<AuthorizedRagRange>, RagError>;
    fn recheck(
        &self,
        identity: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError>;
    /// Authorization only. Graph provenance must not trigger parent-context assembly.
    fn recheck_sources(
        &self,
        identity: &RagIdentity,
        result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        self.recheck(identity, result)
    }
    fn assemble_context(
        &self,
        _identity: &RagIdentity,
        _result: &mut RagEvidenceResult,
    ) -> Result<(), RagError> {
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RagQueryInput {
    pub query: String,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(default = "empty_filters")]
    pub filters: Value,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
fn empty_filters() -> Value {
    json!({})
}
fn default_limit() -> usize {
    8
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagEvidenceView {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub chunk_code: String,
    pub title: Option<String>,
    pub text: String,
    pub location: Value,
    pub source: Value,
    pub channels: Vec<String>,
    pub score: f64,
    pub parts: Vec<RagEvidencePartView>,
    pub images: Vec<RagEvidenceImageView>,
    pub associations: Vec<RagEvidenceAssociationView>,
    pub truncated_by: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagQueryResult {
    pub query: String,
    pub authorized_corpus_ids: Vec<String>,
    pub stages: Vec<RagRetrievalStage>,
    pub warnings: Vec<String>,
    pub evidence: Vec<RagEvidenceView>,
    pub graph_context: Vec<super::graph::GraphEvidenceView>,
    pub duration_ms: u64,
}
impl<R: RagSearchRepository + Clone + Send + Sync, E: RagSearchRuntime> RagService<R, E> {
    pub async fn search(
        &self,
        access: &dyn RagAccessResolver,
        identity: &RagIdentity,
        input: RagQueryInput,
        run: Option<&eliteforge_agent_framework::RunContext>,
    ) -> Result<RagQueryResult, RagError> {
        if input.query.trim().is_empty()
            || input.query.len() > 16000
            || !(1..=100).contains(&input.limit)
            || !input.filters.is_object()
        {
            return Err(RagError::SearchRequestInvalid);
        }
        let combined = crate::application::memory::rag_access::MemoryRagAccess {
            base: access,
            rag: self,
            store: self.memories.as_deref(),
        };
        let access: &dyn RagAccessResolver = &combined;
        let started = std::time::Instant::now();
        let mut ranges = access.resolve(identity)?;
        let authorized_corpus_ids = ranges
            .iter()
            .map(|range| range.corpus_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        // A requested corpus may narrow the source snapshot, never grant access.
        if input
            .corpus_ids
            .iter()
            .any(|id| !authorized_corpus_ids.contains(id))
        {
            return Err(RagError::SearchScopeInvalid);
        }
        ranges.retain(|range| {
            input.corpus_ids.is_empty() || input.corpus_ids.contains(&range.corpus_id)
        });
        let metadata = self.repository.get_corpora(
            &ranges
                .iter()
                .map(|range| range.corpus_id.clone())
                .collect::<Vec<_>>(),
        )?;
        let corpora = ranges
            .iter()
            .map(|range| {
                metadata
                    .get(&range.corpus_id)
                    .ok_or(RagError::CorpusNotFound)
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_request_filters_for_corpora(&corpora, &input.filters)?;
        let mut rerank = None;
        for (range, corpus) in ranges.iter_mut().zip(&corpora) {
            range.filters = merge_search_request_filters(corpus, &range.filters, &input.filters)?;
            let settings = RagSettings::from_capabilities(&corpus.capabilities)?;
            if rerank.is_none() {
                rerank = settings.rerank_model_code;
            }
        }
        let model_workspace = if ranges.iter().all(|r| r.corpus_id == "memory-card") {
            super::MEMORY_MODEL_WORKSPACE
        } else {
            &identity.workspace_code
        };
        let mut result = self
            .recall_authorized(model_workspace, &input.query, &ranges, input.limit, run)
            .await?;
        let recalled_count = result.hits.len();
        self.recheck_published(&ranges, &mut result)?;
        access.recheck_sources(identity, &mut result)?;
        if result.hits.len() != recalled_count {
            invalidate_graph_statistics(&mut result);
        }
        self.assemble_source_bundles(&ranges, &mut result);
        self.recheck_evidence_members(access, identity, &ranges, &mut result)?;
        access.assemble_context(identity, &mut result)?;
        let mut result = self
            .rank_evidence(
                model_workspace,
                &input.query,
                result,
                input.limit,
                rerank.as_deref(),
            )
            .await?;
        let before_recheck = result
            .hits
            .iter()
            .map(|hit| {
                (
                    hit.row.corpus_id.clone(),
                    hit.row.source_code.clone(),
                    hit.row.source_version.clone(),
                    hit.row.scope_type.clone(),
                    hit.row.scope_code.clone(),
                    hit.row.target_code.clone(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        self.recheck_evidence_members(access, identity, &ranges, &mut result)?;
        let after_recheck = result
            .hits
            .iter()
            .map(|hit| {
                (
                    hit.row.corpus_id.clone(),
                    hit.row.source_code.clone(),
                    hit.row.source_version.clone(),
                    hit.row.scope_type.clone(),
                    hit.row.scope_code.clone(),
                    hit.row.target_code.clone(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        if !before_recheck.is_subset(&after_recheck) {
            // Counts must not disclose facts whose source authorization changed.
            invalidate_graph_statistics(&mut result);
        }
        // Business grants may have changed while the model was running. Resolve
        // them again before exposing aliases, relationship quotes or node counts.
        let mut current_ranges = access.resolve(identity)?;
        current_ranges.retain(|range| {
            input.corpus_ids.is_empty() || input.corpus_ids.contains(&range.corpus_id)
        });
        let corpora = self.repository.get_corpora(
            &current_ranges
                .iter()
                .map(|range| range.corpus_id.clone())
                .collect::<Vec<_>>(),
        )?;
        for range in &mut current_ranges {
            let corpus = corpora
                .get(&range.corpus_id)
                .ok_or(RagError::CorpusNotFound)?;
            range.filters = merge_search_request_filters(&corpus, &range.filters, &input.filters)?;
        }
        self.recheck_graph_context(&current_ranges, &mut result)?;
        super::graph::retain_returned_evidence(&mut result);
        let graph_context = match super::graph::evidence_graph_view(&result) {
            Ok(view) => view,
            Err(_) => {
                result
                    .warnings
                    .push("图谱原文引用已改变，相关图谱展示已撤销".into());
                vec![]
            }
        };
        if result.hits.is_empty() {
            result
                .warnings
                .push("当前授权及已发布范围内没有命中证据".into());
        }
        let reranked = result
            .stages
            .iter()
            .any(|stage| stage.channel == "rerank" && stage.status == "COMPLETED");
        let evidence = result
            .hits
            .into_iter()
            .map(|hit| {
                let text = hit.rerank_text();
                let (parts, images, associations, truncated_by) =
                    super::evidence::public_evidence(&hit);
                let mut source = hit.row.text_metadata.clone();
                if let Some(object) = source.as_object_mut() {
                    object.remove("matchedText");
                    object.remove("matchedLocation");
                }
                if let Some(primary) = parts
                    .iter()
                    .find(|part| part.chunk_code == hit.row.target_code)
                    .or_else(|| parts.first())
                {
                    source["location"] = primary.location.clone();
                    source["sourceLocations"] = serde_json::json!(primary.source_locations);
                    source["contextLocations"] = serde_json::json!(primary.source_locations);
                    source["imageAnnotations"] = serde_json::json!(
                        primary
                            .source_locations
                            .iter()
                            .flat_map(|location| location["imageAnnotations"]
                                .as_array()
                                .into_iter()
                                .flatten())
                            .collect::<Vec<_>>()
                    );
                }
                if let Some(spans) = super::source_parts::selected_spans(
                    &hit.row.text_metadata,
                    &hit.selected_blocks,
                ) {
                    source["sourceSpans"] = serde_json::json!(spans);
                }
                source["selectedSourceBlockIds"] = serde_json::json!(hit.selected_blocks);
                source["publication"] = hit.row.source_metadata.clone();
                source["relevance"] = serde_json::json!({
                    "vectorSimilarity": hit.vector_similarity,
                    "rerankScore": reranked.then_some(hit.score),
                });
                let location = source["location"].clone();
                let row = hit.row;
                let published_version = row.source_metadata["indexTaskId"]
                    .as_str()
                    .unwrap_or(&row.source_code)
                    .to_owned();
                RagEvidenceView {
                    corpus_id: row.corpus_id,
                    source_code: row.source_code,
                    source_version: published_version,
                    chunk_code: row.target_code,
                    title: row.title,
                    text,
                    location,
                    source,
                    channels: hit.channels.into_iter().collect(),
                    score: hit.score,
                    parts,
                    images,
                    associations,
                    truncated_by,
                }
            })
            .collect();
        Ok(RagQueryResult {
            query: input.query,
            authorized_corpus_ids: current_ranges
                .iter()
                .map(|range| range.corpus_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            stages: result.stages,
            warnings: result.warnings,
            evidence,
            graph_context,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

pub(super) fn invalidate_graph_statistics(result: &mut RagEvidenceResult) {
    for stage in result
        .stages
        .iter_mut()
        .filter(|stage| stage.channel.starts_with("graph") || stage.channel == "source.structure")
    {
        stage.hit_count = 0;
        stage.status = "INVALIDATED".into();
        stage.error = Some("来源授权或发布状态已改变，图谱统计已撤销".into());
    }
}
