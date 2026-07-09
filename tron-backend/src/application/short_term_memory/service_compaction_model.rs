use serde_json::json;

use crate::{
    domain::{
        agent::{
            AgentMessage, AgentModelConfig, AgentModelProtocol, ModelFinish, ModelRequest,
            ModelResponse, ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot,
        },
        short_term_memory::{
            ShortTermMemoryCompaction, ShortTermMemoryError, ShortTermMemoryMessage,
            ShortTermMemoryPart,
        },
    },
    shared::time::current_timestamp,
};

use super::{
    CompactionContextPolicy, CreateCompactionCommand, ModelCompactionFailure,
    ModelCompactionOutput, PreparedCompactionInput, ShortTermMemoryService,
    history_retention::{select_history_from_tail_start, select_retained_history},
    service_compaction::previous_checkpoint_context_from_parts,
    service_compaction_prompt::{CompactionPromptParts, hash_prompt, render_compaction_prompt},
    service_compaction_summary::compaction_transcript_summary,
    service_parts::estimate_text_tokens,
};

pub(super) const DEFAULT_COMPACTION_OUTPUT_MAX_TOKENS: u32 = 4_096;

impl ShortTermMemoryService {
    pub fn prepare_manual_model_compaction_input(
        &self,
        command: CreateCompactionCommand,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
    ) -> Result<PreparedCompactionInput, ShortTermMemoryError> {
        let policy = CompactionContextPolicy {
            todo_enabled: command.todo_enabled.unwrap_or(true),
        };
        self.prepare_model_compaction_input_with_trigger(
            command,
            policy,
            true,
            "MANUAL_CHECKPOINT",
            "user",
            "chat_page",
            context_window_tokens,
            max_output_tokens,
            None,
            None,
            None,
        )
    }

