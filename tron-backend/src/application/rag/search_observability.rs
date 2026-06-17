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
    CompiledRetrievalProfile, ContextPack, RagSearchRepository, RagSearchRequest, RagSearchResult,
    RagSearchTraceWrite, RagStageExecutionResult, publish_rag_event, safe_trace_payload,
    search::RankedCandidate,
};

pub(super) fn persist_trace_if_enabled<R: RagSearchRepository>(
    repository: &R,
    request: &RagSearchRequest,
    compiled: &CompiledRetrievalProfile,
    hits: &[RankedCandidate],
    context_pack: Option<&ContextPack>,
    stage_results: &[RagStageExecutionResult],
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
        "stageResults": stage_results.iter().map(stage_result_summary).collect::<Vec<_>>(),
        "resultCount": hits.len(),
        "tokenUsage": {
            "tokenBudget": request.token_budget,
            "contextTokens": context_pack.map_or(0, |pack| pack.token_count)
        },
        "contextStrategy": request.context.strategy,
        "rankingPolicy": {
            "fusion": compiled.fusion_policy.type_name(),
            "dedup": compiled.dedup_policy.type_name(),
            "semantic": compiled.topic_policy.trace_type_name()
        },
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

pub(super) fn publish_search_started_event(event_bus: &EventBusHandle, request: &RagSearchRequest) {
    publish_rag_event(
        event_bus,
        "rag.search.started",
        BusOutcome::Started,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("profile_id", request.profile_id.clone()),
            ("status", "started".to_string()),
        ],
    );
}

pub(super) fn publish_search_completed_event(
    event_bus: &EventBusHandle,
    request: &RagSearchRequest,
    result: &RagSearchResult,
) {
    let mut fields = vec![
        ("workspace_code", request.workspace_code.clone()),
        ("profile_id", request.profile_id.clone()),
        ("status", result.status_for_event()),
        ("result_count", result.hits.len().to_string()),
        ("warning_count", result.warnings.len().to_string()),
        ("context_budget_units", request.token_budget.to_string()),
        (
            "context_used_units",
            result
                .context_pack
                .as_ref()
                .map_or(0, |pack| pack.token_count)
                .to_string(),
        ),
    ];
    if let Some(trace_code) = &result.trace_code {
        fields.push(("trace_code", trace_code.clone()));
    }
    publish_rag_event(
        event_bus,
        "rag.search.completed",
        BusOutcome::Completed,
        &fields,
    );
}

pub(super) fn publish_search_failed_event(
    event_bus: &EventBusHandle,
    request: &RagSearchRequest,
    error: &RagError,
) {
    publish_rag_event(
        event_bus,
        "rag.search.failed",
        BusOutcome::Failure,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("profile_id", request.profile_id.clone()),
            ("status", "failed".to_string()),
            ("error_code", format!("{error:?}")),
        ],
    );
}

pub(super) fn publish_route_events(
    event_bus: &EventBusHandle,
    hits: &[RankedCandidate],
    request: &RagSearchRequest,
) {
    for (route_id, count) in route_counts(hits) {
        publish_rag_event(
            event_bus,
            "rag.search.route.completed",
            BusOutcome::Completed,
            &[
                ("workspace_code", request.workspace_code.clone()),
                ("profile_id", request.profile_id.clone()),
                ("route_id", route_id),
                ("status", "completed".to_string()),
                ("result_count", count.to_string()),
                ("count", count.to_string()),
            ],
        );
    }
}

pub(super) fn publish_route_skipped_event(
    event_bus: &EventBusHandle,
    request: &RagSearchRequest,
    route_id: &str,
) {
    publish_rag_event(
        event_bus,
        "rag.search.route.skipped",
        BusOutcome::Completed,
        &[
            ("workspace_code", request.workspace_code.clone()),
            ("profile_id", request.profile_id.clone()),
            ("route_id", route_id.to_string()),
            ("status", "skipped".to_string()),
        ],
    );
}

trait SearchEventStatus {
    fn status_for_event(&self) -> String;
}

impl SearchEventStatus for RagSearchResult {
    fn status_for_event(&self) -> String {
        if self.warnings.is_empty() {
            "success".to_string()
        } else {
            "partial_warning".to_string()
        }
    }
}

fn candidate_ref(hit: &RankedCandidate) -> Value {
    json!({
        "corpusId": hit.result.corpus_id,
        "sourceCode": hit.result.source_code,
        "sourceVersion": hit.result.source_version,
        "targetType": "TEXT_UNIT",
        "targetCode": hit.target_code,
        "routeIds": hit.route_ids,
    })
}

fn stage_result_summary(stage: &RagStageExecutionResult) -> Value {
    json!({
        "nodeId": &stage.node_id,
        "stageType": &stage.stage_type,
        "strategyId": &stage.strategy_id,
        "status": &stage.status,
        "durationMs": stage.duration_ms,
        "inputArtifactKeys": &stage.input_artifact_keys,
        "outputArtifactKeys": &stage.output_artifact_keys,
        "warnings": &stage.warnings,
    })
}

fn route_counts(hits: &[RankedCandidate]) -> BTreeMap<String, usize> {
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
