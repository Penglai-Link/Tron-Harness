use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::domain::rag::RagError;

use super::{
    CandidateHit, CompiledRetrievalProfile, ContextPack, ContextPackItem, RagRerankCall,
    RagSearchRequest, RagSearchRow, RagSearchRuntime, RetrievalRoute,
    search::{CandidateRouteRank, RankedCandidate},
    search_profile::{DedupPolicy, EmbeddingMmrPolicy, FusionPolicy, RerankStrategy, TopicPolicy},
    search_strategy::{RetrieverKind, RetrieverStrategy},
};

const PERSON_AI_LLM_FALLBACK_ROUTE: &str = "PERSON_AI:ROUTE:LlmFallback";

pub(super) fn rank_route_rows(
    rows: Vec<RagSearchRow>,
    route: &RetrievalRoute,
    strategy: &RetrieverStrategy,
    query: &str,
    query_embedding: Option<&[f32]>,
) -> Vec<RankedCandidate> {
    let mut hits = rows
        .into_iter()
        .filter_map(|row| {
            let score = row.storage_score.unwrap_or_else(|| match strategy.kind {
                RetrieverKind::Fulltext if fulltext_matches(&row.search_text, query) => {
                    fulltext_score(&row.search_text, query)
                }
                RetrieverKind::Vector => row
                    .embedding
                    .as_deref()
                    .and_then(|embedding| {
                        query_embedding.map(|query| cosine_similarity(query, embedding))
                    })
                    .unwrap_or(0.0),
                RetrieverKind::StructuredFilter => 1.0,
                _ => 0.0,
            });
            (score > 0.0).then(|| {
                let mut hit = hit_from_row(&row);
                hit.result.score = score * route.weight;
                hit
            })
        })
        .collect::<Vec<_>>();
    hits.sort_by(stable_hit_order);
    hits.truncate(route.top_k);
    for (index, hit) in hits.iter_mut().enumerate() {
        hit.route_ids.push(route.route_id.clone());
        hit.route_ranks.push(CandidateRouteRank {
            route_id: route.route_id.clone(),
            rank: index + 1,
            weight: route.weight,
        });
    }
    hits
}

pub(super) fn fuse_and_dedup(
    compiled: &CompiledRetrievalProfile,
    hits: Vec<RankedCandidate>,
) -> Vec<RankedCandidate> {
    let mut by_target = BTreeMap::<String, RankedCandidate>::new();
    for hit in hits {
        let dedup_key = dedup_key(&compiled.dedup_policy, &hit);
        match by_target.entry(dedup_key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(hit);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                merge_candidate(entry.get_mut(), hit, &compiled.fusion_policy);
            }
        }
    }
    let mut values = by_target.into_values().collect::<Vec<_>>();
    if let FusionPolicy::Rrf { rank_constant } = compiled.fusion_policy {
        for hit in &mut values {
            hit.result.score = rrf_score(hit, rank_constant);
        }
    }
    values
}

pub(super) fn apply_topic_policy(
    compiled: &CompiledRetrievalProfile,
    hits: &mut Vec<RankedCandidate>,
) {
    match &compiled.topic_policy {
        TopicPolicy::Disabled => {}
        TopicPolicy::EmbeddingMmr(policy) => apply_embedding_mmr(policy, hits),
    }
}