    pub(super) fn prepare_model_compaction_input_with_trigger(
        &self,
        command: CreateCompactionCommand,
        policy: CompactionContextPolicy,
        enforce_idle: bool,
        strategy: &str,
        triggered_by: &str,
        source: &str,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
        pinned_run_code: Option<&str>,
        requested_tail_start_seq: Option<i64>,
        requested_retained_history_tokens: Option<u32>,
    ) -> Result<PreparedCompactionInput, ShortTermMemoryError> {
        let session_code = command.session_code.trim();
        let workspace_code = self.ensure_session(session_code)?;
        if enforce_idle && self.repository().active_run_exists(session_code)? {
            return Err(ShortTermMemoryError::CompactionConflict);
        }

        let previous_checkpoint = self.repository().latest_compaction(session_code)?;
        let previous_checkpoint_context = previous_checkpoint
            .as_ref()
            .and_then(|checkpoint| {
                self.repository()
                    .message_parts(&checkpoint.checkpoint_message_code)
                    .ok()
            })
            .and_then(|parts| previous_checkpoint_context_from_parts(&parts, policy.todo_enabled));
        let min_seq = Self::checkpoint_replay_min_seq(previous_checkpoint.as_ref());
        let messages = self
            .repository()
            .list_messages_after_seq(session_code, min_seq)?;
        let effective_max_output_tokens =
            max_output_tokens.or(Some(DEFAULT_COMPACTION_OUTPUT_MAX_TOKENS));
        let budget_plan = self
            .config
            .model_context_budget_plan(context_window_tokens, effective_max_output_tokens);
        let head_only_tail_replayed = matches!(
            strategy,
            "AUTO_CHECKPOINT" | "OVERFLOW_CHECKPOINT" | "MANUAL_CHECKPOINT"
        );
        let retained_history_tokens = requested_retained_history_tokens.unwrap_or_else(|| {
            budget_plan.effective_input_budget.saturating_mul(
                self.config
                    .compaction
                    .retained_history_ratio_bps
                    .min(10_000),
            ) / 10_000
        });
        let selection = if let Some(tail_start_seq) = requested_tail_start_seq {
            select_history_from_tail_start(
                messages.clone(),
                tail_start_seq,
                pinned_run_code,
                self.config.context.reasoning_summary_replay_enabled,
            )
        } else {
            select_retained_history(
                messages.clone(),
                retained_history_tokens,
                pinned_run_code,
                self.config.context.reasoning_summary_replay_enabled,
            )
        };
        let source_start_seq = selection.source_start_seq;
        let selection_source_end_seq = selection.source_end_seq;
        let head_messages = selection.head_messages.clone();
        if head_only_tail_replayed && head_messages.is_empty() {
            return Err(ShortTermMemoryError::ContextBudgetExceeded);
        }
        let head_end_seq = head_messages
            .last()
            .map(|(message, _)| message.seq)
            .unwrap_or_else(|| selection.tail_start_seq.saturating_sub(1));
        let tail_start_seq = if head_only_tail_replayed {
            head_end_seq.saturating_add(1)
        } else {
            selection.tail_start_seq
        };
        let tail_messages = if head_only_tail_replayed && tail_start_seq < selection.tail_start_seq
        {
            messages
                .iter()
                .filter(|(message, _)| message.seq >= tail_start_seq)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            selection.tail_messages.clone()
        };
        let coverage = if head_only_tail_replayed {
            "head_only_tail_replayed"
        } else {
            "full_source_includes_tail"
        };
        let selector_version = if head_only_tail_replayed { 4 } else { 2 };
        let source_end_seq = if head_only_tail_replayed {
            head_end_seq
        } else {
            selection_source_end_seq
        };
        let (history_to_compact, recent) = compaction_transcript_summary(
            &head_messages,
            &tail_messages,
            self.config.context.reasoning_summary_replay_enabled,
        );
        let tail_token_estimate =
            if head_only_tail_replayed && tail_start_seq < selection.tail_start_seq {
                estimate_text_tokens(&recent)
            } else {
                selection.tail_token_estimate
            };
        let prompt = render_compaction_prompt(CompactionPromptParts {
            previous_checkpoint: previous_checkpoint_context.as_deref(),
            history_to_compact: &history_to_compact,
            retained_recent: &recent,
            reason: command.reason.as_deref(),
            note: command.note.as_deref(),
        });
        let prompt_hash = hash_prompt(&prompt);
        let compaction_input_token_estimate = estimate_text_tokens(&prompt);
        let max_input_tokens =
            self.compaction_input_budget(context_window_tokens, effective_max_output_tokens);

        let input = PreparedCompactionInput {
            session_code: session_code.to_string(),
            workspace_code: workspace_code.clone(),
            reason: command.reason.clone(),
            note: command.note.clone(),
            todo_enabled: policy.todo_enabled,
            enforce_idle,
            strategy: strategy.to_string(),
            triggered_by: triggered_by.to_string(),
            source: source.to_string(),
            previous_checkpoint_message_code: previous_checkpoint
                .as_ref()
                .map(|item| item.checkpoint_message_code.clone()),
            previous_checkpoint_included_recent: previous_checkpoint_context.is_some(),
            coverage: coverage.to_string(),
            selector_version,
            selection_source_end_seq,
            trigger_ratio_bps: self.config.compaction.trigger_ratio_bps,
            retained_history_ratio_bps: self.config.compaction.retained_history_ratio_bps,
            effective_retained_history_tokens: retained_history_tokens,
            source_start_seq,
            source_end_seq,
            tail_start_seq,
            source_message_count: head_messages.len(),
            selection_source_message_count: selection.source_message_count,
            source_token_estimate: estimate_text_tokens(&history_to_compact),
            tail_token_estimate,
            compaction_input_token_estimate,
            max_input_tokens,
            recent,
            prompt,
            prompt_hash,
            pinned_start_seq: selection.pinned_start_seq,
        };

        self.append_event(
            &input.session_code,
            &workspace_code,
            None,
            None,
            None,
            None,
            "agent.compaction.started",
            "COMPACTION",
            json!({
                "reason": input.reason,
                "sourceStartSeq": input.source_start_seq,
                "sourceEndSeq": input.source_end_seq,
                "selectionSourceEndSeq": input.selection_source_end_seq,
                "tailStartSeq": input.tail_start_seq,
                "pinnedStartSeq": input.pinned_start_seq,
                "sourceMessageCount": input.source_message_count,
                "selectionSourceMessageCount": input.selection_source_message_count,
                "sourceTokenEstimate": input.source_token_estimate,
                "tailTokenEstimate": input.tail_token_estimate,
                "compactionInputTokenEstimate": input.compaction_input_token_estimate,
                "maxInputTokens": input.max_input_tokens,
                "triggerRatioBps": input.trigger_ratio_bps,
                "retainedHistoryRatioBps": input.retained_history_ratio_bps,
                "effectiveRetainedHistoryTokens": input.effective_retained_history_tokens,
                "previousCheckpointMessageCode": input.previous_checkpoint_message_code,
                "promptHash": input.prompt_hash,
                "compactionMode": "model",
                "selectorVersion": input.selector_version,
                "coverage": input.coverage,
            }),
            None,
        )?;

        if input.compaction_input_token_estimate > input.max_input_tokens {
            self.record_compaction_failed(
                &input,
                &ModelCompactionFailure {
                    error_code: "COMPACTION_INPUT_TOO_LARGE".into(),
                    message: "compaction input is too large".into(),
                },
            )?;
            return Err(ShortTermMemoryError::CompactionInputTooLarge);
        }

        Ok(input)
    }

