use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::domain::rag::RagError;

use super::{
    CandidateHit, CompiledRetrievalProfile, ContextPack, ContextPackItem, RagRerankCall,
    RagSearchRequest, RagSearchRow, RagSearchRuntime, RetrievalRoute,
    search_strategy::{RetrieverKind, RetrieverStrategy},
};

pub(super) fn rank_route_rows(
    rows: Vec<RagSearchRow>,
    route: &RetrievalRoute,
    strategy: &RetrieverStrategy,
    request: &RagSearchRequest,
    query_embedding: Option<&[f32]>,
) -> Vec<CandidateHit> {
    let mut hits = rows
        .into_iter()
        .filter_map(|row| {
            let score = match strategy.kind {
                RetrieverKind::Fulltext if fulltext_matches(&row.search_text, &request.query) => {
                    fulltext_score(&row.search_text, &request.query)
                }
                RetrieverKind::Vector => row
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
        metadata: structured_metadata(row),
    }
}

fn structured_metadata(row: &RagSearchRow) -> Value {
    let mut flat = row.source_metadata.as_object().cloned().unwrap_or_default();
    let mut metadata = Map::new();

    let source = take_group(&mut flat, &["title", "sourceType", "sourceContentHash"]);
    if !source.is_empty() {
        metadata.insert("source".into(), Value::Object(source));
    }

    let memory = take_group(
        &mut flat,
        &[
            "memoryType",
            "memoryStatus",
            "reviewStatus",
            "version",
            "supersededBy",
            "conflictGroup",
            "duplicateGroup",
            "injectable",
            "importanceScore",
            "confidenceScore",
            "qualityScore",
            "pinLevel",
            "expiresAt",
            "derivedSourceType",
            "derivedSourceCode",
        ],
    );
    if !memory.is_empty() {
        metadata.insert("memory".into(), Value::Object(memory));
    }

    let document = take_group(
        &mut flat,
        &[
            "documentCode",
            "documentType",
            "path",
            "bodyHash",
            "repoCode",
            "moduleCode",
            "moduleName",
        ],
    );
    if !document.is_empty() {
        metadata.insert("document".into(), Value::Object(document));
    }

    let mut text_unit = Map::new();
    text_unit.insert("headingPath".into(), json!(row.heading_path));
    text_unit.insert("ordinal".into(), json!(row.ordinal));
    if let Some(status) = flat.remove("textUnitStatus") {
        text_unit.insert("status".into(), status);
    }
    if let Some(text_metadata) = row
        .text_metadata
        .as_object()
        .filter(|object| !object.is_empty())
    {
        text_unit.insert("metadata".into(), Value::Object(text_metadata.clone()));
    }
    metadata.insert("textUnit".into(), Value::Object(text_unit));

    let mut extra = take_extra(&mut flat);
    for (key, value) in flat {
        if !is_promoted_extra_key(&key) && !extra.contains_key(&key) {
            extra.insert(key, value);
        }
    }
    sanitize_extra_metadata(&mut extra);
    metadata.insert("extra".into(), Value::Object(extra));

    Value::Object(metadata)
}

fn take_group(flat: &mut Map<String, Value>, keys: &[&str]) -> Map<String, Value> {
    let mut group = Map::new();
    for key in keys {
        if let Some(value) = flat.remove(*key) {
            group.insert((*key).into(), value);
        }
    }
    group
}

fn take_extra(flat: &mut Map<String, Value>) -> Map<String, Value> {
    let mut extra = match flat.remove("extra") {
        Some(Value::Object(object)) => object,
        _ => Map::new(),
    };
    let legacy_payload_shape = extra.keys().any(|key| is_promoted_extra_key(key));
    if legacy_payload_shape {
        if let Some(Value::Object(nested)) = extra.remove("metadata") {
            for (key, value) in nested {
                extra.entry(key).or_insert(value);
            }
        } else {
            extra.remove("metadata");
        }
    }
    sanitize_extra_metadata(&mut extra);
    extra
}

fn sanitize_extra_metadata(extra: &mut Map<String, Value>) {
    for key in PROMOTED_EXTRA_KEYS {
        extra.remove(*key);
    }
}

fn is_promoted_extra_key(key: &str) -> bool {
    PROMOTED_EXTRA_KEYS.contains(&key)
}

const PROMOTED_EXTRA_KEYS: &[&str] = &[
    "title",
    "text",
    "sourceType",
    "sourceContentHash",
    "sourceMetadata",
    "payload",
    "memoryType",
    "memoryStatus",
    "reviewStatus",
    "version",
    "supersededBy",
    "conflictGroup",
    "duplicateGroup",
    "injectable",
    "importanceScore",
    "confidenceScore",
    "qualityScore",
    "pinLevel",
    "expiresAt",
    "derivedSourceType",
    "derivedSourceCode",
    "documentCode",
    "documentType",
    "path",
    "bodyHash",
    "repoCode",
    "moduleCode",
    "moduleName",
    "headingPath",
    "textUnitStatus",
];
