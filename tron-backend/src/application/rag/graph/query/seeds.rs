//! Global, authorized node candidates. Raw distances from different spaces never mix.
use super::*;
use crate::application::rag::graph::recall::QueryEmbeddingCache;
use crate::application::rag::{
    RagEmbeddingCall, RagSearchRepository, RagSearchRuntime, RagService,
};
use serde_json::json;
use std::{collections::BTreeMap, time::Instant};

fn fuse(seeds: &mut BTreeMap<String, GraphRecallSeed>, page: GraphNodeSearchPage, channel: &str) {
    for (rank, hit) in page.hits.into_iter().enumerate() {
        let seed = seeds
            .entry(hit.entity_id.clone())
            .or_insert(GraphRecallSeed {
                id: hit.entity_id,
                score: 0.,
                channels: BTreeSet::new(),
            });
        seed.score += 1. / (60. + rank as f64 + 1.);
        seed.channels.insert(channel.into());
    }
}
pub(super) fn stage(
    snapshot: &AuthorizedGraphSnapshot,
    channel: &str,
    start: Instant,
    count: usize,
    error: Option<String>,
    truncated: bool,
) -> RagRetrievalStage {
    RagRetrievalStage {
        corpus_id: snapshot.corpus_label(),
        channel: channel.into(),
        status: if error.is_some() {
            "FAILED"
        } else if truncated {
            "TRUNCATED"
        } else {
            "COMPLETED"
        }
        .into(),
        hit_count: count,
        duration_ms: start.elapsed().as_millis().min(u64::MAX as u128) as u64,
        error,
    }
}
impl<R: RagSearchRepository + Clone, E: RagSearchRuntime> RagService<R, E> {
    pub(super) async fn graph_node_seeds(
        &self,
        workspace: &str,
        query: &str,
        snapshot: &AuthorizedGraphSnapshot,
        cache: &mut QueryEmbeddingCache,
        run: Option<&eliteforge_agent_framework::RunContext>,
    ) -> Result<NodeRecall, GraphQueryError> {
        let mut result = NodeRecall::default();
        let manifests = snapshot
            .manifests
            .iter()
            .filter(|g| g.expected_projections > 0)
            .collect::<Vec<_>>();
        if manifests.is_empty() {
            return Ok(result);
        }
        let infra = self
            .graph
            .as_ref()
            .ok_or_else(|| GraphQueryError::Unavailable("graph infrastructure".into()))?;
        let mut seeds = BTreeMap::new();
        let started = Instant::now();
        match infra.index.search_nodes(GraphNodeSearchQuery {
            generation_ids: manifests.iter().map(|g| g.id.clone()).collect(),
            matching: GraphNodeMatch::Fulltext(query.into()),
            limit: 100,
        }) {
            Ok(page) => {
                result.stages.push(stage(
                    snapshot,
                    "graph.fulltext",
                    started,
                    page.hits.len(),
                    None,
                    page.has_more,
                ));
                fuse(&mut seeds, page, "fulltext");
            }
            Err(error) => {
                result.stages.push(stage(
                    snapshot,
                    "graph.fulltext",
                    started,
                    0,
                    Some(error.clone()),
                    false,
                ));
                result.warnings.push(error);
            }
        }
        let mut spaces = BTreeMap::<(String, String, Option<String>, usize), Vec<String>>::new();
        for g in manifests {
            spaces
                .entry((
                    g.model_code.clone(),
                    g.model_fingerprint.clone(),
                    g.provider_model.clone(),
                    g.dimension,
                ))
                .or_default()
                .push(g.id.clone());
        }
        for ((model, fingerprint, provider, dimension), generations) in spaces {
            let started = Instant::now();
            let recalled: Result<GraphNodeSearchPage, String> = async {
                let before =
                    self.runtime
                        .embedding_fingerprint(workspace, Some(&model), dimension)?;
                if before.as_deref() != Some(&fingerprint) {
                    return Err("graph query model differs from published node space".into());
                }
                let key = (
                    Some(model.clone()),
                    Some(dimension),
                    provider.clone(),
                    Some(fingerprint.clone()),
                );
                if !cache.contains_key(&key) {
                    let embedded = self
                        .runtime
                        .embed_for_retrieval(
                            RagEmbeddingCall {
                                workspace_code: workspace.into(),
                                model_code: Some(model.clone()),
                                input: query.into(),
                            },
                            Some(dimension),
                            run,
                        )
                        .await
                        .and_then(|value| {
                            if value.model_code != model
                                || value.provider_model != provider
                                || value.embedding.len() != dimension
                                || value.embedding.iter().any(|v| !v.is_finite())
                            {
                                for range in &snapshot.ranges {
                                    let mut query = source_query(range);
                                    query.policy = json!({"graphGenerationIds":generations});
                                    self.repository
                                        .invalidate_vector_cache(query)
                                        .map_err(|e| e.to_string())?;
                                }
                                Err(
                                    "graph query vector identity differs from published node space"
                                        .into(),
                                )
                            } else {
                                Ok(value.embedding)
                            }
                        });
                    cache.insert(key.clone(), embedded);
                }
                let embedding = cache
                    .get(&key)
                    .ok_or("graph query vector is absent")?
                    .clone()?;
                if self
                    .runtime
                    .embedding_fingerprint(workspace, Some(&model), dimension)?
                    != before
                {
                    return Err("graph query model changed during retrieval".into());
                }
                infra.index.search_nodes(GraphNodeSearchQuery {
                    generation_ids: generations,
                    matching: GraphNodeMatch::Vector {
                        embedding,
                        model_code: model,
                        model_fingerprint: fingerprint,
                        provider_model: provider,
                    },
                    limit: 100,
                })
            }
            .await;
            match recalled {
                Ok(page) => {
                    if page.candidate_limit_reached {
                        result.warnings.push("NodeVectorCandidateBudget".into());
                    }
                    result.stages.push(stage(
                        snapshot,
                        "graph.vector",
                        started,
                        page.hits.len(),
                        None,
                        page.has_more || page.candidate_limit_reached,
                    ));
                    fuse(&mut seeds, page, "vector");
                }
                Err(error) => {
                    result.stages.push(stage(
                        snapshot,
                        "graph.vector",
                        started,
                        0,
                        Some(error.clone()),
                        false,
                    ));
                    result.warnings.push(error);
                }
            }
        }
        result.seeds = seeds.into_values().collect();
        result
            .seeds
            .sort_by(|a, b| b.score.total_cmp(&a.score).then(a.id.cmp(&b.id)));
        Ok(result)
    }
}