pub(super) async fn apply_rerank<E>(
    runtime: &E,
    compiled: &CompiledRetrievalProfile,
    request: &RagSearchRequest,
    query: &str,
    hits: &mut Vec<RankedCandidate>,
) -> Result<(), RagError>
where
    E: RagSearchRuntime,
{
    if compiled.rerank_strategy == RerankStrategy::Disabled {
        return Ok(());
    }
    let documents = hits
        .iter()
        .map(|hit| hit.result.text.clone())
        .collect::<Vec<_>>();
    let order = runtime
        .rerank(RagRerankCall {
            workspace_code: request.workspace_code.clone(),
            model_code: match compiled.rerank_strategy {
                RerankStrategy::Disabled => return Ok(()),
                RerankStrategy::Native => compiled.rerank_model_code.clone(),
                RerankStrategy::LlmFallback => Some(PERSON_AI_LLM_FALLBACK_ROUTE.into()),
            },
            query: query.to_string(),
            documents,
            top_n: Some(request.limit),
        })
        .await;
    let order = match order {
        Ok(order) => order,
        // LLM fallback is opportunistic: invalid JSON, timeout, or an upstream
        // failure preserves the existing embedding/fusion ranking.
        Err(_) if compiled.rerank_strategy == RerankStrategy::LlmFallback => return Ok(()),
        Err(_) => return Err(RagError::RerankFailed),
    };
    if order.is_empty() {
        return Ok(());
    }
    let mut ranked = Vec::new();
    for (index, score) in order {
        if let Some(mut hit) = hits.get(index).cloned() {
            hit.result.score = score;
            ranked.push(hit);
        }
    }
    if !ranked.is_empty() {
        *hits = ranked;
    }
    Ok(())
}

pub(super) fn stable_hit_order(
    left: &RankedCandidate,
    right: &RankedCandidate,
) -> std::cmp::Ordering {
    right
        .result
        .score
        .partial_cmp(&left.result.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.result.source_code.cmp(&right.result.source_code))
        .then_with(|| left.target_code.cmp(&right.target_code))
}

pub(super) fn assemble_context_pack(
    strategy: &str,
    hits: &[RankedCandidate],
    token_budget: usize,
) -> Option<ContextPack> {
    if strategy == "RESULT_LIST" {
        return None;
    }
    let mut token_count = 0usize;
    let mut items = Vec::new();
    for hit in hits {
        let tokens =
            (hit.token_count.max(1) as usize).min(token_budget.saturating_sub(token_count));
        if tokens == 0 {
            break;
        }
        token_count += tokens;
        items.push(ContextPackItem {
            source_code: hit.result.source_code.clone(),
            text: hit.result.text.clone(),
            tokens,
            metadata: context_pack_metadata(strategy, &hit.result),
        });
    }
    Some(ContextPack { items, token_count })
}

fn context_pack_metadata(strategy: &str, hit: &CandidateHit) -> Value {
    let mut metadata = Map::new();
    metadata.insert("strategy".into(), json!(strategy));
    metadata.insert("corpusId".into(), json!(hit.corpus_id.clone()));
    metadata.insert("sourceVersion".into(), json!(hit.source_version.clone()));
    metadata.insert("title".into(), json!(hit.title.clone()));
    metadata.insert("metadata".into(), hit.metadata.clone());
    metadata.insert("returnFields".into(), Value::Object(hit.fields.clone()));
    Value::Object(metadata)
}

fn fulltext_matches(text: &str, query: &str) -> bool {
    let text = text.to_ascii_lowercase();
    query
        .split_whitespace()
        .any(|term| text.contains(&term.to_ascii_lowercase()))
}