    pub fn compaction_model_config(&self, base: &AgentModelConfig) -> AgentModelConfig {
        let mut config = base.clone();
        config.max_tokens = Some(
            base.max_tokens
                .filter(|tokens| *tokens > 0)
                .unwrap_or(DEFAULT_COMPACTION_OUTPUT_MAX_TOKENS),
        );
        config.temperature = Some(0.0);
        config.top_p = Some(1.0);
        config.thinking_enabled = false;
        config.responses_stateful_enabled = false;
        config
    }

    pub fn compaction_model_request(&self, input: &PreparedCompactionInput) -> ModelRequest {
        ModelRequest {
            messages: vec![
                AgentMessage::system(
                    "Generate a short-term memory compaction checkpoint. Return final text only.",
                ),
                AgentMessage::user(input.prompt.clone()),
            ],
            tools: Vec::new(),
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        }
    }

    pub fn model_compaction_output(
        &self,
        response: &ModelResponse,
    ) -> Result<ModelCompactionOutput, ModelCompactionFailure> {
        if !response.tool_calls().is_empty() {
            return Err(ModelCompactionFailure {
                error_code: "COMPACTION_MODEL_TOOL_CALL".into(),
                message: "compaction model returned tool calls".into(),
            });
        }
        if response.finish != ModelFinish::Final {
            return Err(ModelCompactionFailure {
                error_code: "COMPACTION_MODEL_NON_FINAL".into(),
                message: "compaction model returned a non-final response".into(),
            });
        }
        let summary = response.output_text().trim().to_string();
        if summary.is_empty() {
            return Err(ModelCompactionFailure {
                error_code: "COMPACTION_MODEL_EMPTY_OUTPUT".into(),
                message: "compaction model returned empty output".into(),
            });
        }
        Ok(ModelCompactionOutput {
            summary,
            token_usage: response.token_usage(),
            provider_metadata: Some(json!({
                "responseId": response.response_id(),
            })),
        })
    }

