//! Final SDK reports, rather than raw source DTOs, describe the admitted window.
use crate::application::short_term_memory::{PreparedModelContext, ShortTermMemoryConfig};
use eliteforge_agent_core as fw;
use sha2::{Digest, Sha256};
pub(super) fn update(
    projection: &mut PreparedModelContext,
    window: &fw::ContextWindow,
    input: &fw::ContextAssemblyInput<'_>,
    config: &ShortTermMemoryConfig,
) -> Result<(), fw::AgentError> {
    let mut fixed = Vec::new();
    let mut offset = 0usize;
    for block in &window.report.blocks {
        let end = offset
            .checked_add(block.final_message_count)
            .filter(|end| *end <= window.messages.len())
            .ok_or_else(|| {
                fw::AgentError::InvalidConfiguration(
                    "SDK block report message count mismatch".into(),
                )
            })?;
        if block.id != fw::HISTORY_BLOCK {
            fixed.push((&block.id, &window.messages[offset..end]));
        }
        offset = end;
    }
    if offset != window.messages.len() {
        return Err(fw::AgentError::InvalidConfiguration(
            "SDK block report does not cover the final window".into(),
        ));
    }
    let bytes = serde_json::to_vec(&(
        fixed,
        &config.context.providers.order,
        &config.context.providers.shares,
        input.tools,
        input.options,
        &input.history.generation,
    ))
    .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))?;
    projection.context_hash = Some(format!("{:x}", Sha256::digest(bytes)));
    projection.effective_input_budget =
        Some(window.report.input_capacity.min(u32::MAX as usize) as u32);
    projection.tool_schema_token_estimate =
        window.report.protocol_tokens.min(u32::MAX as usize) as u32;
    projection.rebased |= window
        .report
        .blocks
        .iter()
        .any(|block| matches!(block.outcome, fw::ContextBlockOutcome::Reduced));
    // A trigger records what happened during assembly, including manual /compact.
    // Inspector health describes the final admitted window, after that work.
    projection.compaction_trigger_reached = current_compaction_pressure(&window.report, config);
    projection.compaction_trigger_tokens = config.compaction.auto_enabled.then(|| {
        threshold_tokens(
            window.report.input_capacity,
            config.compaction.trigger_ratio_bps,
        )
        .min(u32::MAX as usize) as u32
    });
    let tokens = |id: &str| {
        window
            .report
            .blocks
            .iter()
            .find(|block| block.id == id)
            .map_or(0, |block| block.final_tokens.min(u32::MAX as usize) as u32)
    };
    let cap = |id: &str| {
        window
            .report
            .blocks
            .iter()
            .find(|block| block.id == id)
            .map(|block| block.allocated_tokens.min(u32::MAX as usize) as u32)
    };
    let history_cap = cap(fw::HISTORY_BLOCK).unwrap_or(0);
    // The SDK report contains the exact percentage-derived quotas, including
    // token rounding and the current request's protocol overhead.
    let configured_cap = |id: &str| window.report.blocks.iter().find(|block| block.id == id)
        .and_then(|block| block.configured_tokens).unwrap_or(0).min(u32::MAX as usize) as u32;
    let seed_cap = configured_cap(fw::INSTRUCTIONS_BLOCK);
    let checkpoint_cap = configured_cap("conversation_history_summary");
    let working_cap = configured_cap("current_session_working_state");
    let tool_cap = Some(window.report.input_capacity.min(u32::MAX as usize) as u32);
    let history_target = eliteforge_agent_memory::ratio_tokens(
        history_cap,
        config.context.providers.history.target_ratio_bps,
    );
    projection.effective_retained_history_tokens = Some(history_target);
    let count = |id: &str| {
        window
            .report
            .blocks
            .iter()
            .find(|block| block.id == id)
            .map_or(0, |block| block.final_message_count)
    };
    {
        // Native storage no longer prepares a product budget report. Project the
        // final Framework assembly report for the inspector on every model call.
        let report = projection
            .budget_breakdown
            .get_or_insert_with(Default::default);
        report.context_window_limit = projection.context_window_limit;
        report.output_token_reserve = projection.output_token_reserve;
        report.application_context_tokens = tokens("application_context");
        report.application_context_message_count = count("application_context");
        report.prompt_prefix_message_count = count(fw::INSTRUCTIONS_BLOCK);
        report.prompt_prefix_tokens = tokens(fw::INSTRUCTIONS_BLOCK);
        report.checkpoint_tokens = tokens("conversation_history_summary");
        report.working_state_tokens = tokens("current_session_working_state");
        report.recent_transcript_tokens = tokens(fw::HISTORY_BLOCK);
        report.raw_tail_transcript_tokens = report.recent_transcript_tokens;
        report.retained_history_ratio_bps = Some(config.context.providers.history.target_ratio_bps);
        report.effective_retained_history_tokens = Some(history_target);
        report.effective_input_budget = projection.effective_input_budget;
        report.tool_schema_tokens = projection.tool_schema_token_estimate;
        report.tool_schema_budget_tokens = tool_cap;
        report.prompt_prefix_budget_tokens = Some(seed_cap);
        report.prompt_prefix_max_ratio_bps = config.context.providers.shares.get("seed_prompt_prefix").copied();
        report.prompt_prefix_max_tokens = None;
        let history_report = window
            .report
            .blocks
            .iter()
            .find(|block| block.id == fw::HISTORY_BLOCK);
        report.raw_tail_message_count = history_report.map(|block| block.final_message_count);
        report.history_source_message_count = Some(input.history.messages.len());
        report.compressed_history_message_count = Some(
            report
                .history_source_message_count
                .unwrap_or(0)
                .saturating_sub(report.raw_tail_message_count.unwrap_or(0)),
        );
        report.compressed_history_tokens = history_report.map_or(0, |block| {
            block
                .original_tokens
                .unwrap_or(0)
                .saturating_sub(block.final_tokens)
                .min(u32::MAX as usize) as u32
        });
        report.estimated_input_tokens = projection.estimated_tokens;
        report.input_budget_exceeded = false;
        report.compaction_trigger_reached = projection.compaction_trigger_reached;
        report.compaction_trigger_ratio_bps = config
            .compaction
            .auto_enabled
            .then_some(config.compaction.trigger_ratio_bps);
        report.compaction_trigger_tokens = projection.compaction_trigger_tokens;
        report.checkpoint_budget_tokens = Some(checkpoint_cap);
        report.working_state_budget_tokens = Some(working_cap);
        report.current_session_history_budget_tokens = cap(fw::HISTORY_BLOCK);
    }
    Ok(())
}

