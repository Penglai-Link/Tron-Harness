use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::application::rag::{CandidateHit, ContextPack};

use super::{
    code::{
        CurrentMemory, MemoryLocator, MemoryScope, SearchMode, encode_memory_code,
        memory_token_count,
    },
    dto::{
        MemoryContextPack, MemoryContextPackItem, MemoryItem, MemoryRunContext,
        MemorySearchCommand, MemorySearchResponse,
    },
};

pub(super) fn memory_item_from_hit(hit: &CandidateHit, context: &MemoryRunContext) -> MemoryItem {
    let metadata = &hit.metadata;
    let fields = &hit.fields;
    let locator = inferred_locator(hit, context);
    MemoryItem {
        memory_code: encode_memory_code(&locator),
        scope: locator.scope.as_public().into(),
        scope_code: locator.scope_code.clone(),
        memory_type: value_string(fields, "memoryType"),
        memory_status: value_string(fields, "memoryStatus"),
        review_status: value_string(fields, "reviewStatus"),
        injectable: fields.get("injectable").and_then(Value::as_bool),
        score: Some(hit.score),
        importance_score: value_i64(fields, "importanceScore"),
        confidence_score: value_i64(fields, "confidenceScore"),
        quality_score: value_i64(fields, "qualityScore"),
        title: hit.title.clone(),
        preview: None,
        text: Some(hit.text.clone()),
        expires_at: value_i64(fields, "expiresAt"),
        derived_source_type: value_string(fields, "derivedSourceType"),
        derived_source_code: value_string(fields, "derivedSourceCode"),
        updated_at: value_i64(fields, "sourceUpdatedAt"),
        metadata: agent_facing_metadata(metadata),
    }
}

pub(super) fn memory_item_from_current(current: &CurrentMemory) -> MemoryItem {
    MemoryItem {
        memory_code: encode_memory_code(&current.locator),
        scope: current.locator.scope.as_public().into(),
        scope_code: current.locator.scope_code.clone(),
        memory_type: current.memory_type.clone(),
        memory_status: current.memory_status.clone(),
        review_status: current.review_status.clone(),
        injectable: current.injectable,
        score: None,
        importance_score: current.importance_score,
        confidence_score: current.confidence_score,
        quality_score: current.quality_score,
        title: current.title.clone(),
        preview: None,
        text: Some(current.text.clone()),
        expires_at: current.expires_at,
        derived_source_type: current.derived_source_type.clone(),
        derived_source_code: current.derived_source_code.clone(),
        updated_at: current.updated_at,
        metadata: agent_facing_metadata(&current.metadata),
    }
}

pub(super) fn context_pack_from_rag(
    pack: ContextPack,
    hits: &[CandidateHit],
    context: &MemoryRunContext,
) -> MemoryContextPack {
    let codes_by_source = hits
        .iter()
        .map(|hit| {
            let locator = inferred_locator(hit, context);
            (hit.source_code.clone(), encode_memory_code(&locator))
        })
        .collect::<BTreeMap<_, _>>();
    MemoryContextPack {
        token_count: pack.token_count,
        items: pack
            .items
            .into_iter()
            .map(|item| MemoryContextPackItem {
                memory_code: codes_by_source.get(&item.source_code).cloned(),
                text: item.text,
                tokens: item.tokens,
                metadata: item
                    .metadata
                    .get("metadata")
                    .map(agent_facing_metadata)
                    .unwrap_or_else(|| agent_facing_metadata(&item.metadata)),
            })
            .collect(),
    }
}

