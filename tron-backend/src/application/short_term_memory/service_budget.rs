use super::{ModelContextBudgetPlan, PreparedModelContext, ShortTermMemoryConfig};

pub(super) fn estimate_tool_schema_tokens(tools: &[crate::domain::agent::EffectiveTool]) -> u32 {
    serde_json::to_string(tools)
        .ok()
        .map(|text| ((text.len() as u32).saturating_add(3)) / 4)
        .unwrap_or(0)
}

pub(super) fn apply_request_budget_fields(
    prepared: &mut PreparedModelContext,
    budget_plan: &ModelContextBudgetPlan,
    config: &ShortTermMemoryConfig,
    tool_schema_token_estimate: u32,
) {
    prepared.estimated_tokens = prepared
        .message_token_estimate
        .saturating_add(tool_schema_token_estimate);
    prepared.tool_schema_token_estimate = tool_schema_token_estimate;
    prepared.context_window_limit = Some(budget_plan.model_context_window);
    prepared.effective_input_budget = Some(budget_plan.effective_input_budget);
    prepared.output_token_reserve = Some(budget_plan.output_token_reserve);
    let trigger_tokens = compaction_trigger_tokens(budget_plan, Some(config));
    prepared.compaction_trigger_tokens = Some(trigger_tokens);
    prepared.compaction_trigger_reached = prepared.estimated_tokens >= trigger_tokens;
    let input_budget_exceeded = prepared.estimated_tokens > budget_plan.effective_input_budget;
    prepared.over_budget = input_budget_exceeded || prompt_prefix_budget_exceeded(prepared);
    if let Some(breakdown) = prepared.budget_breakdown.as_mut() {
        breakdown.tool_schema_tokens = tool_schema_token_estimate;
        breakdown.estimated_input_tokens = prepared.estimated_tokens;
        breakdown.effective_input_budget = Some(budget_plan.effective_input_budget);
        breakdown.context_window_limit = Some(budget_plan.model_context_window);
        breakdown.output_token_reserve = Some(budget_plan.output_token_reserve);
        breakdown.prompt_prefix_budget_tokens = Some(
            budget_plan
                .block_budgets
                .seed_prompt_prefix
                .effective_limit_tokens,
        );
        breakdown.compaction_trigger_ratio_bps = Some(config.compaction.trigger_ratio_bps);
        breakdown.compaction_trigger_tokens = Some(trigger_tokens);
        breakdown.compaction_trigger_reached = prepared.compaction_trigger_reached;
        breakdown.input_budget_exceeded = input_budget_exceeded;
    }
    refresh_over_budget_reason(prepared);
}

pub(super) fn annotate_compaction_budget(
    prepared: &mut PreparedModelContext,
    budget_plan: &ModelContextBudgetPlan,
    config: &ShortTermMemoryConfig,
    selection_source_message_count: usize,
    raw_tail_message_count: usize,
    compressed_history_message_count: usize,
    tail_start_seq: Option<i64>,
    source_history_token_estimate: Option<u32>,
) {
    let raw_tail_tokens = prepared
        .budget_breakdown
        .as_ref()
        .map(|breakdown| breakdown.recent_transcript_tokens)
        .unwrap_or(0);
    let history_budget_tokens = current_session_history_budget_tokens(prepared);
    let retained_history_tokens = history_budget_tokens
        .saturating_mul(config.compaction.retained_history_ratio_bps.min(10_000))
        / 10_000;
    let trigger_tokens = compaction_trigger_tokens(budget_plan, Some(config));
    prepared.compaction_trigger_tokens = Some(trigger_tokens);
    prepared.compaction_trigger_reached = prepared.estimated_tokens >= trigger_tokens;
    prepared.effective_retained_history_tokens = Some(retained_history_tokens);
    let input_budget_exceeded = prepared.estimated_tokens > budget_plan.effective_input_budget;
    prepared.over_budget = input_budget_exceeded || prompt_prefix_budget_exceeded(prepared);
    if let Some(breakdown) = prepared.budget_breakdown.as_mut() {
        breakdown.raw_tail_transcript_tokens = raw_tail_tokens;
        breakdown.compressed_history_tokens = if compressed_history_message_count > 0 {
            source_history_token_estimate
                .unwrap_or(breakdown.recent_transcript_tokens)
                .saturating_sub(raw_tail_tokens)
        } else {
            0
        };
        breakdown.compaction_trigger_ratio_bps = Some(config.compaction.trigger_ratio_bps);
        breakdown.compaction_trigger_tokens = Some(trigger_tokens);
        breakdown.compaction_trigger_reached = prepared.compaction_trigger_reached;
        breakdown.input_budget_exceeded = input_budget_exceeded;
        breakdown.retained_history_ratio_bps = Some(config.compaction.retained_history_ratio_bps);
        breakdown.effective_retained_history_tokens = Some(retained_history_tokens);
        breakdown.current_session_history_budget_tokens = Some(history_budget_tokens);
        breakdown.history_source_message_count = Some(selection_source_message_count);
        breakdown.raw_tail_message_count = Some(raw_tail_message_count);
        breakdown.compressed_history_message_count = Some(compressed_history_message_count);
        breakdown.raw_tail_start_seq = tail_start_seq;
    }
    refresh_over_budget_reason(prepared);
}

pub(super) fn refresh_over_budget_reason(prepared: &mut PreparedModelContext) {
    if !prepared.over_budget {
        prepared.over_budget_reason = None;
        return;
    }
    let Some(breakdown) = prepared.budget_breakdown.as_ref() else {
        prepared.over_budget_reason = Some("NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET".into());
        return;
    };
    if prompt_prefix_budget_exceeded(prepared) {
        prepared.over_budget_reason = Some("SEED_PROMPT_PREFIX_OVER_BUDGET".into());
        return;
    }
    let compressed_history_message_count = breakdown.compressed_history_message_count.unwrap_or(0);
    prepared.over_budget_reason = Some(
        if breakdown.input_budget_exceeded {
            if compressed_history_message_count > 0 {
                "COMPRESSED_HISTORY_OVER_EFFECTIVE_BUDGET"
            } else {
                "NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET"
            }
        } else {
            "NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET"
        }
        .to_string(),
    );
}

fn compaction_trigger_tokens(
    budget_plan: &ModelContextBudgetPlan,
    config: Option<&ShortTermMemoryConfig>,
) -> u32 {
    let ratio = config
        .map(|config| config.compaction.trigger_ratio_bps)
        .unwrap_or(10_000)
        .min(10_000);
    budget_plan.effective_input_budget.saturating_mul(ratio) / 10_000
}

pub(super) fn prompt_prefix_budget_exceeded(prepared: &PreparedModelContext) -> bool {
    prepared
        .budget_breakdown
        .as_ref()
        .and_then(|breakdown| {
            breakdown
                .prompt_prefix_budget_tokens
                .map(|limit| breakdown.prompt_prefix_tokens > limit)
        })
        .unwrap_or(false)
}

pub(super) fn current_session_history_budget_tokens(prepared: &PreparedModelContext) -> u32 {
    let Some(breakdown) = prepared.budget_breakdown.as_ref() else {
        return prepared.effective_input_budget.unwrap_or(0);
    };
    let effective_input_budget = breakdown.effective_input_budget.unwrap_or(0);
    let fixed_tokens = breakdown
        .prompt_prefix_tokens
        .saturating_add(breakdown.checkpoint_tokens)
        .saturating_add(breakdown.session_family_context_tokens)
        .saturating_add(breakdown.working_state_tokens)
        .saturating_add(breakdown.tool_schema_tokens);
    effective_input_budget.saturating_sub(fixed_tokens)
}
