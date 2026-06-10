use serde_json::Value;

use crate::{application::agent::AgentAguiStreamService, domain::agent::AgentAguiProtocolEvent};

pub(crate) fn record_protocol_event(
    service: &AgentAguiStreamService,
    stream_key: &str,
    sse_event: &str,
    data: &Value,
) {
    let _ = service.record_protocol_event(
        stream_key,
        AgentAguiProtocolEvent {
            sse_event: sse_event.to_string(),
            data: data.clone(),
        },
    );
}
