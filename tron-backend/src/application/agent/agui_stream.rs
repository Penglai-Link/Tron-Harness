use std::{collections::BTreeMap, sync::Arc};

use crate::{
    domain::agent::{AgentAguiEvent, AgentAguiEventSnapshot, AgentAguiStreamState},
    shared::{
        event_bus::{AguiEventSink, BusEvent, BusOutcome, BusRouteClass},
        time::timestamp_from_millis,
    },
};

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
