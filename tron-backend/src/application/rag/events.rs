use crate::shared::event_bus::{BusEventDraft, BusOutcome, EventBusHandle};

const ALLOWED_KEYS: &[&str] = &[
    "workspace_code",
    "corpus_id",
    "profile_id",
    "source_code",
    "source_version",
    "scope_type",
    "scope_code",
    "stage_id",
    "route_id",
    "status",
    "warning_code",
    "trace_code",
    "count",
    "updated_count",
    "result_count",
    "warning_count",
    "context_budget_units",
    "context_used_units",
    "error_code",
];

pub fn publish_rag_event(
    event_bus: &EventBusHandle,
    event_name: &'static str,
    outcome: BusOutcome,
    fields: &[(&'static str, String)],
) {
    if !event_name.starts_with("rag.") {
        return;
    }
    let mut draft = BusEventDraft::audit(event_name, "rag_service")
        .outcome(outcome)
        .summary(event_name);
    for (key, value) in fields {
        if ALLOWED_KEYS.contains(key) && !value.trim().is_empty() {
            draft = draft.attr(*key, compact_event_value(value));
        }
    }
    let _ = event_bus.publish(draft);
}

fn compact_event_value(value: &str) -> String {
    const MAX_EVENT_ATTR_VALUE: usize = 512;
    let trimmed = value.trim();
    if trimmed.len() <= MAX_EVENT_ATTR_VALUE {
        return trimmed.to_string();
    }
    let mut end = MAX_EVENT_ATTR_VALUE;
    while !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    trimmed[..end].to_string()
}