pub(super) fn merge_context_pack_search_responses(
    mut baseline: MemorySearchResponse,
    related: Option<MemorySearchResponse>,
    limit: usize,
    token_budget: usize,
) -> MemorySearchResponse {
    let mut warnings = std::mem::take(&mut baseline.warnings);
    let trace_code = baseline.trace_code.take();
    let baseline_context_items = baseline
        .context_pack
        .take()
        .map(|pack| pack.items)
        .unwrap_or_default();
    let baseline_item_codes = baseline
        .items
        .iter()
        .map(|item| item.memory_code.clone())
        .collect::<BTreeSet<_>>();

    let mut items_by_code = BTreeMap::<String, MemoryItem>::new();
    for item in baseline.items {
        items_by_code.insert(item.memory_code.clone(), item);
    }

    let mut related_context_items = Vec::new();
    if let Some(mut related) = related {
        warnings.append(&mut related.warnings);
        if let Some(related_trace_code) = related.trace_code {
            warnings.push(json!({
                "code": "MEMORY_CONTEXT_PACK_RELATED_TRACE",
                "message": "Related memory query trace is available separately.",
                "traceCode": related_trace_code
            }));
        }
        for item in related.items {
            if !baseline_item_codes.contains(&item.memory_code) {
                if let Some(context_item) = context_item_from_memory_item(&item) {
                    related_context_items.push(context_item);
                }
            }
            match items_by_code.entry(item.memory_code.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(item);
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    if should_replace_memory_item(entry.get(), &item) {
                        entry.insert(item);
                    }
                }
            }
        }
    }

    let mut items = items_by_code.into_values().collect::<Vec<_>>();
    items.sort_by(memory_item_order);
    items.truncate(limit);
    let final_codes = items
        .iter()
        .map(|item| item.memory_code.clone())
        .collect::<BTreeSet<_>>();
    let context_pack = Some(merge_context_items(
        baseline_context_items,
        related_context_items,
        &final_codes,
        token_budget,
    ));
    MemorySearchResponse {
        success: true,
        count: items.len(),
        items,
        context_pack,
        warnings,
        trace_code,
    }
}

pub(super) fn current_matches_filters(
    current: &CurrentMemory,
    command: &MemorySearchCommand,
    mode: &SearchMode,
) -> bool {
    let default_memory_status = json!("ACTIVE");
    let default_review_status = json!("APPROVED");
    value_filter_matches(&current.memory_type, command.memory_type.as_ref())
        && value_filter_matches(
            &current.memory_status,
            command
                .memory_status
                .as_ref()
                .or(Some(&default_memory_status)),
        )
        && value_filter_matches(
            &current.review_status,
            command
                .review_status
                .as_ref()
                .or(Some(&default_review_status)),
        )
        && bool_filter_matches(current.injectable, command.injectable.as_ref())
        && (*mode != SearchMode::ContextPack || current.injectable.unwrap_or(false))
}

pub(super) fn agent_metadata_from_source(metadata: &Value) -> Value {
    let mut object = metadata
        .get("extra")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| {
            agent_metadata(metadata)
                .as_object()
                .cloned()
                .unwrap_or_default()
        });
    for key in ["sourceRefs", "semanticKey"] {
        if !object.contains_key(key) {
            if let Some(value) = metadata.get(key) {
                object.insert(key.into(), value.clone());
            }
        }
    }
    Value::Object(object)
}

fn inferred_locator(hit: &CandidateHit, context: &MemoryRunContext) -> MemoryLocator {
    let scope = MemoryScope::parse(&wire_scope_to_public(&hit.scope_type))
        .unwrap_or_else(|_| fallback_scope(hit, context));
    let scope_code = normalized_hit_scope_code(&hit.scope_code).unwrap_or_else(|| match scope {
        MemoryScope::User => context.user_code.clone(),
        MemoryScope::WorkDomain => context.work_domain_code.clone(),
        MemoryScope::Space => context.workspace_code.clone(),
    });
    MemoryLocator {
        scope,
        scope_code,
        source_code: hit.source_code.clone(),
        source_version: hit.source_version.clone(),
    }
}

fn wire_scope_to_public(scope_type: &str) -> String {
    match scope_type.trim().to_ascii_uppercase().as_str() {
        "AGENT" => "WORK_DOMAIN".into(),
        value => value.into(),
    }
}

