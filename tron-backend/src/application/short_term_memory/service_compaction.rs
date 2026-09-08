use serde_json::json;

use crate::domain::short_term_memory::{
    ShortTermMemoryCompaction, ShortTermMemoryError, ShortTermMemoryPart,
};

use super::{
    CompactionContextPolicy, CreateCompactionCommand, ModelCompactionFailure,
    PreparedCompactionInput, ShortTermMemoryService,
    service_compaction_model::DEFAULT_COMPACTION_OUTPUT_MAX_TOKENS,
    service_compaction_summary::{redact_todo_sections, truncate_chars},
};

impl ShortTermMemoryService {
    pub fn latest_compaction(
        &self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryCompaction>, ShortTermMemoryError> {
        self.ensure_session(session_code)?;
        self.repository().latest_compaction(session_code)
    }

    pub fn create_compaction(
        &self,
        command: CreateCompactionCommand,
    ) -> Result<ShortTermMemoryCompaction, ShortTermMemoryError> {
        let input = self.prepare_manual_model_compaction_input(command, None, None)?;
        let failure = ModelCompactionFailure {
            error_code: "COMPACTION_MODEL_REQUIRED".into(),
            message: "manual compaction requires a model-generated checkpoint summary".into(),
        };
        self.record_compaction_failed(&input, &failure)?;
        Err(ShortTermMemoryError::CompactionModelFailed)
    }

    pub fn session_workspace_for_compaction(
        &self,
        session_code: &str,
    ) -> Result<String, ShortTermMemoryError> {
        self.ensure_session(session_code)
    }

    pub fn prepare_automatic_model_compaction_input(
        &self,
        session_code: &str,
        reason: &str,
        policy: CompactionContextPolicy,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
        pinned_run_code: Option<&str>,
        tail_start_seq: Option<i64>,
        retained_history_tokens: Option<u32>,
    ) -> Result<PreparedCompactionInput, ShortTermMemoryError> {
        self.prepare_model_compaction_input_with_trigger(
            CreateCompactionCommand {
                session_code: session_code.to_string(),
                reason: Some(reason.to_string()),
                note: None,
                todo_enabled: Some(policy.todo_enabled),
            },
            policy,
            false,
            "AUTO_CHECKPOINT",
            "runtime",
            "runtime_context_budget",
            context_window_tokens,
            max_output_tokens,
            pinned_run_code,
            tail_start_seq,
            retained_history_tokens,
        )
    }

    pub fn prepare_overflow_model_compaction_input(
        &self,
        session_code: &str,
        provider_error: &str,
        policy: CompactionContextPolicy,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
        pinned_run_code: Option<&str>,
        tail_start_seq: Option<i64>,
        retained_history_tokens: Option<u32>,
    ) -> Result<PreparedCompactionInput, ShortTermMemoryError> {
        self.prepare_model_compaction_input_with_trigger(
            CreateCompactionCommand {
                session_code: session_code.to_string(),
                reason: Some("provider_context_overflow".to_string()),
                note: Some(truncate_chars(provider_error, 512)),
                todo_enabled: Some(policy.todo_enabled),
            },
            policy,
            false,
            "OVERFLOW_CHECKPOINT",
            "runtime",
            "provider_context_overflow",
            context_window_tokens,
            max_output_tokens,
            pinned_run_code,
            tail_start_seq,
            retained_history_tokens,
        )
    }

    pub fn overflow_retry_limit(&self) -> u32 {
        1
    }

    pub(super) fn checkpoint_replay_min_seq(compaction: Option<&ShortTermMemoryCompaction>) -> i64 {
        checkpoint_replay_min_seq(compaction)
    }

    pub(super) fn compaction_input_budget(
        &self,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
    ) -> u32 {
        let output_reserve = max_output_tokens.or(Some(DEFAULT_COMPACTION_OUTPUT_MAX_TOKENS));
        let budget_plan = self
            .config
            .model_context_budget_plan(context_window_tokens, output_reserve);
        budget_plan.effective_input_budget
    }

    pub fn record_compaction_failed(
        &self,
        input: &PreparedCompactionInput,
        failure: &ModelCompactionFailure,
    ) -> Result<(), ShortTermMemoryError> {
        self.append_event(
            &input.session_code,
            &input.workspace_code,
            input.run_code.as_deref(),
            None,
            None,
            None,
            "agent.compaction.failed",
            "COMPACTION",
            json!({
                "errorCode": failure.error_code,
                "message": failure.message,
                "compactionInputTokenEstimate": input.compaction_input_token_estimate,
                "maxInputTokens": input.max_input_tokens,
                "promptHash": input.prompt_hash,
                "strategy": input.strategy,
            }),
            None,
        )
    }
}

fn checkpoint_replay_min_seq(compaction: Option<&ShortTermMemoryCompaction>) -> i64 {
    compaction
        .map(|item| {
            if item.selector_version >= 2
                && item.coverage.as_deref() == Some("full_source_includes_tail")
            {
                item.source_end_seq.saturating_add(1)
            } else if item.selector_version >= 3
                && item.coverage.as_deref() == Some("head_only_tail_replayed")
            {
                item.tail_start_seq
            } else {
                item.tail_start_seq
            }
        })
        .unwrap_or(0)
}

pub(super) fn previous_checkpoint_context_from_parts(
    parts: &[ShortTermMemoryPart],
    todo_enabled: bool,
) -> Option<String> {
    let summary = parts
        .iter()
        .find(|part| part.part_type == "compaction_summary")
        .and_then(|part| part.content_text.as_deref())
        .unwrap_or_default();
    let summary_coverage = parts
        .iter()
        .find(|part| part.part_type == "compaction_summary")
        .and_then(|part| part.content_json.as_ref())
        .and_then(|value| value.get("coverage"))
        .and_then(|value| value.as_str());
    let include_recent = summary_coverage != Some("head_only_tail_replayed");
    let recent = if include_recent {
        parts
            .iter()
            .find(|part| part.part_type == "compaction_recent")
            .and_then(|part| part.content_text.as_deref())
            .or_else(|| {
                parts
                    .iter()
                    .find(|part| part.part_type == "compaction_summary")
                    .and_then(|part| part.content_json.as_ref())
                    .and_then(|value| value.get("recent"))
                    .and_then(|value| value.as_str())
            })
            .unwrap_or_default()
    } else {
        ""
    };
    let summary = if todo_enabled {
        summary.trim().to_string()
    } else {
        redact_todo_sections(summary)
    };
    let mut sections = Vec::new();
    if !summary.trim().is_empty() {
        sections.push(summary);
    }
    if !recent.trim().is_empty() {
        sections.push(format!(
            "Previous retained recent transcript:\n{}",
            truncate_chars(recent, 12_000)
        ));
    }
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}