fn fulltext_score(text: &str, query: &str) -> f64 {
    query
        .split_whitespace()
        .filter(|term| {
            text.to_ascii_lowercase()
                .contains(&term.to_ascii_lowercase())
        })
        .count() as f64
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    let (mut dot, mut left_norm, mut right_norm) = (0.0f64, 0.0f64, 0.0f64);
    for (left, right) in left.iter().zip(right.iter()) {
        let left = *left as f64;
        let right = *right as f64;
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn dedup_key(policy: &DedupPolicy, hit: &RankedCandidate) -> String {
    match policy {
        DedupPolicy::TextUnit => hit.identity_key.clone(),
        DedupPolicy::CanonicalTextUnit => hit.canonical_dedup_key.clone(),
    }
}

fn merge_candidate(
    existing: &mut RankedCandidate,
    hit: RankedCandidate,
    fusion_policy: &FusionPolicy,
) {
    let existing_score = existing.result.score;
    let hit_score = hit.result.score;
    let replace_representative = should_replace_representative(existing, &hit);
    if replace_representative {
        let previous = existing.clone();
        *existing = hit;
        merge_candidate_metadata(existing, &previous);
    } else {
        merge_candidate_metadata(existing, &hit);
    }
    existing.result.score = match fusion_policy {
        FusionPolicy::SumScore => existing_score + hit_score,
        FusionPolicy::Rrf { .. } => existing.result.score.max(existing_score).max(hit_score),
    };
}

fn merge_candidate_metadata(existing: &mut RankedCandidate, hit: &RankedCandidate) {
    for route_id in &hit.route_ids {
        if !existing.route_ids.contains(route_id) {
            existing.route_ids.push(route_id.clone());
        }
    }
    for route_rank in &hit.route_ranks {
        merge_route_rank(&mut existing.route_ranks, route_rank.clone());
    }
    if existing.embedding.is_none() {
        existing.embedding = hit.embedding.clone();
    }
}

fn merge_route_rank(route_ranks: &mut Vec<CandidateRouteRank>, incoming: CandidateRouteRank) {
    if let Some(existing) = route_ranks
        .iter_mut()
        .find(|rank| rank.route_id == incoming.route_id)
    {
        if incoming.rank < existing.rank {
            *existing = incoming;
        }
    } else {
        route_ranks.push(incoming);
    }
}

fn should_replace_representative(existing: &RankedCandidate, hit: &RankedCandidate) -> bool {
    hit.result
        .score
        .partial_cmp(&existing.result.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| hit.token_count.cmp(&existing.token_count))
        .then_with(|| existing.result.source_code.cmp(&hit.result.source_code))
        .then_with(|| existing.target_code.cmp(&hit.target_code))
        == std::cmp::Ordering::Greater
}

fn rrf_score(hit: &RankedCandidate, rank_constant: f64) -> f64 {
    hit.route_ranks
        .iter()
        .map(|rank| rank.weight / (rank_constant + rank.rank as f64))
        .sum()
}

fn apply_embedding_mmr(policy: &EmbeddingMmrPolicy, hits: &mut Vec<RankedCandidate>) {
    let window = policy.candidate_window.min(hits.len());
    if window <= 1 || hits[..window].iter().any(|hit| hit.embedding.is_none()) {
        return;
    }
    let max_score = hits[..window]
        .iter()
        .map(|hit| hit.result.score)
        .fold(0.0_f64, f64::max);
    if max_score <= 0.0 {
        return;
    }
    let mut selected = Vec::<usize>::new();
    let mut remaining = (0..window).collect::<Vec<_>>();
    while !remaining.is_empty() {
        let next = best_mmr_candidate(policy, hits, &remaining, &selected, max_score, true)
            .or_else(|| best_mmr_candidate(policy, hits, &remaining, &selected, max_score, false));
        let Some(next) = next else {
            break;
        };
        selected.push(next);
        remaining.retain(|index| *index != next);
    }
    if selected.len() != window {
        return;
    }
    let mut reordered = selected
        .into_iter()
        .map(|index| hits[index].clone())
        .collect::<Vec<_>>();
    reordered.extend(hits[window..].iter().cloned());
    *hits = reordered;
}

fn best_mmr_candidate(
    policy: &EmbeddingMmrPolicy,
    hits: &[RankedCandidate],
    remaining: &[usize],
    selected: &[usize],
    max_score: f64,
    enforce_topic_cap: bool,
) -> Option<usize> {
    let mut best = None;
    let mut best_score = f64::NEG_INFINITY;
    for index in remaining {
        if enforce_topic_cap && exceeds_topic_cap(policy, hits, *index, selected) {
            continue;
        }
        let score = mmr_score(policy, hits, *index, selected, max_score);
        if best.is_none()
            || score > best_score
            || (score == best_score
                && stable_hit_order(&hits[*index], &hits[best.unwrap()])
                    == std::cmp::Ordering::Less)
        {
            best = Some(*index);
            best_score = score;
        }
    }
    best
}

fn mmr_score(
    policy: &EmbeddingMmrPolicy,
    hits: &[RankedCandidate],
    index: usize,
    selected: &[usize],
    max_score: f64,
) -> f64 {
    let relevance = hits[index].result.score / max_score;
    let max_similarity = selected
        .iter()
        .map(|selected_index| candidate_similarity(&hits[index], &hits[*selected_index]))
        .fold(0.0_f64, f64::max);
    policy.lambda * relevance - (1.0 - policy.lambda) * max_similarity
}

fn exceeds_topic_cap(
    policy: &EmbeddingMmrPolicy,
    hits: &[RankedCandidate],
    index: usize,
    selected: &[usize],
) -> bool {
    policy.max_per_topic > 0
        && selected
            .iter()
            .filter(|selected_index| {
                candidate_similarity(&hits[index], &hits[**selected_index])
                    >= policy.similarity_threshold
            })
            .count()
            >= policy.max_per_topic
}

fn candidate_similarity(left: &RankedCandidate, right: &RankedCandidate) -> f64 {
    match (left.embedding.as_deref(), right.embedding.as_deref()) {
        (Some(left), Some(right)) => cosine_similarity(left, right),
        _ => 0.0,
    }
}

fn hit_from_row(row: &RagSearchRow) -> RankedCandidate {
    let identity_key = format!(
        "{}:{}:{}:{}:{}:{}",
        row.corpus_id,
        row.source_code,
        row.source_version,
        row.scope_type,
        row.scope_code,
        row.target_code
    );
    let canonical_dedup_key = normalized_text(&row.text)
        .map(|text| {
            format!(
                "text:{}:{}:{}:{}:{}",
                row.corpus_id, row.source_version, row.scope_type, row.scope_code, text
            )
        })
        .unwrap_or_else(|| identity_key.clone());
    RankedCandidate {
        result: CandidateHit {
            corpus_id: row.corpus_id.clone(),
            source_code: row.source_code.clone(),
            source_version: row.source_version.clone(),
            scope_type: row.scope_type.clone(),
            scope_code: row.scope_code.clone(),
            title: row.title.clone(),
            score: 0.0,
            text: row.text.clone(),
            metadata: row.source_metadata.clone(),
            fields: row.field_values.clone(),
        },
        target_code: row.target_code.clone(),
        scope_type: row.scope_type.clone(),
        scope_code: row.scope_code.clone(),
        route_ids: Vec::new(),
        route_ranks: Vec::new(),
        token_count: row.token_count,
        embedding: row.embedding.clone(),
        identity_key,
        canonical_dedup_key,
    }
}

fn normalized_text(text: &str) -> Option<String> {
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
    };

    use serde_json::{Map, json};

    use crate::application::rag::{
        RagEmbeddingCall, RagEmbeddingResult, RagIngestAccessContext, RagSearchContextRequest,
        RagSearchGraphExpansionRequest, RagSearchTraceRequest,
    };

    use super::*;

    #[derive(Clone)]
    struct RecordingRerankRuntime {
        result: Result<Vec<(usize, f64)>, String>,
        calls: Arc<Mutex<Vec<RagRerankCall>>>,
    }

    impl RagSearchRuntime for RecordingRerankRuntime {
        fn embed<'a>(
            &'a self,
            _call: RagEmbeddingCall,
        ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
            Box::pin(async { Err("embedding should not be called in rerank tests".into()) })
        }

        fn rerank<'a>(
            &'a self,
            call: RagRerankCall,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
            self.calls.lock().expect("record rerank call").push(call);
            let result = self.result.clone();
            Box::pin(async move { result })
        }
    }

    fn compiled_profile(rerank_strategy: RerankStrategy) -> CompiledRetrievalProfile {
        CompiledRetrievalProfile {
            profile_id: "profile-1".into(),
            corpus_id: "corpus-1".into(),
            corpus_ids: vec!["corpus-1".into()],
            required_source_version: "v1".into(),
            pipeline: None,
            routes: Vec::new(),
            supported_context_strategies: vec!["RESULT_LIST".into()],
            embedding_model_code: Some("embedding-1".into()),
            rerank_model_code: Some("native-rerank-1".into()),
            rerank_enabled: true,
            rerank_strategy,
            fusion_policy: FusionPolicy::SumScore,
            dedup_policy: DedupPolicy::TextUnit,
            topic_policy: TopicPolicy::Disabled,
            graph_expansion_policy: None,
            degrade_policy: "FAIL_CLOSED".into(),
        }
    }

    fn search_request() -> RagSearchRequest {
        RagSearchRequest {
            profile_id: "profile-1".into(),
            workspace_code: "workspace-1".into(),
            query: "which document is relevant".into(),
            access_context: RagIngestAccessContext {
                user_code: "user-1".into(),
                agent_code: "agent-1".into(),
                workspace_code: "workspace-1".into(),
            },
            filters: json!({}),
            limit: 10,
            token_budget: 1_000,
            context: RagSearchContextRequest {
                strategy: "RESULT_LIST".into(),
                return_fields: Vec::new(),
                graph_expansion: RagSearchGraphExpansionRequest::default(),
            },
            trace: RagSearchTraceRequest::default(),
        }
    }

    fn ranked_hit(source_code: &str, text: &str, score: f64) -> RankedCandidate {
        RankedCandidate {
            result: CandidateHit {
                corpus_id: "corpus-1".into(),
                source_code: source_code.into(),
                source_version: "v1".into(),
                scope_type: "Workspace".into(),
                scope_code: "workspace-1".into(),
                title: None,
                score,
                text: text.into(),
                metadata: json!({}),
                fields: Map::new(),
            },
            target_code: format!("text-unit-{source_code}"),
            scope_type: "Workspace".into(),
            scope_code: "workspace-1".into(),
            route_ids: Vec::new(),
            route_ranks: Vec::new(),
            token_count: 10,
            embedding: None,
            identity_key: source_code.into(),
            canonical_dedup_key: source_code.into(),
        }
    }

    #[tokio::test]
    async fn llm_fallback_rerank_uses_text_default_route_and_applies_valid_order() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runtime = RecordingRerankRuntime {
            result: Ok(vec![(1, 0.99), (0, 0.42)]),
            calls: calls.clone(),
        };
        let mut hits = vec![
            ranked_hit("source-a", "first candidate", 0.91),
            ranked_hit("source-b", "second candidate", 0.52),
        ];

        apply_rerank(
            &runtime,
            &compiled_profile(RerankStrategy::LlmFallback),
            &search_request(),
            "which document is relevant",
            &mut hits,
        )
        .await
        .expect("LLM fallback rerank succeeds");

        assert_eq!(
            hits.iter()
                .map(|hit| hit.result.source_code.as_str())
                .collect::<Vec<_>>(),
            vec!["source-b", "source-a"]
        );
        assert_eq!(hits[0].result.score, 0.99);
        assert_eq!(hits[1].result.score, 0.42);
        let calls = calls.lock().expect("read rerank calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].model_code.as_deref(),
            Some(PERSON_AI_LLM_FALLBACK_ROUTE)
        );
        assert_eq!(
            calls[0].documents,
            vec!["first candidate", "second candidate"]
        );
    }

    #[tokio::test]
    async fn llm_fallback_rerank_failure_preserves_embedding_order() {
        let runtime = RecordingRerankRuntime {
            result: Err("invalid JSON from LLM".into()),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let mut hits = vec![
            ranked_hit("source-a", "first candidate", 0.91),
            ranked_hit("source-b", "second candidate", 0.52),
        ];

        apply_rerank(
            &runtime,
            &compiled_profile(RerankStrategy::LlmFallback),
            &search_request(),
            "which document is relevant",
            &mut hits,
        )
        .await
        .expect("LLM fallback failure is non-fatal");

        assert_eq!(
            hits.iter()
                .map(|hit| hit.result.source_code.as_str())
                .collect::<Vec<_>>(),
            vec!["source-a", "source-b"]
        );
        assert_eq!(hits[0].result.score, 0.91);
        assert_eq!(hits[1].result.score, 0.52);
    }
}
