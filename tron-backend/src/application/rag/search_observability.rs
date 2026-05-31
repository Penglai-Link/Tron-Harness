use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

use crate::{
    domain::rag::RagError,
    shared::event_bus::{BusOutcome, EventBusHandle},
};

use super::{
    CandidateHit, CompiledRetrievalProfile, ContextPack, RagSearchRepository, RagSearchRequest,
    RagSearchTraceWrite, publish_rag_event, safe_trace_payload,
};

pub(super) fn persist_trace_if_enabled<R: RagSearchRepository>(
    repository: &R,
    request: &RagSearchRequest,
    compiled: &CompiledRetrievalProfile,
    hits: &[CandidateHit],
    context_pack: Option<&ContextPack>,
    warnings: &mut Vec<Value>,
) -> Result<Option<String>, RagError> {
    if !request.trace.enabled {
        return Ok(None);
    }
    let now = now_millis();
    let trace_code = format!("rag_trace_{now}");
    let payload = json!({
        "routeCounts": route_counts(hits),
        "routeTimingsMs": {},
        "warnings": warnings,
        "candidateRefs": hits.iter().map(candidate_ref).collect::<Vec<_>>(),
        "resultCount": hits.len(),
        "tokenUsage": {
            "tokenBudget": request.token_budget,
            "contextTokens": context_pack.map_or(0, |pack| pack.token_count)
        },
        "contextStrategy": request.context.strategy,
    });
    if let Err(error) = safe_trace_payload(&payload) {
        warnings.push(json!({
            "code": "RAG_SEARCH_TRACE_WRITE_WARNING",
            "message": "Search trace skipped; payload failed safety validation"
        }));
        return Err(error);
    }
    match repository.persist_search_trace(RagSearchTraceWrite {
        record_code: trace_code.clone(),
        profile_id: compiled.profile_id.clone(),
        workspace_code: request.workspace_code.clone(),
        user_code: request.access_context.user_code.clone(),
        agent_code: request.access_context.agent_code.clone(),
        query_text: request.query.clone(),
        payload,
        created_at: now,
        expires_at: now + 7 * 24 * 60 * 60 * 1000,
    }) {
        Ok(()) => Ok(Some(trace_code)),
        Err(_) => {
            warnings.push(json!({
                "code": "RAG_SEARCH_TRACE_WRITE_WARNING",
                "message": "Search trace write failed; search result remains available"
            }));
            Ok(None)
        }
    }
}

pub(super) fn publish_search_event(
    event_bus: &EventBusHandle,
    event_name: &'static str,
    request: &RagSearchRequest,
) {
    publish_rag_event(
        event_bus,
        event_name,
        BusOutcome::Success,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("profile_id", request.profile_id.clone()),
            ("status", "success".to_string()),
        ],
    );
}

pub(super) fn publish_route_events(
    event_bus: &EventBusHandle,
    hits: &[CandidateHit],
    request: &RagSearchRequest,
) {
    for (route_id, count) in route_counts(hits) {
        publish_rag_event(
            event_bus,
            "rag.route.completed",
            BusOutcome::Success,
            &[
                ("workspace_code", request.workspace_code.clone()),
                ("profile_id", request.profile_id.clone()),
                ("route_id", route_id),
                ("count", count.to_string()),
            ],
        );
    }
}

fn candidate_ref(hit: &CandidateHit) -> Value {
    json!({
        "corpusId": hit.corpus_id,
        "sourceCode": hit.source_code,
        "sourceVersion": hit.source_version,
        "targetType": hit.target_type,
        "targetCode": hit.target_code,
        "routeIds": hit.route_ids,
    })
}

fn route_counts(hits: &[CandidateHit]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for hit in hits {
        for route in &hit.route_ids {
            *counts.entry(route.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
