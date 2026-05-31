use serde_json::{Value, json};

use crate::domain::agent::{ModelStreamEvent, ModelStreamEventKind};

pub(super) struct SystemAgentLiveProtocolEvent {
    pub(super) data: Value,
    pub(super) sse_event: String,
}

impl SystemAgentLiveProtocolEvent {
    fn new(sse_event: &str, data: Value) -> Self {
        Self {
            data,
            sse_event: sse_event.to_string(),
        }
    }
}

pub(super) struct SystemAgentLiveEventBuilder {
    enabled: bool,
    reasoning_started: bool,
    reasoning_message_started: bool,
    reasoning_ended: bool,
    stream_key: String,
    text_ended: bool,
    text_started: bool,
    reasoning_message_id: String,
    reasoning_raw_message_id: String,
    run_code: String,
    text_message_id: String,
}

impl SystemAgentLiveEventBuilder {
    pub(super) fn new(stream_key: &str, run_code: &str, enabled: bool) -> Self {
        Self {
            enabled,
            reasoning_started: false,
            reasoning_message_started: false,
            reasoning_ended: false,
            stream_key: stream_key.to_string(),
            text_ended: false,
            text_started: false,
            reasoning_message_id: format!("{run_code}-reasoning"),
            reasoning_raw_message_id: format!("{run_code}-reasoning-raw"),
            run_code: run_code.to_string(),
            text_message_id: format!("{run_code}-assistant"),
        }
    }

    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(super) fn ingest(&mut self, event: ModelStreamEvent) -> Vec<SystemAgentLiveProtocolEvent> {
        if !self.enabled {
            return Vec::new();
        }

        match event.kind {
            ModelStreamEventKind::Reasoning | ModelStreamEventKind::ReasoningSummary => {
                self.reasoning_event_payloads(event)
            }
            ModelStreamEventKind::Text => self.text_event_payloads(event.delta),
        }
    }

    pub(super) fn finalize(&mut self) -> Vec<SystemAgentLiveProtocolEvent> {
        if !self.enabled {
            return Vec::new();
        }

        let mut payloads = Vec::new();
        if self.reasoning_started && !self.reasoning_ended {
            let timestamp = current_stream_timestamp();
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "REASONING_MESSAGE_END",
                json!({
                    "type": "REASONING_MESSAGE_END",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "messageId": self.reasoning_message_id,
                    "timestamp": timestamp,
                }),
            ));
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "REASONING_END",
                json!({
                    "type": "REASONING_END",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "timestamp": timestamp,
                }),
            ));
            self.reasoning_ended = true;
        }
        if self.text_started && !self.text_ended {
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "TEXT_MESSAGE_END",
                json!({
                    "type": "TEXT_MESSAGE_END",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "messageId": self.text_message_id,
                    "timestamp": current_stream_timestamp(),
                }),
            ));
            self.text_ended = true;
        }
        payloads
    }

    fn reasoning_event_payloads(
        &mut self,
        event: ModelStreamEvent,
    ) -> Vec<SystemAgentLiveProtocolEvent> {
        let mut payloads = Vec::new();
        if !self.reasoning_started {
            self.reasoning_started = true;
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "RUN_STARTED",
                json!({
                    "type": "RUN_STARTED",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "timestamp": current_stream_timestamp(),
                }),
            ));
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "REASONING_START",
                json!({
                    "type": "REASONING_START",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "timestamp": current_stream_timestamp(),
                }),
            ));
        }
        if !self.reasoning_message_started {
            self.reasoning_message_started = true;
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "REASONING_MESSAGE_START",
                json!({
                    "type": "REASONING_MESSAGE_START",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "messageId": self.reasoning_message_id,
                    "timestamp": current_stream_timestamp(),
                }),
            ));
        }
        let delta = event.delta;
        if delta.trim().is_empty() {
            return payloads;
        }

        match event.kind {
            ModelStreamEventKind::Reasoning => {
                payloads.push(SystemAgentLiveProtocolEvent::new(
                    "REASONING_TEXT_CONTENT",
                    json!({
                        "type": "REASONING_TEXT_CONTENT",
                        "threadId": self.stream_key,
                        "runId": self.run_code,
                        "messageId": self.reasoning_raw_message_id,
                        "delta": delta,
                        "timestamp": current_stream_timestamp(),
                    }),
                ));
            }
            ModelStreamEventKind::ReasoningSummary => {
                payloads.push(SystemAgentLiveProtocolEvent::new(
                    "REASONING_MESSAGE_CONTENT",
                    json!({
                        "type": "REASONING_MESSAGE_CONTENT",
                        "threadId": self.stream_key,
                        "runId": self.run_code,
                        "messageId": self.reasoning_message_id,
                        "delta": delta,
                        "timestamp": current_stream_timestamp(),
                    }),
                ));
            }
            ModelStreamEventKind::Text => {}
        }
        payloads
    }

    fn text_event_payloads(&mut self, delta: String) -> Vec<SystemAgentLiveProtocolEvent> {
        let mut payloads = Vec::new();
        if !self.text_started {
            self.text_started = true;
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "RUN_STARTED",
                json!({
                    "type": "RUN_STARTED",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "timestamp": current_stream_timestamp(),
                }),
            ));
            payloads.push(SystemAgentLiveProtocolEvent::new(
                "TEXT_MESSAGE_START",
                json!({
                    "type": "TEXT_MESSAGE_START",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "messageId": self.text_message_id,
                    "role": "assistant",
                    "timestamp": current_stream_timestamp(),
                }),
            ));
        }
        if delta.trim().is_empty() {
            return payloads;
        }
        payloads.push(SystemAgentLiveProtocolEvent::new(
            "TEXT_MESSAGE_CONTENT",
            json!({
                "type": "TEXT_MESSAGE_CONTENT",
                "threadId": self.stream_key,
                "runId": self.run_code,
                "messageId": self.text_message_id,
                "delta": delta,
                "timestamp": current_stream_timestamp(),
            }),
        ));
        payloads
    }
}

fn current_stream_timestamp() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
