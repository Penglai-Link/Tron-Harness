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
                | "usage_estimate_json"
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

pub fn translate_protocol_events(
    stream_key: &str,
    event: &BusEvent,
) -> Vec<AgentAguiProtocolEvent> {
    if event.route_class != BusRouteClass::Agui || !event.event_name.starts_with("agent.") {
        return Vec::new();
    }
    let Some(canonical_event) =
        CanonicalRuntimeEvent::from_bus_event_name(event.event_name.as_str())
    else {
        return Vec::new();
    };
    let run_code = event
        .subject
        .as_ref()
        .and_then(|subject| subject.subject_code.clone())
        .unwrap_or_else(|| stream_key.to_string());
    let timestamp = event.occurred_at_ms;
    let assistant_message_id = format!("{run_code}-assistant");
    let reasoning_message_id = format!("{run_code}-reasoning");

    match canonical_event {
        CanonicalRuntimeEvent::RunRequested => Vec::new(),
        CanonicalRuntimeEvent::RunStarted => vec![
            protocol_event(
                "RUN_STARTED",
                json!({
                    "type": "RUN_STARTED",
                    "threadId": stream_key,
                    "runId": run_code,
                    "reasoningEffort": payload_value(event, "reasoning_effort"),
                    "timestamp": timestamp,
                }),
            ),
            activity_snapshot(stream_key, &run_code, "PREPARING", timestamp),
        ],
        CanonicalRuntimeEvent::ModelRequested => vec![
            activity_snapshot(stream_key, &run_code, "THINKING", timestamp),
            protocol_event(
                "REASONING_START",
                json!({
                    "type": "REASONING_START",
                    "threadId": stream_key,
                    "runId": run_code,
                    "timestamp": timestamp,
                }),
            ),
            protocol_event(
                "REASONING_MESSAGE_START",
                json!({
                    "type": "REASONING_MESSAGE_START",
                    "threadId": stream_key,
                    "runId": run_code,
                    "messageId": reasoning_message_id,
                    "timestamp": timestamp,
                }),
            ),
        ],
        CanonicalRuntimeEvent::ReasoningDelta => payload_delta(event)
            .map(|delta| {
                vec![protocol_event(
                    "REASONING_MESSAGE_CONTENT",
                    json!({
                        "type": "REASONING_MESSAGE_CONTENT",
                        "threadId": stream_key,
                        "runId": run_code,
                        "messageId": reasoning_message_id,
                        "delta": delta,
                        "timestamp": timestamp,
                    }),
                )]
            })
            .unwrap_or_default(),
        CanonicalRuntimeEvent::ReasoningRawDelta => payload_delta(event)
            .map(|delta| {
                vec![protocol_event(
                    "REASONING_TEXT_CONTENT",
                    json!({
                        "type": "REASONING_TEXT_CONTENT",
                        "threadId": stream_key,
                        "runId": run_code,
                        "messageId": format!("{run_code}-reasoning-raw"),
                        "delta": delta,
                        "timestamp": timestamp,
                    }),
                )]
            })
            .unwrap_or_default(),
        CanonicalRuntimeEvent::ModelResponded => vec![
            protocol_event(
                "REASONING_MESSAGE_END",
                json!({
                    "type": "REASONING_MESSAGE_END",
                    "threadId": stream_key,
                    "runId": run_code,
                    "messageId": reasoning_message_id,
                    "timestamp": timestamp,
                }),
            ),
            protocol_event(
                "REASONING_END",
                json!({
                    "type": "REASONING_END",
                    "threadId": stream_key,
                    "runId": run_code,
                    "timestamp": timestamp,
                }),
            ),
        ],
        CanonicalRuntimeEvent::OutputStarted => vec![
            activity_snapshot(stream_key, &run_code, "RESPONDING", timestamp),
            protocol_event(
                "TEXT_MESSAGE_START",
                json!({
                    "type": "TEXT_MESSAGE_START",
                    "threadId": stream_key,
                    "runId": run_code,
                    "messageId": assistant_message_id,
                    "role": "assistant",
                    "timestamp": timestamp,
                }),
            ),
        ],
        CanonicalRuntimeEvent::OutputDelta => payload_delta(event)
            .map(|delta| {
                vec![protocol_event(
                    "TEXT_MESSAGE_CONTENT",
                    json!({
                        "type": "TEXT_MESSAGE_CONTENT",
                        "threadId": stream_key,
                        "runId": run_code,
                        "messageId": assistant_message_id,
                        "delta": delta,
                        "timestamp": timestamp,
                    }),
                )]
            })
            .unwrap_or_default(),
        CanonicalRuntimeEvent::OutputCompleted => vec![protocol_event(
            "TEXT_MESSAGE_END",
            json!({
                "type": "TEXT_MESSAGE_END",
                "threadId": stream_key,
                "runId": run_code,
                "messageId": assistant_message_id,
                "timestamp": timestamp,
            }),
        )],
        CanonicalRuntimeEvent::RunCompleted => {
            let outcome = if event.summary.as_deref() == Some("awaiting_user_input") {
                json!({
                    "type": "interrupt",
                    "interrupts": [{
                        "id": format!("{run_code}-user-input"),
                        "reason": "input_required",
                        "message": "The agent is waiting for user input.",
                        "metadata": { "runCode": run_code },
                    }],
                })
            } else {
                json!({ "type": "success" })
            };
            vec![protocol_event(
                "RUN_FINISHED",
                json!({
                    "type": "RUN_FINISHED",
                    "threadId": stream_key,
                    "runId": run_code,
                    "outcome": outcome,
                    "timestamp": timestamp,
                }),
            )]
        }
        CanonicalRuntimeEvent::RunFailed => vec![protocol_event(
            "RUN_ERROR",
            json!({
                "type": "RUN_ERROR",
                "threadId": stream_key,
                "runId": run_code,
                "code": payload_value(event, "failureCode"),
                "message": event.summary.clone().unwrap_or_else(|| "agent run failed".into()),
                "timestamp": timestamp,
            }),
        )],
        CanonicalRuntimeEvent::ToolSelected => {
            let tool_name = event_tool_name(event);
            let tool_call_id = event_tool_call_id(event, &run_code, &tool_name);
            vec![protocol_event(
                "TOOL_CALL_START",
                json!({
                    "type": "TOOL_CALL_START",
                    "threadId": stream_key,
                    "runId": run_code,
                    "toolCallId": tool_call_id,
                    "toolCallName": tool_name,
                    "parentMessageId": assistant_message_id,
                    "timestamp": timestamp,
                }),
            )]
        }
        CanonicalRuntimeEvent::ToolStarted => {
            let tool_name = event_tool_name(event);
            let tool_call_id = event_tool_call_id(event, &run_code, &tool_name);
            let tool_args = event_tool_args(event);
            vec![protocol_event(
                "TOOL_CALL_ARGS",
                json!({
                    "type": "TOOL_CALL_ARGS",
                    "threadId": stream_key,
                    "runId": run_code,
                    "toolCallId": tool_call_id,
                    "delta": serde_json::to_string(&tool_args).unwrap_or_else(|_| "{}".into()),
                    "timestamp": timestamp,
                }),
            )]
        }
        CanonicalRuntimeEvent::ToolCompleted => {
            let tool_name = event_tool_name(event);
            let tool_call_id = event_tool_call_id(event, &run_code, &tool_name);
            let result = event_tool_result(event).unwrap_or_else(|| "completed".into());
            vec![
                protocol_event(
                    "TOOL_CALL_END",
                    json!({
                        "type": "TOOL_CALL_END",
                        "threadId": stream_key,
                        "runId": run_code,
                        "toolCallId": tool_call_id,
                        "timestamp": timestamp,
                    }),
                ),
                protocol_event(
                    "TOOL_CALL_RESULT",
                    json!({
                        "type": "TOOL_CALL_RESULT",
                        "threadId": stream_key,
                        "runId": run_code,
                        "messageId": format!("{run_code}-tool-result"),
                        "toolCallId": tool_call_id,
                        "content": result,
                        "role": "tool",
                        "timestamp": timestamp,
                    }),
                ),
            ]
        }
        CanonicalRuntimeEvent::ToolFailed => {
            let tool_name = event_tool_name(event);
            let tool_call_id = event_tool_call_id(event, &run_code, &tool_name);
            let result = event_tool_result(event).unwrap_or_else(|| "failed".into());
            vec![
                protocol_event(
                    "TOOL_CALL_RESULT",
                    json!({
                        "type": "TOOL_CALL_RESULT",
                        "threadId": stream_key,
                        "runId": run_code,
                        "messageId": format!("{run_code}-tool-result"),
                        "toolCallId": tool_call_id,
                        "toolCallName": tool_name,
                        "content": result,
                        "role": "tool",
                        "timestamp": timestamp,
                    }),
                ),
                protocol_event(
                    "TOOL_CALL_END",
                    json!({
                        "type": "TOOL_CALL_END",
                        "threadId": stream_key,
                        "runId": run_code,
                        "toolCallId": tool_call_id,
                        "timestamp": timestamp,
                    }),
                ),
            ]
        }
        CanonicalRuntimeEvent::ToolActivity | CanonicalRuntimeEvent::ObservationCreated => {
            let tool_name = event_tool_name(event);
            let tool_call_id = event_tool_call_id(event, &run_code, &tool_name);
            let log_line = event_tool_result(event).unwrap_or_else(|| "observation created".into());
            vec![protocol_event(
                "ACTIVITY_DELTA",
                json!({
                    "type": "ACTIVITY_DELTA",
                    "threadId": stream_key,
                    "runId": run_code,
                    "toolCallId": tool_call_id,
                    "toolCallName": tool_name,
                    "delta": [{"op":"add","path":"/logs/-","value":log_line}],
                    "timestamp": timestamp,
                }),
            )]
        }
        CanonicalRuntimeEvent::ContextAssembled
        | CanonicalRuntimeEvent::ContextCompactionRequested
        | CanonicalRuntimeEvent::ContextRebased
        | CanonicalRuntimeEvent::CompactionStarted
        | CanonicalRuntimeEvent::CompactionCompleted
        | CanonicalRuntimeEvent::CompactionFailed => {
            let name = short_term_memory_custom_event_name(canonical_event);
            let value = payload_value(event, "context_event_json")
                .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
                .unwrap_or_else(|| json!({}));
            vec![protocol_event(
                "CUSTOM",
                json!({
                    "type": "CUSTOM",
                    "name": name,
                    "value": value,
                    "threadId": stream_key,
                    "runId": run_code,
                    "timestamp": timestamp,
                }),
            )]
        }
    }
}

