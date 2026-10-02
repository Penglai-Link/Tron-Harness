use super::{ModelContextBudgetPlan, PreparedModelContext, ShortTermMemoryConfig};

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
    let tools_over_budget =
        tool_schema_token_estimate > budget_plan.block_budgets.tool_schema.effective_limit_tokens;
    prepared.over_budget =
        input_budget_exceeded || prompt_prefix_budget_exceeded(prepared) || tools_over_budget;
    if let Some(breakdown) = prepared.budget_breakdown.as_mut() {
        breakdown.tool_schema_tokens = tool_schema_token_estimate;
        breakdown.tool_schema_budget_tokens =
            Some(budget_plan.block_budgets.tool_schema.effective_limit_tokens);
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
        breakdown.checkpoint_budget_tokens = Some(
            budget_plan
                .block_budgets
                .current_session_checkpoint
                .effective_limit_tokens,
        );
        breakdown.session_family_context_budget_tokens = Some(
            budget_plan
                .block_budgets
                .session_family_context
                .effective_limit_tokens,
        );
        breakdown.working_state_budget_tokens = Some(
            budget_plan
                .block_budgets
                .current_session_working_state
                .effective_limit_tokens,
        );
        breakdown.compaction_trigger_ratio_bps = Some(config.compaction.trigger_ratio_bps);
        breakdown.compaction_trigger_tokens = Some(trigger_tokens);
        breakdown.compaction_trigger_reached = prepared.compaction_trigger_reached;
        breakdown.input_budget_exceeded = input_budget_exceeded;
    }
    refresh_over_budget_reason(prepared);
    if tools_over_budget {
        prepared.over_budget_reason = Some("TOOL_SCHEMA_OVER_BUDGET".into());
        prepared.compaction_required = false;
    }
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
    let retained_history_tokens = eliteforge_agent_memory::ratio_tokens(
        history_budget_tokens,
        config.compaction.retained_history_ratio_bps,
    );
    let trigger_tokens = compaction_trigger_tokens(budget_plan, Some(config));
    prepared.compaction_trigger_tokens = Some(trigger_tokens);
    prepared.compaction_trigger_reached = prepared.estimated_tokens >= trigger_tokens;
    prepared.effective_retained_history_tokens = Some(retained_history_tokens);
    let input_budget_exceeded = prepared.estimated_tokens > budget_plan.effective_input_budget;
    prepared.over_budget = input_budget_exceeded
        || prompt_prefix_budget_exceeded(prepared)
        || tool_schema_budget_exceeded(prepared);
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
    prepared.over_budget_reason = eliteforge_agent_memory::budget_reason(
        prepared.estimated_tokens > prepared.effective_input_budget.unwrap_or(u32::MAX),
        prompt_prefix_budget_exceeded(prepared),
        tool_schema_budget_exceeded(prepared),
        prepared
            .budget_breakdown
            .as_ref()
            .is_some_and(|b| b.compressed_history_message_count.unwrap_or(0) > 0),
    )
    .map(str::to_owned);
}

fn compaction_trigger_tokens(
    budget_plan: &ModelContextBudgetPlan,
    config: Option<&ShortTermMemoryConfig>,
) -> u32 {
    let ratio = config
        .map(|config| config.compaction.trigger_ratio_bps)
        .unwrap_or(10_000)
        .min(10_000);
    eliteforge_agent_memory::ratio_tokens(budget_plan.effective_input_budget, ratio)
}

fn tool_schema_budget_exceeded(prepared: &PreparedModelContext) -> bool {
    prepared
        .budget_breakdown
        .as_ref()
        .and_then(|budget| budget.tool_schema_budget_tokens)
        .is_some_and(|limit| prepared.tool_schema_token_estimate > limit)
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
    eliteforge_agent_memory::remaining_history_budget(
        effective_input_budget,
        &[
            breakdown.prompt_prefix_tokens,
            breakdown.checkpoint_tokens,
            breakdown.session_family_context_tokens,
            breakdown.working_state_tokens,
            breakdown.tool_schema_tokens,
        ],
    )
}