    pub fn commit_model_compaction(
        &self,
        input: PreparedCompactionInput,
        output: ModelCompactionOutput,
        model: &AgentModelConfig,
    ) -> Result<ShortTermMemoryCompaction, ShortTermMemoryError> {
        let message_code = self.next_code("ATM")?;
        let summary_part_code = self.next_code("ATP")?;
        let recent_part_code = self.next_code("ATP")?;
        let now = current_timestamp();
        let summary_token_estimate = estimate_text_tokens(&output.summary);
        let model_protocol = model_protocol_code(model.protocol).to_string();
        let usage_estimate = compaction_usage_estimate(&input, model, &model_protocol);
        let usage_audit =
            compaction_usage_audit(usage_estimate.clone(), output.token_usage, &output);
        let compaction_metadata = json!({
            "reason": input.reason,
            "strategy": input.strategy,
            "triggeredBy": input.triggered_by,
            "source": input.source,
            "compactionMode": "model",
            "model": {
                "code": model.model_code,
                "provider": model.base_url,
                "name": model.model_name,
                "protocol": model_protocol,
            },
            "promptHash": input.prompt_hash,
            "tokenUsage": output.token_usage,
            "usageEstimate": usage_estimate,
            "usageAudit": usage_audit,
            "previousCheckpointMessageCode": input.previous_checkpoint_message_code,
            "previousCheckpointIncludedRecent": input.previous_checkpoint_included_recent,
            "sourceStartSeq": input.source_start_seq,
            "sourceEndSeq": input.source_end_seq,
            "selectionSourceEndSeq": input.selection_source_end_seq,
            "tailStartSeq": input.tail_start_seq,
            "pinnedStartSeq": input.pinned_start_seq,
            "sourceMessageCount": input.source_message_count,
            "selectionSourceMessageCount": input.selection_source_message_count,
            "sourceTokenEstimate": input.source_token_estimate,
            "tailTokenEstimate": input.tail_token_estimate,
            "compactionInputTokenEstimate": input.compaction_input_token_estimate,
            "summaryTokenEstimate": summary_token_estimate,
            "selectorVersion": input.selector_version,
            "coverage": input.coverage,
            "triggerRatioBps": input.trigger_ratio_bps,
            "retainedHistoryRatioBps": input.retained_history_ratio_bps,
            "effectiveRetainedHistoryTokens": input.effective_retained_history_tokens,
        });
        let token_usage = output.token_usage;
        let message = ShortTermMemoryMessage {
            id: 0,
            message_code: message_code.clone(),
            session_code: input.session_code.clone(),
            workspace_code: input.workspace_code.clone(),
            run_code: None,
            seq: 0,
            role: "checkpoint".into(),
            message_type: "compaction".into(),
            status: "completed".into(),
            agent_code: None,
            model_code: Some(model.model_code.clone()),
            model_provider: Some(model.base_url.clone()),
            model_name: Some(model.model_name.clone()),
            model_protocol: Some(model_protocol),
            finish_reason: Some("final".into()),
            error_json: None,
            input_tokens: token_usage.map(|usage| usage.input_tokens),
            output_tokens: token_usage.map(|usage| usage.output_tokens),
            reasoning_tokens: None,
            total_tokens: token_usage
                .map(|usage| usage.total_tokens)
                .or(Some(summary_token_estimate)),
            metadata_json: Some(compaction_metadata.clone()),
            created_at: now.clone(),
            completed_at: Some(now.clone()),
        };
        let summary_part = ShortTermMemoryPart {
            id: 0,
            part_code: summary_part_code,
            session_code: input.session_code.clone(),
            workspace_code: input.workspace_code.clone(),
            message_code: message_code.clone(),
            seq: 1,
            part_type: "compaction_summary".into(),
            status: "completed".into(),
            content_text: Some(output.summary.clone()),
            content_json: Some(json!({
                "recent": input.recent,
                "sourceStartSeq": input.source_start_seq,
                "sourceEndSeq": input.source_end_seq,
                "selectionSourceEndSeq": input.selection_source_end_seq,
                "tailStartSeq": input.tail_start_seq,
                "selectorVersion": input.selector_version,
                "coverage": input.coverage,
                "triggerRatioBps": input.trigger_ratio_bps,
                "retainedHistoryRatioBps": input.retained_history_ratio_bps,
                "effectiveRetainedHistoryTokens": input.effective_retained_history_tokens,
                "compactionMode": "model",
                "promptHash": input.prompt_hash,
            })),
            provider_metadata_json: output.provider_metadata,
            visibility: "model".into(),
            created_at: now.clone(),
            completed_at: Some(now.clone()),
        };
        let recent_part = ShortTermMemoryPart {
            id: 0,
            part_code: recent_part_code,
            session_code: input.session_code.clone(),
            workspace_code: input.workspace_code.clone(),
            message_code: message_code.clone(),
            seq: 2,
            part_type: "compaction_recent".into(),
            status: "completed".into(),
            content_text: Some(input.recent.clone()),
            content_json: Some(json!({
                "sourceStartSeq": input.tail_start_seq,
                "sourceEndSeq": input.selection_source_end_seq,
                "selectionSourceEndSeq": input.selection_source_end_seq,
                "coverage": input.coverage,
                "policy": {
                    "triggerRatioBps": input.trigger_ratio_bps,
                    "retainedHistoryRatioBps": input.retained_history_ratio_bps,
                    "effectiveRetainedHistoryTokens": input.effective_retained_history_tokens,
                    "reasoningSummaryReplayEnabled": self.config.context.reasoning_summary_replay_enabled,
                }
            })),
            provider_metadata_json: None,
            visibility: "model".into(),
            created_at: now.clone(),
            completed_at: Some(now.clone()),
        };
        let compaction = ShortTermMemoryCompaction {
            session_code: input.session_code.clone(),
            checkpoint_message_code: message_code.clone(),
            status: "completed".into(),
            source_start_seq: input.source_start_seq,
            source_end_seq: input.source_end_seq,
            tail_start_seq: input.tail_start_seq,
            selector_version: input.selector_version,
            coverage: Some(input.coverage.clone()),
            summary_token_estimate,
            created_at: now,
        };
        let event = self.new_event(
            &input.session_code,
            &input.workspace_code,
            None,
            None,
            Some(&message_code),
            None,
            "agent.compaction.completed",
            "COMPACTION",
            json!({"checkpointMessageCode": message_code, "metadata": compaction_metadata}),
            None,
        )?;
        self.repository().create_compaction(
            &compaction,
            &message,
            &[summary_part, recent_part],
            event.as_ref(),
            input.enforce_idle,
        )?;
        Ok(compaction)
    }
}

