use std::collections::BTreeMap;

use serde_json::json;

use crate::domain::rag::RagError;

use super::{
    CandidateHit, CompiledRetrievalProfile, ContextPack, ContextPackItem, RagRerankCall,
    RagSearchRequest, RagSearchRow, RagSearchRuntime, RetrievalRoute,
};

pub(super) fn rank_route_rows(
    rows: Vec<RagSearchRow>,
    route: &RetrievalRoute,
    request: &RagSearchRequest,
    query_embedding: Option<&[f32]>,
) -> Vec<CandidateHit> {
    let mut hits = rows
        .into_iter()
        .filter_map(|row| {
            let score = match route.retriever.as_str() {
                "FULLTEXT" if fulltext_matches(&row.search_text, &request.query) => {
                    fulltext_score(&row.search_text, &request.query)
                }
                "VECTOR" => row
                    .embedding
                    .as_deref()
                    .and_then(|embedding| {
                        query_embedding.map(|query| cosine_similarity(query, embedding))
                    })
                    .unwrap_or(0.0),
                _ => 0.0,
            };
            (score > 0.0).then(|| {
                let mut hit = hit_from_row(&row);
                hit.route_ids.push(route.route_id.clone());
                hit.score = score * route.weight;
                hit
            })
        })
        .collect::<Vec<_>>();
    hits.sort_by(stable_hit_order);
    hits.truncate(route.top_k);
    hits
}

pub(super) fn fuse_and_dedup(hits: Vec<CandidateHit>) -> Vec<CandidateHit> {
    let mut by_target = BTreeMap::<String, CandidateHit>::new();
    for hit in hits {
        match by_target.entry(hit.target_code.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(hit);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                for route_id in hit.route_ids {
                    if !existing.route_ids.contains(&route_id) {
                        existing.route_ids.push(route_id);
                    }
                }
                existing.score += hit.score;
            }
        }
    }
    by_target.into_values().collect()
}

pub(super) async fn apply_rerank<E>(
    runtime: &E,
    compiled: &CompiledRetrievalProfile,
    request: &RagSearchRequest,
    hits: &mut Vec<CandidateHit>,
) -> Result<(), RagError>
where
    E: RagSearchRuntime,
{
    let documents = hits.iter().map(|hit| hit.text.clone()).collect::<Vec<_>>();
    let order = runtime
        .rerank(RagRerankCall {
            workspace_code: request.workspace_code.clone(),
            model_code: compiled.rerank_model_code.clone(),
            query: request.query.clone(),
            documents,
            top_n: Some(request.limit),
        })
        .await
        .map_err(|_| RagError::RerankFailed)?;
    if order.is_empty() {
        return Ok(());
    }
    let mut ranked = Vec::new();
    for (index, score) in order {
        if let Some(mut hit) = hits.get(index).cloned() {
            hit.score = score;
            ranked.push(hit);
        }
    }
    if !ranked.is_empty() {
        *hits = ranked;
    }
    Ok(())
}

pub(super) fn stable_hit_order(left: &CandidateHit, right: &CandidateHit) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.source_code.cmp(&right.source_code))
        .then_with(|| left.target_code.cmp(&right.target_code))
}

pub(super) fn assemble_context_pack(
    strategy: &str,
    hits: &[CandidateHit],
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
            source_code: hit.source_code.clone(),
            text: hit.text.clone(),
            tokens,
            metadata: json!({
                "strategy": strategy,
                "targetCode": hit.target_code,
                "metadata": hit.metadata
            }),
        });
    }
    Some(ContextPack { items, token_count })
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

fn hit_from_row(row: &RagSearchRow) -> CandidateHit {
    let mut metadata = row.source_metadata.clone();
    if let (Some(target), Some(source)) = (metadata.as_object_mut(), row.text_metadata.as_object())
    {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    if let Some(target) = metadata.as_object_mut() {
        target.insert("headingPath".into(), json!(row.heading_path));
    }
    CandidateHit {
        corpus_id: row.corpus_id.clone(),
        source_code: row.source_code.clone(),
        source_version: row.source_version.clone(),
        target_type: "TEXT_UNIT".into(),
        target_code: row.target_code.clone(),
        route_ids: Vec::new(),
        score: 0.0,
        scope_type: row.scope_type.clone(),
        scope_code: row.scope_code.clone(),
        preview: row.text.chars().take(160).collect(),
        text: row.text.clone(),
        token_count: row.token_count,
        metadata,
    }
}
