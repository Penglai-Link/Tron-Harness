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
];

pub fn publish_rag_event(
    event_bus: &EventBusHandle,
    event_name: &'static str,
    outcome: BusOutcome,
    fields: &[(&'static str, String)],
) {
    let mut draft = BusEventDraft::audit(event_name, "rag_service")
        .outcome(outcome)
        .summary(event_name);
    for (key, value) in fields {
        if ALLOWED_KEYS.contains(key) && !value.trim().is_empty() {
            draft = draft.payload_field(*key, value.clone());
        }
    }
    let _ = event_bus.publish(draft);
}
