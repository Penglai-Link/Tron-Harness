use std::{collections::BTreeMap, sync::Arc};

use crate::{
    domain::agent::{
        AgentAguiEvent, AgentAguiEventSnapshot, AgentAguiProtocolEvent, AgentAguiStreamState,
    },
    shared::{
        event_bus::{AguiEventSink, BusEvent, BusOutcome, BusRouteClass},
        time::timestamp_from_millis,
    },
};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAguiStreamError {
    StreamKeyRequired,
}

#[derive(Clone)]
pub struct AgentAguiStreamService {
    sink: Arc<AguiEventSink>,
}

impl AgentAguiStreamService {
    pub fn new(sink: Arc<AguiEventSink>) -> Self {
        Self { sink }
    }

    pub fn activate(&self, stream_key: &str) -> Result<AgentAguiStreamState, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        self.sink.activate_stream(stream_key.clone());
        Ok(self.state(stream_key))
    }

    pub fn deactivate(
        &self,
        stream_key: &str,
    ) -> Result<AgentAguiStreamState, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        self.sink.deactivate_stream(&stream_key);
        Ok(self.state(stream_key))
    }

    pub fn events(&self, stream_key: &str) -> Result<AgentAguiEventSnapshot, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        let snapshot = self.sink.snapshot();
        let events = self
            .sink
            .replay(&stream_key)
            .into_iter()
            .filter_map(|event| convert_agui_event(&stream_key, event))
            .collect::<Vec<_>>();
        Ok(AgentAguiEventSnapshot {
            stream_key,
            active_streams: snapshot.active_streams,
            total_streams: snapshot.total_streams,
            events,
        })
    }

    pub fn protocol_events(
        &self,
        stream_key: &str,
    ) -> Result<Vec<AgentAguiProtocolEvent>, AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        Ok(self
            .sink
            .replay(&stream_key)
            .into_iter()
            .flat_map(|event| convert_protocol_events(&stream_key, event))
            .collect())
    }

    fn state(&self, stream_key: String) -> AgentAguiStreamState {
        let snapshot = self.sink.snapshot();
        AgentAguiStreamState {
            stream_key,
            active_streams: snapshot.active_streams,
            total_streams: snapshot.total_streams,
            total_replay_bytes: snapshot.total_replay_bytes,
            replay_evicted: snapshot.replay_evicted,
        }
    }
}

fn normalized_stream_key(stream_key: &str) -> Result<String, AgentAguiStreamError> {
    let stream_key = stream_key.trim();
    if stream_key.is_empty() {
        return Err(AgentAguiStreamError::StreamKeyRequired);
    }
    Ok(stream_key.to_string())
}

fn convert_agui_event(stream_key: &str, event: BusEvent) -> Option<AgentAguiEvent> {
    if event.route_class != BusRouteClass::Agui || !event.event_name.starts_with("agent.") {
        return None;
    }
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
    for attr in event.attrs {
        metadata.insert(attr.key, attr.value);
    }
    Some(AgentAguiEvent {
        id: event.event_id.as_decimal_string(),
        event_type: event.event_name.clone(),
        event_name: event.event_name,
        run_code,
        stream_key: stream_key.to_string(),
        occurred_at: timestamp_from_millis(event.occurred_at_ms),
        outcome: outcome_to_code(event.outcome).into(),
        metadata,
    })
}

fn convert_protocol_events(stream_key: &str, event: BusEvent) -> Vec<AgentAguiProtocolEvent> {
    if event.route_class != BusRouteClass::Agui || !event.event_name.starts_with("agent.") {
        return Vec::new();
    }
    let run_code = event
        .subject
        .as_ref()
        .and_then(|subject| subject.subject_code.clone())
        .unwrap_or_else(|| stream_key.to_string());
    let timestamp = event.occurred_at_ms;
    let assistant_message_id = format!("{run_code}-assistant");
    let reasoning_message_id = format!("{run_code}-reasoning");

    match event.event_name.as_str() {
        "agent.run.started" => vec![
            protocol_event(
                "RUN_STARTED",
                json!({
                    "type": "RUN_STARTED",
                    "threadId": stream_key,
                    "runId": run_code,
                    "timestamp": timestamp,
                }),
            ),
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
        "agent.model.requested" => vec![
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
        "agent.reasoning.delta" => payload_delta(&event)
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
        "agent.model.responded" => vec![
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
        "agent.output.delta" => payload_delta(&event)
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
        "agent.output.created" => vec![protocol_event(
            "TEXT_MESSAGE_END",
            json!({
                "type": "TEXT_MESSAGE_END",
                "threadId": stream_key,
                "runId": run_code,
                "messageId": assistant_message_id,
                "timestamp": timestamp,
            }),
        )],
        "agent.run.completed" => vec![protocol_event(
            "RUN_FINISHED",
            json!({
                "type": "RUN_FINISHED",
                "threadId": stream_key,
                "runId": run_code,
                "timestamp": timestamp,
            }),
        )],
        "agent.run.failed" => vec![protocol_event(
            "RUN_ERROR",
            json!({
                "type": "RUN_ERROR",
                "threadId": stream_key,
                "runId": run_code,
                "message": event.summary.unwrap_or_else(|| "agent run failed".into()),
                "timestamp": timestamp,
            }),
        )],
        "agent.tool.started" | "agent.tool.selected" => {
            let tool_name = event
                .summary
                .clone()
                .or_else(|| attr_value(&event, "tool_name"))
                .unwrap_or_else(|| "tool".into());
            vec![protocol_event(
                "TOOL_CALL_START",
                json!({
                    "type": "TOOL_CALL_START",
                    "threadId": stream_key,
                    "runId": run_code,
                    "toolCallId": tool_call_id(&run_code, &tool_name),
                    "toolCallName": tool_name,
                    "parentMessageId": assistant_message_id,
                    "timestamp": timestamp,
                }),
            )]
        }
        "agent.tool.completed" => {
            let tool_name = event.summary.clone().unwrap_or_else(|| "tool".into());
            let tool_call_id = tool_call_id(&run_code, &tool_name);
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
                        "content": "completed",
                        "role": "tool",
                        "timestamp": timestamp,
                    }),
                ),
            ]
        }
        "agent.tool.failed" => {
            let tool_name = event.summary.clone().unwrap_or_else(|| "tool".into());
            vec![protocol_event(
                "TOOL_CALL_RESULT",
                json!({
                    "type": "TOOL_CALL_RESULT",
                    "threadId": stream_key,
                    "runId": run_code,
                    "messageId": format!("{run_code}-tool-result"),
                    "toolCallId": tool_call_id(&run_code, &tool_name),
                    "content": "failed",
                    "role": "tool",
                    "timestamp": timestamp,
                }),
            )]
        }
        "agent.observation.created" => vec![protocol_event(
            "ACTIVITY_DELTA",
            json!({
                "type": "ACTIVITY_DELTA",
                "threadId": stream_key,
                "runId": run_code,
                "delta": [{"op":"add","path":"/logs/-","value":"observation created"}],
                "timestamp": timestamp,
            }),
        )],
        _ => Vec::new(),
    }
}

fn protocol_event(sse_event: &str, data: Value) -> AgentAguiProtocolEvent {
    AgentAguiProtocolEvent {
        sse_event: sse_event.into(),
        data,
    }
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

fn tool_call_id(run_code: &str, tool_name: &str) -> String {
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
