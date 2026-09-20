use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::application::agent::AgentAguiStreamService;
use crate::domain::agent::{ModelStreamEvent, ModelStreamEventKind};

use super::protocol_replay::record_protocol_event;

pub(crate) struct SystemAgentLiveProtocolEvent {
    pub(crate) data: Value,
    pub(crate) sse_event: String,
}

impl SystemAgentLiveProtocolEvent {
    fn new(sse_event: &str, data: Value) -> Self {
        Self {
            data,
            sse_event: sse_event.to_string(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct SystemAgentLiveReplayRecorder {
    builder: Arc<Mutex<SystemAgentLiveEventBuilder>>,
    enabled: bool,
    service: AgentAguiStreamService,
    stream_key: String,
}

impl SystemAgentLiveReplayRecorder {
    pub(crate) fn new(
        service: AgentAguiStreamService,
        stream_key: &str,
        run_code: &str,
        enabled: bool,
    ) -> Self {
        Self {
            builder: Arc::new(Mutex::new(SystemAgentLiveEventBuilder::new(
                stream_key, run_code, enabled,
            ))),
            enabled,
            service,
            stream_key: stream_key.to_string(),
        }
    }

    pub(crate) fn ingest(&self, event: ModelStreamEvent) {
        if !self.enabled {
            return;
        }
        let payloads = self
            .builder
            .lock()
            .map(|mut builder| builder.ingest(event))
            .unwrap_or_default();
        self.record(payloads);
    }

    pub(crate) fn start(&self) {
        if !self.enabled {
            return;
        }
        let payloads = self
            .builder
            .lock()
            .map(|mut builder| builder.start())
            .unwrap_or_default();
        self.record(payloads);
    }

    pub(crate) fn finalize(&self) {
        if !self.enabled {
            return;
        }
        let payloads = self
            .builder
            .lock()
            .map(|mut builder| builder.finalize())
            .unwrap_or_default();
        self.record(payloads);
    }

    fn record(&self, payloads: Vec<SystemAgentLiveProtocolEvent>) {
        for payload in payloads {
            record_protocol_event(
                &self.service,
                &self.stream_key,
                &payload.sse_event,
                &payload.data,
            );
        }
    }
}

pub(crate) struct SystemAgentLiveEventBuilder {
    enabled: bool,
    run_started: bool,
    reasoning_started: bool,
    reasoning_message_started: bool,
    reasoning_ended: bool,
    stream_key: String,
    text_ended: bool,
    text_started: bool,
    reasoning_message_id: String,
    reasoning_raw_message_id: String,
    reasoning_fallback: String,
    reasoning_summary: String,
    reasoning_summary_snapshot: String,
    run_code: String,
    text_message_id: String,
}

impl SystemAgentLiveEventBuilder {
    pub(crate) fn new(stream_key: &str, run_code: &str, enabled: bool) -> Self {
        Self {
            enabled,
            run_started: false,
            reasoning_started: false,
            reasoning_message_started: false,
            reasoning_ended: false,
            stream_key: stream_key.to_string(),
            text_ended: false,
            text_started: false,
            reasoning_message_id: format!("{run_code}-reasoning"),
            reasoning_raw_message_id: format!("{run_code}-reasoning-raw"),
            reasoning_fallback: String::new(),
            reasoning_summary: String::new(),
            reasoning_summary_snapshot: String::new(),
            run_code: run_code.to_string(),
            text_message_id: format!("{run_code}-assistant"),
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn start(&mut self) -> Vec<SystemAgentLiveProtocolEvent> {
        if !self.enabled || self.run_started {
            return Vec::new();
        }
        self.run_started = true;
        vec![
            SystemAgentLiveProtocolEvent::new(
                "RUN_STARTED",
                json!({
                    "type": "RUN_STARTED",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "timestamp": current_stream_timestamp(),
                }),
            ),
            self.activity_snapshot_payload("PREPARING", None),
        ]
    }

    pub(crate) fn ingest(&mut self, event: ModelStreamEvent) -> Vec<SystemAgentLiveProtocolEvent> {
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

    pub(crate) fn finalize(&mut self) -> Vec<SystemAgentLiveProtocolEvent> {
        if !self.enabled {
            return Vec::new();
        }

        let mut payloads = Vec::new();
        if self.reasoning_started && !self.reasoning_ended {
            payloads.extend(self.close_reasoning_payloads());
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
            payloads.extend(self.start());
            payloads.push(self.activity_snapshot_payload("THINKING", None));
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
                self.reasoning_fallback.push_str(&delta);
                if self.reasoning_summary.trim().is_empty()
                    && let Some(summary) = stable_reasoning_summary(&self.reasoning_fallback)
                    && summary != self.reasoning_summary_snapshot
                {
                    self.reasoning_summary_snapshot = summary.clone();
                    payloads.push(self.activity_snapshot_payload("THINKING", Some(&summary)));
                }
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
                self.reasoning_summary.push_str(&delta);
                let summary = compact_reasoning_summary(&self.reasoning_summary);
                if !summary.is_empty() && summary != self.reasoning_summary_snapshot {
                    self.reasoning_summary_snapshot = summary.clone();
                    payloads.push(self.activity_snapshot_payload("THINKING", Some(&summary)));
                }
            }
            ModelStreamEventKind::Text => {}
        }
        payloads
    }

    fn text_event_payloads(&mut self, delta: String) -> Vec<SystemAgentLiveProtocolEvent> {
        let mut payloads = Vec::new();
        if self.reasoning_started && !self.reasoning_ended {
            payloads.extend(self.close_reasoning_payloads());
        }
        if !self.text_started {
            self.text_started = true;
            payloads.extend(self.start());
            payloads.push(self.activity_snapshot_payload("RESPONDING", None));
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

    fn activity_snapshot_payload(
        &self,
        activity: &str,
        summary: Option<&str>,
    ) -> SystemAgentLiveProtocolEvent {
        SystemAgentLiveProtocolEvent::new(
            "ACTIVITY_SNAPSHOT",
            json!({
                "type": "ACTIVITY_SNAPSHOT",
                "threadId": self.stream_key,
                "runId": self.run_code,
                "activity": activity,
                "summary": summary,
                "timestamp": current_stream_timestamp(),
            }),
        )
    }

    fn close_reasoning_payloads(&mut self) -> Vec<SystemAgentLiveProtocolEvent> {
        let timestamp = current_stream_timestamp();
        self.reasoning_ended = true;
        vec![
            SystemAgentLiveProtocolEvent::new(
                "REASONING_MESSAGE_END",
                json!({
                    "type": "REASONING_MESSAGE_END",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "messageId": self.reasoning_message_id,
                    "timestamp": timestamp,
                }),
            ),
            SystemAgentLiveProtocolEvent::new(
                "REASONING_END",
                json!({
                    "type": "REASONING_END",
                    "threadId": self.stream_key,
                    "runId": self.run_code,
                    "timestamp": timestamp,
                }),
            ),
        ]
    }
}

fn current_stream_timestamp() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn stable_reasoning_summary(content: &str) -> Option<String> {
    let content = content.trim();
    if content.is_empty() {
        return None;
    }

    let latest_complete_sentence = content
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, '.' | '!' | '?' | '。' | '！' | '？'))
        .map(|(end, _)| {
            let complete = &content[..=end];
            let start = complete
                .char_indices()
                .rev()
                .skip(1)
                .find(|(_, character)| {
                    matches!(character, '.' | '!' | '?' | '。' | '！' | '？')
                })
                .map_or(0, |(index, character)| index + character.len_utf8());
            complete[start..].trim()
        })
        .filter(|sentence| sentence.chars().count() >= 12)
        .map(compact_reasoning_summary);

    latest_complete_sentence
        .or_else(|| (content.chars().count() >= 72).then(|| compact_reasoning_summary(content)))
}

fn compact_reasoning_summary(content: &str) -> String {
    const MAX_SUMMARY_CHARS: usize = 160;
    let mut summary = content.chars().take(MAX_SUMMARY_CHARS).collect::<String>();
    if content.chars().count() > MAX_SUMMARY_CHARS {
        summary.push('…');
    }
    summary.trim().to_string()
}

#[cfg(test)]
mod tests {
    use crate::domain::agent::{ModelStreamEvent, ModelStreamEventKind};

    use super::SystemAgentLiveEventBuilder;

    #[test]
    fn starts_immediately_and_emits_stable_activity_stages() {
        let mut builder = SystemAgentLiveEventBuilder::new("SESSION-1", "RUN-1", true);

        let start = builder.start();
        assert_eq!(start.len(), 2);
        assert_eq!(start[0].sse_event, "RUN_STARTED");
        assert_eq!(start[1].data["activity"], "PREPARING");

        let text = builder.text_event_payloads("Hello".to_string());
        assert_eq!(text[0].sse_event, "ACTIVITY_SNAPSHOT");
        assert_eq!(text[0].data["activity"], "RESPONDING");
        assert_eq!(text[1].sse_event, "TEXT_MESSAGE_START");
    }

    #[test]
    fn snapshots_structured_reasoning_only_after_a_stable_summary_is_available() {
        let mut builder = SystemAgentLiveEventBuilder::new("SESSION-1", "RUN-1", true);
        let events = builder.ingest(ModelStreamEvent {
            kind: ModelStreamEventKind::ReasoningSummary,
            delta: "Reviewing the implementation plan and its dependencies.".to_string(),
        });

        let snapshot = events
            .iter()
            .find(|event| {
                event.sse_event == "ACTIVITY_SNAPSHOT" && event.data["summary"].is_string()
            })
            .expect("expected a stable reasoning summary snapshot");
        assert_eq!(snapshot.data["activity"], "THINKING");
        assert_eq!(
            snapshot.data["summary"],
            "Reviewing the implementation plan and its dependencies."
        );
    }

    #[test]
    fn snapshots_raw_reasoning_when_no_structured_summary_is_available() {
        let mut builder = SystemAgentLiveEventBuilder::new("SESSION-1", "RUN-1", true);
        let events = builder.ingest(ModelStreamEvent {
            kind: ModelStreamEventKind::Reasoning,
            delta: "Checking the required dependencies before implementation.".to_string(),
        });

        let snapshot = events
            .iter()
            .find(|event| {
                event.sse_event == "ACTIVITY_SNAPSHOT" && event.data["summary"].is_string()
            })
            .expect("expected a fallback reasoning summary snapshot");
        assert_eq!(snapshot.data["activity"], "THINKING");
        assert_eq!(
            snapshot.data["summary"],
            "Checking the required dependencies before implementation."
        );
    }
}