fn short_term_memory_custom_event_name(event: CanonicalRuntimeEvent) -> &'static str {
    match event {
        CanonicalRuntimeEvent::ContextAssembled => "tron.short_term_memory.context.assembled",
        CanonicalRuntimeEvent::ContextCompactionRequested => {
            "tron.short_term_memory.context.compaction_requested"
        }
        CanonicalRuntimeEvent::ContextRebased => "tron.short_term_memory.context.rebased",
        CanonicalRuntimeEvent::CompactionStarted => "tron.short_term_memory.compaction.started",
        CanonicalRuntimeEvent::CompactionCompleted => "tron.short_term_memory.compaction.completed",
        CanonicalRuntimeEvent::CompactionFailed => "tron.short_term_memory.compaction.failed",
        _ => unreachable!("only short-term-memory events use custom AG-UI translation"),
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::{
        event_bus::{BusEvent, BusEventDraft, BusOutcome, EventBusConfig},
        id::FixedSnowflakeIdProvider,
    };

    use super::{translate_agui_event, translate_protocol_events};

    #[test]
    fn translate_agui_event_exposes_short_term_memory_usage_payload() {
        let event = BusEvent::from_draft(
            BusEventDraft::agui("agent.model.requested", "agent_runtime")
                .outcome(BusOutcome::Started)
                .subject("agent_run", "RUN-1")
                .payload_field("model_name", "gpt-5-codex")
                .payload_field("context_window_limit", "128000")
                .payload_field("usage_estimate_json", r#"{"estimatedInputTokens":22600}"#)
                .payload_field("usage_audit_json", r#"{"usageSource":"provider"}"#)
                .payload_field("budgetBreakdown", r#"{"estimatedInputTokens":22600}"#)
                .payload_field("overBudgetReason", "too_many_tokens")
                .payload_field("internal_only", "hidden"),
            &FixedSnowflakeIdProvider::new(1),
            &EventBusConfig::default(),
        )
        .expect("event should validate");

        let translated = translate_agui_event("SESSION-1", &event).expect("event should translate");

        assert_eq!(
            translated
                .metadata
                .get("usage_estimate_json")
                .map(String::as_str),
            Some(r#"{"estimatedInputTokens":22600}"#),
        );
        assert_eq!(
            translated
                .metadata
                .get("usage_audit_json")
                .map(String::as_str),
            Some(r#"{"usageSource":"provider"}"#),
        );
        assert_eq!(
            translated
                .metadata
                .get("budgetBreakdown")
                .map(String::as_str),
            Some(r#"{"estimatedInputTokens":22600}"#),
        );
        assert_eq!(
            translated
                .metadata
                .get("overBudgetReason")
                .map(String::as_str),
            Some("too_many_tokens"),
        );
        assert_eq!(translated.metadata.get("internal_only"), None);
    }

    #[test]
    fn run_started_exposes_effective_reasoning_effort_in_both_agui_shapes() {
        let event = BusEvent::from_draft(
            BusEventDraft::agui("agent.run.started", "agent_runtime")
                .outcome(BusOutcome::Started)
                .subject("agent_run", "RUN-1")
                .payload_field("reasoning_effort", "High"),
            &FixedSnowflakeIdProvider::new(1),
            &EventBusConfig::default(),
        )
        .expect("event should validate");

        let translated = translate_agui_event("SESSION-1", &event).expect("event should translate");
        assert_eq!(
            translated
                .metadata
                .get("reasoningEffort")
                .map(String::as_str),
            Some("High")
        );

        let protocol_events = translate_protocol_events("SESSION-1", &event);
        assert_eq!(protocol_events[0].data["reasoningEffort"], "High");
        assert_eq!(protocol_events[1].sse_event, "ACTIVITY_SNAPSHOT");
        assert_eq!(protocol_events[1].data["activity"], "PREPARING");
    }

    #[test]
    fn short_term_memory_context_event_translates_to_agui_custom_event() {
        let event = BusEvent::from_draft(
            BusEventDraft::agui("agent.context.assembled", "short_term_memory")
                .outcome(BusOutcome::Completed)
                .subject("agent_run", "RUN-1")
                .stream_key("SESSION-1")
                .payload_field(
                    "context_event_json",
                    r#"{"estimatedTokens":12000,"budgetBreakdown":{"toolSchemaTokens":3000}}"#,
                ),
            &FixedSnowflakeIdProvider::new(1),
            &EventBusConfig::default(),
        )
        .expect("event should validate");

        let canonical = translate_agui_event("SESSION-1", &event)
            .expect("context event should translate to canonical AG-UI");
        assert_eq!(canonical.event_type, "agent.context.assembled");
        assert_eq!(
            canonical
                .metadata
                .get("context_event_json")
                .map(String::as_str),
            Some(r#"{"estimatedTokens":12000,"budgetBreakdown":{"toolSchemaTokens":3000}}"#),
        );

        let protocol_events = translate_protocol_events("SESSION-1", &event);
        assert_eq!(protocol_events.len(), 1);
        assert_eq!(protocol_events[0].sse_event, "CUSTOM");
        assert_eq!(
            protocol_events[0].data["name"],
            "tron.short_term_memory.context.assembled"
        );
        assert_eq!(
            protocol_events[0].data["value"]["budgetBreakdown"]["toolSchemaTokens"],
            3000
        );
    }

    #[test]
    fn run_failed_exposes_machine_readable_failure_code() {
        let event = BusEvent::from_draft(
            BusEventDraft::agui("agent.run.failed", "agent_runtime")
                .outcome(BusOutcome::Failure)
                .subject("agent_run", "RUN-1")
                .stream_key("SESSION-1")
                .summary("TOOL_SCHEMA_OVER_BUDGET")
                .payload_field("failureCode", "tool_schema_over_budget"),
            &FixedSnowflakeIdProvider::new(1),
            &EventBusConfig::default(),
        )
        .expect("event should validate");

        let protocol_events = translate_protocol_events("SESSION-1", &event);
        assert_eq!(protocol_events[0].sse_event, "RUN_ERROR");
        assert_eq!(protocol_events[0].data["code"], "tool_schema_over_budget");
    }
}

fn protocol_event(sse_event: &str, data: Value) -> AgentAguiProtocolEvent {
    AgentAguiProtocolEvent {
        sse_event: sse_event.into(),
        data,
    }
}

fn activity_snapshot(
    stream_key: &str,
    run_code: &str,
    activity: &str,
    timestamp: i64,
) -> AgentAguiProtocolEvent {
    protocol_event(
        "ACTIVITY_SNAPSHOT",
        json!({
            "type": "ACTIVITY_SNAPSHOT",
            "threadId": stream_key,
            "runId": run_code,
            "activity": activity,
            "timestamp": timestamp,
        }),
    )
}

fn payload_value(event: &BusEvent, key: &str) -> Option<String> {
    event
        .payload_fields
        .iter()
        .find(|attr| attr.key == key)
        .map(|attr| attr.value.clone())
}

fn payload_delta(event: &BusEvent) -> Option<String> {
    payload_value(event, "delta_json")
        .and_then(|value| serde_json::from_str::<String>(&value).ok())
        .or_else(|| payload_value(event, "delta"))
}

fn attr_value(event: &BusEvent, key: &str) -> Option<String> {
    event
        .attrs
        .iter()
        .find(|attr| attr.key == key)
        .map(|attr| attr.value.clone())
}

fn event_tool_name(event: &BusEvent) -> String {
    payload_value(event, "tool_name")
        .or_else(|| event.summary.clone())
        .or_else(|| attr_value(event, "tool_name"))
        .unwrap_or_else(|| "tool".into())
}

fn event_tool_call_id(event: &BusEvent, run_code: &str, tool_name: &str) -> String {
    payload_value(event, "tool_call_id")
        .unwrap_or_else(|| fallback_tool_call_id(run_code, tool_name))
}

fn event_tool_args(event: &BusEvent) -> Value {
    payload_value(event, "tool_args_json")
        .map(|value| serde_json::from_str::<Value>(&value).unwrap_or(Value::String(value)))
        .unwrap_or(Value::Null)
}

fn event_tool_result(event: &BusEvent) -> Option<String> {
    payload_value(event, "tool_result_json").or_else(|| payload_value(event, "tool_result"))
}

fn fallback_tool_call_id(run_code: &str, tool_name: &str) -> String {
    let suffix = tool_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    format!("{run_code}-tool-{suffix}")
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
