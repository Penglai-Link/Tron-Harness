use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiStreamState {
    pub stream_key: String,
    pub active_streams: usize,
    pub total_streams: usize,
    pub total_replay_bytes: usize,
    pub replay_evicted: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiEventSnapshot {
    pub stream_key: String,
    pub active_streams: usize,
    pub total_streams: usize,
    pub events: Vec<AgentAguiEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub event_name: String,
    pub run_code: Option<String>,
    pub stream_key: String,
    pub occurred_at: String,
    pub outcome: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAguiProtocolEvent {
    pub sse_event: String,
    pub data: Value,
}
