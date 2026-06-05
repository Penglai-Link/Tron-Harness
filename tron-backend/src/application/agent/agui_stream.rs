use std::{collections::BTreeSet, sync::Arc};

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
}

pub trait AgentAguiReplayStore: Send + Sync {
    fn append_protocol_event(
        &self,
        stream_key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), String>;

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
}

impl AgentAguiStreamService {
    pub fn new(sink: Arc<AguiEventSink>) -> Self {
        Self {
            sink,
            replay_store: None,
        }
    }

    pub fn with_replay_store(
        sink: Arc<AguiEventSink>,
        replay_store: Arc<dyn AgentAguiReplayStore>,
    ) -> Self {
        Self {
            sink,
            replay_store: Some(replay_store),
        }
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
        let stored_events = self
            .replay_store
            .as_ref()
            .and_then(|store| store.replay_agui_events(&stream_key).ok())
            .unwrap_or_default();
        let events = merge_agui_events(stored_events, memory_events);
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
        let memory_events = self
            .sink
            .replay(&stream_key)
            .into_iter()
            .flat_map(|event| translate_protocol_events(&stream_key, &event))
            .collect::<Vec<_>>();
        let stored_events = self
            .replay_store
            .as_ref()
            .and_then(|store| store.replay_protocol_events(&stream_key).ok())
            .unwrap_or_default();
        Ok(merge_protocol_events(stored_events, memory_events))
    }

    pub fn record_protocol_event(
        &self,
        stream_key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), AgentAguiStreamError> {
        let stream_key = normalized_stream_key(stream_key)?;
        if let Some(store) = self.replay_store.as_ref() {
            let _ = store.append_protocol_event(&stream_key, event);
        }
        Ok(())
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
        let key = serde_json::to_string(&event).unwrap_or_else(|_| format!("{event:?}"));
        if seen.insert(key) {
            merged.push(event);
        }
    }
    merged
}
