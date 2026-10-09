use std::{
    collections::{BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

use serde_json::Value;

use crate::{
    domain::agent::{
        AgentAguiEvent, AgentAguiEventSnapshot, AgentAguiProtocolEvent, AgentAguiStreamState,
    },
    shared::event_bus::AguiEventSink,
};

use super::agui_translate::{translate_agui_event, translate_protocol_events};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAguiStreamError {
    StreamKeyRequired,
    ReplayReadFailed(String),
    ReplayWriteFailed(String),
    ReplayIncomplete(String),
}

pub trait AgentAguiReplayStore: Send + Sync {
    fn append_protocol_event(
        &self,
        stream_key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), String>;

    /// Nonblocking delivery diagnostics; no database I/O on the bus publication path.
    fn record_delivery_failure(&self, _stream_key: &str, _detail: &str) {}

    fn replay_failure(&self, _stream_key: &str) -> Option<String> {
        None
    }

    fn replay_agui_events(&self, stream_key: &str) -> Result<Vec<AgentAguiEvent>, String>;

    fn replay_protocol_events(
        &self,
        stream_key: &str,
    ) -> Result<Vec<AgentAguiProtocolEvent>, String>;
}

#[derive(Clone)]
pub struct AgentAguiStreamService {
    sink: Arc<AguiEventSink>,
    replay_store: Option<Arc<dyn AgentAguiReplayStore>>,
    transient_protocol: bool,
    protocol_buffer: Arc<Mutex<ProtocolBuffer>>,
}
#[derive(Default)]
struct ProtocolBuffer {
    frames: VecDeque<(String, AgentAguiProtocolEvent, usize)>,
    bytes: usize,
}
const MAX_PROTOCOL_BUFFER_BYTES: usize = 8 * 1024 * 1024;
const MAX_PROTOCOL_BUFFER_FRAMES: usize = 8192;

#[cfg(test)]
mod reconnect_tests {
    use super::*;
    use crate::shared::event_bus::EventBusConfig;

    #[test]
    fn transient_reconnect_retains_a_bounded_tail_across_sessions() {
        let service =
            AgentAguiStreamService::new(Arc::new(AguiEventSink::new(&EventBusConfig::default())));
        for sequence in 0..MAX_PROTOCOL_BUFFER_FRAMES + 2 {
            service
                .record_protocol_event(
                    "session",
                    AgentAguiProtocolEvent {
                        sse_event: "TEXT_MESSAGE_CONTENT".into(),
                        data: serde_json::json!({"sequence": sequence, "delta":"text"}),
                    },
                )
                .unwrap();
        }
        let frames = service.protocol_events("session").unwrap();
        assert_eq!(frames.len(), MAX_PROTOCOL_BUFFER_FRAMES);
        assert_eq!(frames[0].data["sequence"], 2);
        for session in ["other", "third"] {
            service.record_protocol_event(session, AgentAguiProtocolEvent {
                sse_event: "TEXT_MESSAGE_CONTENT".into(),
                data: serde_json::json!({"delta":"x".repeat(MAX_PROTOCOL_BUFFER_BYTES / 2)}),
            }).unwrap();
        }
        service
            .record_protocol_event(
                "oversized",
                AgentAguiProtocolEvent {
                    sse_event: "TEXT_MESSAGE_CONTENT".into(),
                    data: serde_json::json!({"delta":"x".repeat(MAX_PROTOCOL_BUFFER_BYTES)}),
                },
            )
            .unwrap();
        let buffer = service.protocol_buffer.lock().unwrap();
        assert!(buffer.bytes <= MAX_PROTOCOL_BUFFER_BYTES);
        assert!(buffer.frames.len() <= MAX_PROTOCOL_BUFFER_FRAMES);
        assert!(buffer.frames.iter().all(|(key, _, _)| key == "third"));
        drop(buffer);
        assert!(service.protocol_events("oversized").unwrap().is_empty());
    }
}

impl AgentAguiStreamService {
    pub fn new(sink: Arc<AguiEventSink>) -> Self {
        Self {
            sink,
            replay_store: None,
            transient_protocol: true,
            protocol_buffer: Arc::default(),
        }
    }

    pub fn with_replay_store(
        sink: Arc<AguiEventSink>,
        replay_store: Arc<dyn AgentAguiReplayStore>,
    ) -> Self {
        Self {
            sink,
            replay_store: Some(replay_store),
            transient_protocol: false,
            protocol_buffer: Arc::default(),
        }
    }
    /// Ordinary chat uses bounded in-memory reconnect evidence. Permanent
    /// protocol replay is an explicit debugging mode, separate from history.
    pub fn with_transient_protocol(mut self) -> Self {
        self.transient_protocol = true;
        self
    }
    pub(crate) fn records_transient_protocol(&self) -> bool {
        self.transient_protocol
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
        let memory_events = self
            .sink
            .replay(&stream_key)
            .into_iter()
            .filter_map(|event| translate_agui_event(&stream_key, &event))
            .collect::<Vec<_>>();
        let stored_events = match &self.replay_store {
            Some(store) => store
                .replay_agui_events(&stream_key)
                .map_err(AgentAguiStreamError::ReplayReadFailed)?,
            None => Vec::new(),
        };
        let events = merge_agui_events(stored_events, memory_events);
        let replay_incomplete = self.ensure_complete(&stream_key).is_err();
        Ok(AgentAguiEventSnapshot {
            replay_incomplete,
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
        if self.transient_protocol {
            return Ok(self
                .protocol_buffer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .frames
                .iter()
                .filter(|(key, _, _)| key == &stream_key)
                .map(|(_, frame, _)| frame.clone())
                .collect());
        }
        let memory_events = self
            .sink
            .replay(&stream_key)
            .into_iter()
            .flat_map(|event| translate_protocol_events(&stream_key, &event))
            .collect::<Vec<_>>();
        let stored_events = match &self.replay_store {
            Some(store) => {
                // Delivery diagnostics describe history, not read availability.
                // A past dropped frame must not block later runs in this session.
                store
                    .replay_protocol_events(&stream_key)
                    .map_err(AgentAguiStreamError::ReplayReadFailed)?
            }
            None => Vec::new(),
        };
        let mut events = merge_protocol_events(stored_events, memory_events);
        // Older replay rows retain their original bytes; the public read boundary
        // applies the same visibility projection as new native output writes.
        for event in &mut events {
            project_product_visibility(&mut event.data);
        }
        Ok(events)
    }

    pub fn record_protocol_event(
        &self,
        stream_key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        if self.transient_protocol {
            let bytes = serde_json::to_vec(&event)
                .map(|v| v.len() + stream_key.len())
                .unwrap_or(usize::MAX);
            if bytes > MAX_PROTOCOL_BUFFER_BYTES {
                return Ok(());
            }
            let mut buffer = self
                .protocol_buffer
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            while buffer.bytes.saturating_add(bytes) > MAX_PROTOCOL_BUFFER_BYTES
                || buffer.frames.len() >= MAX_PROTOCOL_BUFFER_FRAMES
            {
                if let Some((_, _, size)) = buffer.frames.pop_front() {
                    buffer.bytes = buffer.bytes.saturating_sub(size);
                } else {
                    break;
                }
            }
            buffer.bytes += bytes;
            buffer.frames.push_back((stream_key, event, bytes));
            return Ok(());
        }
        if let Some(store) = self.replay_store.as_ref() {
            if let Err(error) = store.append_protocol_event(&stream_key, event) {
                store.record_delivery_failure(&stream_key, &error);
                return Err(AgentAguiStreamError::ReplayWriteFailed(error));
            }
        }
        Ok(())
    }

    pub fn mark_incomplete(&self, stream_key: &str, detail: &str) {
        if let Some(store) = &self.replay_store {
            store.record_delivery_failure(stream_key, detail);
        }
    }

    pub fn ensure_complete(&self, stream_key: &str) -> Result<(), AgentAguiStreamError> {
        match self
            .replay_store
            .as_ref()
            .and_then(|store| store.replay_failure(stream_key))
        {
            Some(error) => Err(AgentAguiStreamError::ReplayIncomplete(error)),
            None => Ok(()),
        }
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

fn merge_agui_events(
    stored_events: Vec<AgentAguiEvent>,
    memory_events: Vec<AgentAguiEvent>,
) -> Vec<AgentAguiEvent> {
    let mut seen = BTreeSet::new();
    let mut merged = Vec::new();
    for event in stored_events.into_iter().chain(memory_events) {
        if seen.insert(event.id.clone()) {
            merged.push(event);
        }
    }
    merged
}

fn merge_protocol_events(
    stored_events: Vec<AgentAguiProtocolEvent>,
    memory_events: Vec<AgentAguiProtocolEvent>,
) -> Vec<AgentAguiProtocolEvent> {
    let mut seen = BTreeSet::new();
    let mut merged = Vec::new();
    for event in stored_events.into_iter().chain(memory_events) {
        // Repeated content is valid. Only a durable event identity is a dedup key.
        if let Some(key) = protocol_event_identity(&event) {
            if !seen.insert(key) {
                continue;
            }
        }
        merged.push(event);
    }
    merged
}

pub fn protocol_event_identity(event: &AgentAguiProtocolEvent) -> Option<String> {
    event
        .data
        .pointer("/metadata/tron/eventId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

// The lossless native/history codec deliberately retains private product reasoning.
// Its public transport projection follows the existing product visibility policy,
// at explicit SDK Content paths only; arbitrary user JSON is never traversed.
pub(crate) fn project_product_visibility(data: &mut Value) {
    let path = match data.get("type").and_then(Value::as_str) {
        Some("CUSTOM") if data.get("name").and_then(Value::as_str) == Some("framework.message") => {
            "/value/content"
        }
        Some("RUN_FINISHED") => "/result/content",
        _ => return,
    };
    if let Some(content) = data.pointer_mut(path).and_then(Value::as_array_mut) {
        content.retain_mut(|block| {
            if block.get("type").and_then(Value::as_str) != Some("json") {
                return true;
            }
            match block
                .pointer("/value/tronMessagePart/type")
                .and_then(Value::as_str)
            {
                Some(
                    "context" | "reasoning" | "compactionCheckpoint" | "toolCall" | "toolResult",
                ) => false,
                Some("reasoningSummary") => {
                    let Some(text) = block
                        .pointer("/value/tronMessagePart/text")
                        .and_then(Value::as_str)
                    else {
                        return false;
                    };
                    *block = serde_json::json!({"type":"text", "text":text});
                    true
                }
                _ => true,
            }
        });
    }
}
