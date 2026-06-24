use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    domain::short_term_memory::{ShortTermMemoryError, ShortTermMemoryEvent},
    shared::time::{current_timestamp, current_timestamp_seconds},
};

use super::ShortTermMemoryService;

const EVENT_PAYLOAD_PREVIEW_CHARS: usize = 512;

impl ShortTermMemoryService {
    pub(super) fn append_event(
        &self,
        session_code: &str,
        workspace_code: &str,
        run_code: Option<&str>,
        input_code: Option<&str>,
        message_code: Option<&str>,
        part_code: Option<&str>,
        event_type: &str,
        event_source: &str,
        payload_json: Value,
        metadata_json: Option<Value>,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(event) = self.new_event(
            session_code,
            workspace_code,
            run_code,
            input_code,
            message_code,
            part_code,
            event_type,
            event_source,
            payload_json,
            metadata_json,
        )?
        else {
            return Ok(());
        };
        self.repository().create_event(&event)
    }

    pub(super) fn append_run_lifecycle_event(
        &self,
        session_code: &str,
        workspace_code: &str,
        run_code: Option<&str>,
        input_code: Option<&str>,
        message_code: Option<&str>,
        part_code: Option<&str>,
        event_type: &str,
        event_source: &str,
        payload_json: Value,
        metadata_json: Option<Value>,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(event) = self.new_event(
            session_code,
            workspace_code,
            run_code,
            input_code,
            message_code,
            part_code,
            event_type,
            event_source,
            payload_json,
            metadata_json,
        )?
        else {
            return Ok(());
        };
        self.repository().create_event_and_update_run_lease(&event)
    }

    pub(super) fn new_event(
        &self,
        session_code: &str,
        workspace_code: &str,
        run_code: Option<&str>,
        input_code: Option<&str>,
        message_code: Option<&str>,
        part_code: Option<&str>,
        event_type: &str,
        event_source: &str,
        payload_json: Value,
        metadata_json: Option<Value>,
    ) -> Result<Option<ShortTermMemoryEvent>, ShortTermMemoryError> {
        if !self.should_persist_event(event_type) {
            return Ok(None);
        }
        let now = current_timestamp();
        Ok(Some(ShortTermMemoryEvent {
            id: 0,
            event_code: self.next_code("ATE")?,
            session_code: session_code.to_string(),
            workspace_code: workspace_code.to_string(),
            seq: 0,
            run_code: run_code.map(str::to_string),
            input_code: input_code.map(str::to_string),
            message_code: message_code.map(str::to_string),
            part_code: part_code.map(str::to_string),
            event_type: event_type.into(),
            event_source: event_source.into(),
            payload_json: self.bounded_event_payload(payload_json),
            metadata_json,
            occurred_at: now.clone(),
            created_at: now,
        }))
    }

    pub fn cleanup_event_log_on_startup(&self) -> Result<usize, ShortTermMemoryError> {
        let config = &self.config.event_log;
        if !config.cleanup_on_startup {
            return Ok(0);
        }
        let cutoff_ms = current_timestamp_seconds()
            .saturating_sub(i64::from(config.retention_days) * 86_400)
            .saturating_mul(1_000);
        self.repository().cleanup_event_log_before(
            cutoff_ms,
            config.cleanup_batch_size,
            config.cleanup_max_rows_per_startup,
        )
    }

    fn should_persist_event(&self, event_type: &str) -> bool {
        match self.config.event_log.mode.as_str() {
            "lifecycleOnly" => is_run_lifecycle_event(event_type),
            _ => true,
        }
    }

    fn bounded_event_payload(&self, payload_json: Value) -> Value {
        let max_bytes = self.config.event_log.max_payload_bytes;
        let Ok(encoded) = serde_json::to_string(&payload_json) else {
            return payload_json;
        };
        if encoded.len() <= max_bytes {
            return payload_json;
        }
        let mut hasher = Sha256::new();
        hasher.update(encoded.as_bytes());
        let digest = format!("sha256:{}", hex::encode(hasher.finalize()));
        let mut preview_chars = EVENT_PAYLOAD_PREVIEW_CHARS.min(max_bytes / 4);
        loop {
            let bounded = json!({
                "truncated": true,
                "originalBytes": encoded.len(),
                "sha256": digest.as_str(),
                "preview": truncate_chars(&encoded, preview_chars),
            });
            if serde_json::to_string(&bounded)
                .map(|value| value.len() <= max_bytes)
                .unwrap_or(true)
                || preview_chars == 0
            {
                return bounded;
            }
            preview_chars /= 2;
        }
    }
}

fn is_run_lifecycle_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "agent.run.started" | "agent.run.completed" | "agent.run.failed" | "agent.run.recovered"
    )
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}