fn threshold_tokens(capacity: usize, ratio_bps: u32) -> usize {
    ((capacity as u128 * ratio_bps.min(10_000) as u128) / 10_000) as usize
}

fn current_compaction_pressure(
    report: &fw::ContextAssemblyReport,
    config: &ShortTermMemoryConfig,
) -> bool {
    use crate::application::short_term_memory::ContextReductionStrategy as Strategy;
    if config.compaction.auto_enabled
        && report.estimated_input_tokens > 0
        && report.estimated_input_tokens
            >= threshold_tokens(report.input_capacity, config.compaction.trigger_ratio_bps)
    {
        return true;
    }
    report.blocks.iter().any(|block| {
        let policy = match block.id.as_str() {
            fw::HISTORY_BLOCK if config.compaction.auto_enabled => {
                &config.context.providers.history
            }
            "conversation_history_summary" => &config.context.providers.checkpoint,
            "current_session_working_state" => &config.context.providers.working,
            id => match config.context.providers.data_blocks.get(id) {
                Some(data) => &data.policy,
                None => return false,
            },
        };
        // History borrows remaining capacity; other blocks use their configured cap,
        // matching the Framework's threshold calculation before shared allocation.
        let capacity = if block.id == fw::HISTORY_BLOCK {
            block.allocated_tokens
        } else {
            block.configured_tokens.unwrap_or(block.allocated_tokens)
        };
        policy.strategy != Strategy::Keep
            && block.final_tokens > 0
            && block.final_tokens >= threshold_tokens(capacity, policy.trigger_ratio_bps)
    })
}
