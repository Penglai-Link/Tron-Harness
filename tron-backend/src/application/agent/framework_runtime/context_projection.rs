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
    projection.compaction_trigger_reached = window
        .report
        .blocks
        .iter()
        .any(|block| block.compression_trigger != fw::ContextCompressionTrigger::None);
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
    // Allocation lends unused space to history. It is not the configured ceiling
    // of the protected instructions/application pool shown by the inspector.
    let seed_config = &config.context.blocks.seed_prompt_prefix;
    let available = fw::ContextBudget {
        max_tokens: window
            .report
            .input_capacity
            .saturating_sub(window.report.protocol_tokens),
        ..input.budget
    };
    let configured_cap =
        |id: &str,
         config: &crate::application::short_term_memory::ContextBlockBudgetConfig|
         -> Result<u32, fw::AgentError> {
            Ok(fw::configured_block_budget(
                &fw::ContextBlockConfig::new(id, super::context_providers::block_budget(config)?),
                available,
            )?
            .max_tokens
            .min(u32::MAX as usize) as u32)
        };
    let seed_cap = configured_cap(fw::INSTRUCTIONS_BLOCK, seed_config)?;
    let checkpoint_cap = configured_cap(
        "current_session_checkpoint",
        &config.context.blocks.current_session_checkpoint,
    )?;
    let family_cap = configured_cap(
        "session_family_context",
        &config.context.blocks.session_family_context,
    )?;
    let working_cap = configured_cap(
        "current_session_working_state",
        &config.context.blocks.current_session_working_state,
    )?;
    let tool_cap = projection
        .context_window_limit
        .zip(projection.output_token_reserve)
        .map(|(window, reserve)| {
            config
                .context
                .blocks
                .plan(window, reserve)
                .block_budgets
                .tool_schema
                .effective_limit_tokens
        });
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
        report.prompt_prefix_message_count = count(fw::INSTRUCTIONS_BLOCK)
            .saturating_add(count("seed_prompt_prefix"))
            .saturating_add(report.application_context_message_count);
        report.prompt_prefix_tokens = tokens(fw::INSTRUCTIONS_BLOCK)
            .saturating_add(tokens("seed_prompt_prefix"))
            .saturating_add(report.application_context_tokens);
        report.checkpoint_tokens = tokens("current_session_checkpoint");
        report.session_family_context_tokens = tokens("session_family_context");
        report.working_state_tokens = tokens("current_session_working_state");
        report.recent_transcript_tokens = tokens(fw::HISTORY_BLOCK);
        report.raw_tail_transcript_tokens = report.recent_transcript_tokens;
        report.retained_history_ratio_bps = Some(config.context.providers.history.target_ratio_bps);
        report.effective_retained_history_tokens = Some(history_target);
        report.effective_input_budget = projection.effective_input_budget;
        report.tool_schema_tokens = projection.tool_schema_token_estimate;
        report.tool_schema_budget_tokens = tool_cap;
        report.prompt_prefix_budget_tokens = Some(seed_cap);
        report.prompt_prefix_max_ratio_bps = seed_config.max_ratio_bps;
        report.prompt_prefix_max_tokens = seed_config.max_tokens;
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
        report.checkpoint_budget_tokens = Some(checkpoint_cap);
        report.session_family_context_budget_tokens = Some(family_cap);
        report.working_state_budget_tokens = Some(working_cap);
        report.current_session_history_budget_tokens = cap(fw::HISTORY_BLOCK);
    }
    Ok(())
}