fn normalized_hit_scope_code(scope_code: &str) -> Option<String> {
    let value = scope_code.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn fallback_scope(hit: &CandidateHit, context: &MemoryRunContext) -> MemoryScope {
    if hit.scope_code == context.user_code || hit.source_code.starts_with("memory-user-") {
        MemoryScope::User
    } else if hit.scope_code == context.work_domain_code
        || hit.source_code.starts_with("memory-work-domain-")
    {
        MemoryScope::WorkDomain
    } else {
        MemoryScope::Space
    }
}

fn value_filter_matches(actual: &Option<String>, filter: Option<&Value>) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    let Some(actual) = actual else {
        return false;
    };
    if let Some(expected) = filter.as_str() {
        return actual == expected;
    }
    filter
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(actual)))
}

fn bool_filter_matches(actual: Option<bool>, filter: Option<&Value>) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    let Some(actual) = actual else {
        return false;
    };
    if let Some(expected) = filter.as_bool() {
        return actual == expected;
    }
    filter
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_bool() == Some(actual)))
}

fn value_string(fields: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn value_i64(fields: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    fields.get(key).and_then(Value::as_i64)
}

pub(super) fn agent_facing_metadata(metadata: &Value) -> Value {
    let object = metadata
        .get("extra")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| metadata.as_object().cloned().unwrap_or_default());
    let mut visible = serde_json::Map::new();
    for key in ["semanticKey", "language", "subject", "purpose"] {
        if let Some(value) = object.get(key) {
            visible.insert(key.into(), value.clone());
        }
    }
    Value::Object(visible)
}

fn should_replace_memory_item(existing: &MemoryItem, incoming: &MemoryItem) -> bool {
    match (existing.score, incoming.score) {
        (Some(existing), Some(incoming)) => incoming > existing,
        (None, Some(_)) => true,
        (Some(_), None) | (None, None) => false,
    }
}

fn memory_item_order(left: &MemoryItem, right: &MemoryItem) -> std::cmp::Ordering {
    match (left.score, right.score) {
        (Some(left_score), Some(right_score)) => right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
    .then_with(|| left.memory_code.cmp(&right.memory_code))
}

fn context_item_from_memory_item(item: &MemoryItem) -> Option<MemoryContextPackItem> {
    let text = item
        .text
        .as_ref()
        .or(item.preview.as_ref())
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())?
        .to_string();
    Some(MemoryContextPackItem {
        memory_code: Some(item.memory_code.clone()),
        tokens: memory_text_token_count(&text),
        text,
        metadata: item.metadata.clone(),
    })
}

fn merge_context_items(
    baseline_items: Vec<MemoryContextPackItem>,
    related_items: Vec<MemoryContextPackItem>,
    final_codes: &BTreeSet<String>,
    token_budget: usize,
) -> MemoryContextPack {
    let mut token_count = 0usize;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for mut item in baseline_items.into_iter().chain(related_items) {
        let Some(memory_code) = item.memory_code.clone() else {
            continue;
        };
        if !final_codes.contains(&memory_code) || !seen.insert(memory_code) {
            continue;
        }
        let remaining = token_budget.saturating_sub(token_count);
        if remaining == 0 {
            break;
        }
        item.tokens = memory_text_token_count(&item.text).min(remaining);
        token_count += item.tokens;
        items.push(item);
    }
    MemoryContextPack { items, token_count }
}

fn memory_text_token_count(text: &str) -> usize {
    memory_token_count(text).max(1) as usize
}

fn agent_metadata(metadata: &Value) -> Value {
    let mut object = metadata
        .get("extra")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| metadata.as_object().cloned().unwrap_or_default());
    for key in [
        "source",
        "memory",
        "document",
        "textUnit",
        "title",
        "text",
        "sourceCode",
        "sourceVersion",
        "sourceType",
        "sourceContentHash",
        "memoryType",
        "memoryStatus",
        "reviewStatus",
        "injectable",
        "importanceScore",
        "confidenceScore",
        "qualityScore",
        "pinLevel",
        "pin_level",
        "expiresAt",
        "derivedSourceType",
        "derivedSourceCode",
    ] {
        object.remove(key);
    }
    Value::Object(object)
}
