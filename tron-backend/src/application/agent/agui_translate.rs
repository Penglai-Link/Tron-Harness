use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{
    domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
    shared::{
        event_bus::{BusEvent, BusOutcome, BusRouteClass},
        time::timestamp_from_millis,
    },
};

use super::CanonicalRuntimeEvent;

pub fn translate_agui_event(stream_key: &str, event: &BusEvent) -> Option<AgentAguiEvent> {
    if event.route_class != BusRouteClass::Agui || !event.event_name.starts_with("agent.") {
        return None;
    }
    let canonical_event = CanonicalRuntimeEvent::from_bus_event_name(event.event_name.as_str())?;
    let run_code = event
        .subject
        .as_ref()
        .and_then(|subject| subject.subject_code.clone());
    let mut metadata = BTreeMap::new();
    metadata.insert("producer".into(), event.producer.clone());
    if let Some(subject) = &event.subject {
        metadata.insert("subjectType".into(), subject.subject_type.clone());
    }
    if let Some(summary) = &event.summary {
        metadata.insert("summary".into(), summary.clone());
    }
    metadata.insert("canonicalEvent".into(), canonical_event.code().into());
    for attr in &event.attrs {
        metadata.insert(attr.key.clone(), attr.value.clone());
    }
    for field in &event.payload_fields {
        if field.key == "reasoning_effort" {
            metadata.insert("reasoningEffort".into(), field.value.clone());
            continue;
        }
        if matches!(
            field.key.as_str(),
            "budgetBreakdown"
                | "context_window_limit"
                | "model_name"
                | "overBudgetReason"
                | "context_event_json"
                | "failureCode"
                | "usage_audit_json"
                | "context_block_id"
                | "usage_estimate_json"
                | "code"
                | "message"
                | "runCode"
                | "sessionCode"
        ) {
            metadata.insert(field.key.clone(), field.value.clone());
        }
    }
    Some(AgentAguiEvent {
        id: event.event_id.as_decimal_string(),
        event_type: event.event_name.clone(),
        event_name: event.event_name.clone(),
        run_code,
        stream_key: stream_key.to_string(),
        occurred_at: timestamp_from_millis(event.occurred_at_ms),
        outcome: outcome_to_code(event.outcome).into(),
        metadata,
    })
}

/// Only product context/audit notices are translated here. Generic Agent/message/tool
/// protocol events come from Framework's AgUiPlugin, with its execution identity intact.
pub fn translate_protocol_events(
    stream_key: &str,
    event: &BusEvent,
) -> Vec<AgentAguiProtocolEvent> {
    if event.route_class != BusRouteClass::Agui {
        return Vec::new();
    }
    let name = match event.event_name.as_str() {
        "agent.context.assembled" => "tron.short_term_memory.context.assembled",
        "agent.context.block_compressed" => "tron.short_term_memory.context.block_compressed",
        "agent.context.compaction_requested" => {
            "tron.short_term_memory.context.compaction_requested"
        }
        "agent.context.rebased" => "tron.short_term_memory.context.rebased",
        "agent.compaction.started" => "tron.short_term_memory.compaction.started",
        "agent.compaction.completed" => "tron.short_term_memory.compaction.completed",
        "agent.compaction.failed" => "tron.short_term_memory.compaction.failed",
        "agent.session_title_generation_failed" => "tron.session.title_generation_failed",
        "agent.mcp.warning" => "tron.mcp.runtime.warning",
        _ => return Vec::new(),
    };
    let run_code = event
        .subject
        .as_ref()
        .and_then(|subject| subject.subject_code.clone());
    let mut value = serde_json::Map::new();
    for field in &event.payload_fields {
        if field.key == "context_event_json" {
            if let Ok(Value::Object(context)) = serde_json::from_str(&field.value) {
                value.extend(context);
            }
        } else {
            value.insert(
                field.key.clone(),
                serde_json::from_str(&field.value)
                    .unwrap_or_else(|_| Value::String(field.value.clone())),
            );
        }
    }
    vec![AgentAguiProtocolEvent {
        sse_event: "CUSTOM".into(),
        data: json!({
            "type": "CUSTOM", "name": name, "value": value, "threadId": stream_key,
            "runId": run_code, "timestamp": event.occurred_at_ms,
            "metadata": {"tron": {"eventId": format!("bus:{}:0", event.event_id.as_decimal_string())}},
        }),
    }]
}