fn model_protocol_code(protocol: AgentModelProtocol) -> &'static str {
    match protocol {
        AgentModelProtocol::ChatCompletions => "chatCompletions",
        AgentModelProtocol::Responses => "responses",
        AgentModelProtocol::Embeddings => "embeddings",
    }
}

fn compaction_usage_estimate(
    input: &PreparedCompactionInput,
    model: &AgentModelConfig,
    model_protocol: &str,
) -> TokenEstimateSnapshot {
    TokenEstimateSnapshot {
        estimated_input_tokens: input.compaction_input_token_estimate,
        message_token_estimate: input.compaction_input_token_estimate,
        tool_schema_token_estimate: 0,
        context_window_limit: model.context_window_tokens,
        effective_input_budget: Some(input.max_input_tokens),
        output_token_reserve: model.max_tokens,
        message_count: 2,
        part_count: 2,
        rebased: false,
        over_budget: input.compaction_input_token_estimate > input.max_input_tokens,
        checkpoint_code: input.previous_checkpoint_message_code.clone(),
        model_code: model.model_code.clone(),
        model_name: model.model_name.clone(),
        model_protocol: model_protocol.to_string(),
        usage_source: "local_estimate".into(),
        usage_confidence: "estimated".into(),
        compaction_trigger_reached: false,
    }
}

fn compaction_usage_audit(
    request_estimate: TokenEstimateSnapshot,
    provider_usage: Option<ModelTokenUsage>,
    output: &ModelCompactionOutput,
) -> TokenUsageAuditSnapshot {
    let input_token_delta = provider_usage.map(|usage| {
        i64::from(usage.input_tokens) - i64::from(request_estimate.estimated_input_tokens)
    });
    let input_token_ratio = provider_usage.and_then(|usage| {
        if request_estimate.estimated_input_tokens == 0 {
            None
        } else {
            Some(f64::from(usage.input_tokens) / f64::from(request_estimate.estimated_input_tokens))
        }
    });
    let usage_source = if provider_usage.is_some() {
        "provider"
    } else {
        "unavailable"
    };
    let usage_confidence = if provider_usage.is_some() {
        "actual"
    } else {
        "unavailable"
    };
    TokenUsageAuditSnapshot {
        request_estimate,
        provider_usage,
        cumulative_run_usage: None,
        input_token_delta,
        input_token_ratio,
        calibration_factor: input_token_ratio,
        response_id: output
            .provider_metadata
            .as_ref()
            .and_then(|value| value.get("responseId"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        usage_source: usage_source.into(),
        usage_confidence: usage_confidence.into(),
    }
}
