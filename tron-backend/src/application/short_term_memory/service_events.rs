use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    domain::short_term_memory::{ShortTermMemoryError, ShortTermMemoryEvent},
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef},
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
        let payload_for_live_event = payload_json.clone();
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
            self.publish_context_event(
                session_code,
                workspace_code,
                run_code,
                event_type,
                &self.bounded_event_payload(payload_for_live_event),
            );
            return Ok(());
        };
        self.repository().create_event(&event)?;
        self.publish_context_event(
            session_code,
            workspace_code,
            run_code,
            event_type,
            &event.payload_json,
        );
        Ok(())
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

    pub(super) fn publish_context_event(
        &self,
        session_code: &str,
        workspace_code: &str,
        run_code: Option<&str>,
        event_type: &str,
        payload_json: &Value,
    ) {
        let Some(outcome) = context_event_outcome(event_type) else {
            return;
        };
        let mut draft = BusEventDraft::agui(event_type.to_string(), "short_term_memory")
            .outcome(outcome)
            .request(BusRequestRef::with_stream_key(session_code.to_string()))
            .attr("workspace_code", workspace_code.to_string())
            .attr("conversation_code", session_code.to_string())
            .payload_field(
                "context_event_json",
                agui_context_payload(event_type, payload_json),
            );
        draft = match run_code {
            Some(run_code) => draft.subject("agent_run", run_code.to_string()),
            None => draft.subject("chat_session", session_code.to_string()),
        };
        let _ = self.event_bus.publish(draft);
    }
}

fn context_event_outcome(event_type: &str) -> Option<BusOutcome> {
    match event_type {
        "agent.context.compaction_requested" => Some(BusOutcome::Progress),
        "agent.compaction.started" => Some(BusOutcome::Started),
        "agent.context.assembled" | "agent.context.rebased" | "agent.compaction.completed" => {
            Some(BusOutcome::Completed)
        }
        "agent.compaction.failed" => Some(BusOutcome::Failure),
        _ => None,
    }
}

fn agui_context_payload(event_type: &str, payload_json: &Value) -> String {
    const MAX_ENCODED_BYTES: usize = 7_000;
    let safe_payload = match event_type {
        "agent.context.assembled" => json!({
            "estimatedTokens": payload_json.get("estimatedTokens"),
            "toolSchemaTokenEstimate": payload_json.get("toolSchemaTokenEstimate"),
            "rebased": payload_json.get("rebased"),
            "overBudget": payload_json.get("overBudget"),
            "budgetBreakdown": payload_json.get("budgetBreakdown"),
            "overBudgetReason": payload_json.get("overBudgetReason"),
        }),
        "agent.context.compaction_requested" => json!({
            "estimatedTokens": payload_json.get("estimatedTokens"),
            "fullEstimatedTokens": payload_json.get("fullEstimatedTokens"),
            "toolSchemaTokenEstimate": payload_json.get("toolSchemaTokenEstimate"),
            "triggerTokens": payload_json.get("triggerTokens"),
            "compactionTriggerReached": payload_json.get("compactionTriggerReached"),
            "effectiveRetainedHistoryTokens": payload_json.get("effectiveRetainedHistoryTokens"),
            "budgetBreakdown": payload_json.get("budgetBreakdown"),
        }),
        "agent.context.rebased" => json!({
            "reason": payload_json.get("reason"),
        }),
        "agent.compaction.started" => json!({
            "sourceStartSeq": payload_json.get("sourceStartSeq"),
            "sourceEndSeq": payload_json.get("sourceEndSeq"),
            "tailStartSeq": payload_json.get("tailStartSeq"),
            "sourceMessageCount": payload_json.get("sourceMessageCount"),
            "sourceTokenEstimate": payload_json.get("sourceTokenEstimate"),
            "tailTokenEstimate": payload_json.get("tailTokenEstimate"),
            "compactionInputTokenEstimate": payload_json.get("compactionInputTokenEstimate"),
            "maxInputTokens": payload_json.get("maxInputTokens"),
            "compactionMode": payload_json.get("compactionMode"),
            "coverage": payload_json.get("coverage"),
        }),
        "agent.compaction.completed" => {
            let metadata = payload_json.get("metadata").unwrap_or(&Value::Null);
            json!({
                "checkpointMessageCode": payload_json.get("checkpointMessageCode"),
                "strategy": metadata.get("strategy"),
                "triggeredBy": metadata.get("triggeredBy"),
                "source": metadata.get("source"),
                "sourceStartSeq": metadata.get("sourceStartSeq"),
                "sourceEndSeq": metadata.get("sourceEndSeq"),
                "tailStartSeq": metadata.get("tailStartSeq"),
                "sourceMessageCount": metadata.get("sourceMessageCount"),
                "sourceTokenEstimate": metadata.get("sourceTokenEstimate"),
                "tailTokenEstimate": metadata.get("tailTokenEstimate"),
                "compactionInputTokenEstimate": metadata.get("compactionInputTokenEstimate"),
                "summaryTokenEstimate": metadata.get("summaryTokenEstimate"),
                "coverage": metadata.get("coverage"),
            })
        }
        "agent.compaction.failed" => json!({
            "errorCode": payload_json.get("errorCode"),
            "compactionInputTokenEstimate": payload_json.get("compactionInputTokenEstimate"),
            "maxInputTokens": payload_json.get("maxInputTokens"),
            "strategy": payload_json.get("strategy"),
        }),
        _ => json!({}),
    };
    let Ok(encoded) = serde_json::to_string(&safe_payload) else {
        return "{}".to_string();
    };
    if encoded.len() <= MAX_ENCODED_BYTES {
        return encoded;
    }
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    json!({
        "truncated": true,
        "originalBytes": encoded.len(),
        "sha256": format!("sha256:{}", hex::encode(hasher.finalize())),
        "preview": truncate_chars(&encoded, 1_024),
    })
    .to_string()
}

fn is_run_lifecycle_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "agent.run.started"
            | "agent.run.completed"
            | "agent.run.awaiting_input"
            | "agent.run.failed"
            | "agent.run.recovered"
    )
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}