fn outcome_to_code(outcome: BusOutcome) -> &'static str {
    match outcome {
        BusOutcome::Success => "SUCCESS",
        BusOutcome::Failure => "FAILURE",
        BusOutcome::Rejected => "REJECTED",
        BusOutcome::Started => "STARTED",
        BusOutcome::Progress => "PROGRESS",
        BusOutcome::Completed => "COMPLETED",
    }
}

/// Compatibility audit projection of one actual native frame. No run state or
/// content assembly is reconstructed, and only non-sensitive source metadata is copied.
pub fn translate_native_agui_event(
    stream_key: &str,
    event: &AgentAguiProtocolEvent,
) -> Option<AgentAguiEvent> {
    let framework = event.data.pointer("/metadata/framework")?;
    let logical = framework.get("runId")?.as_str()?;
    let execution = framework.get("executionId")?.as_str()?;
    let sequence = framework.get("sourceSequence")?.as_u64()?;
    let (name, canonical, outcome) = match event.sse_event.as_str() {
        "RUN_STARTED" => (
            "agent.run.started",
            CanonicalRuntimeEvent::RunStarted,
            "STARTED",
        ),
        "RUN_FINISHED"
            if event.data.pointer("/outcome/type").and_then(Value::as_str) == Some("interrupt") =>
        {
            (
                "agent.run.awaiting_input",
                CanonicalRuntimeEvent::RunAwaitingInput,
                "AWAITING_INPUT",
            )
        }
        "RUN_FINISHED"
            if event.data.pointer("/outcome/type").and_then(Value::as_str) == Some("success") =>
        {
            (
                "agent.run.completed",
                CanonicalRuntimeEvent::RunCompleted,
                "COMPLETED",
            )
        }
        "RUN_ERROR"
            if !event
                .data
                .get("code")
                .and_then(Value::as_str)
                .is_some_and(|code| {
                    code.starts_with("AGUI_") || code == "CHAT_STREAM_COMPLETION_UNAVAILABLE"
                }) =>
        {
            (
                "agent.run.failed",
                CanonicalRuntimeEvent::RunFailed,
                "FAILURE",
            )
        }
        _ => return None,
    };
    let mut metadata = BTreeMap::from([
        ("producer".into(), "framework.agui".into()),
        ("canonicalEvent".into(), canonical.code().into()),
        ("nativeRunId".into(), logical.into()),
        ("executionId".into(), execution.into()),
        ("sourceSequence".into(), sequence.to_string()),
        ("nativeProtocolEvent".into(), event.sse_event.clone()),
    ]);
    if let Some(ordinal) = event
        .data
        .pointer("/metadata/tron/sourceOrdinal")
        .and_then(Value::as_u64)
    {
        metadata.insert("sourceOrdinal".into(), ordinal.to_string());
    }
    if let Some(code) = event.data.get("code").and_then(Value::as_str) {
        metadata.insert("code".into(), code.to_owned());
    }
    let id = super::protocol_event_identity(event)?;
    Some(AgentAguiEvent {
        id,
        event_type: name.into(),
        event_name: name.into(),
        run_code: event
            .data
            .get("runId")
            .and_then(Value::as_str)
            .map(str::to_owned),
        stream_key: stream_key.into(),
        occurred_at: timestamp_from_millis(event.data.get("timestamp").and_then(Value::as_i64)?),
        outcome: outcome.into(),
        metadata,
    })
}
