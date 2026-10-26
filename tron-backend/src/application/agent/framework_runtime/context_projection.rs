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
    // Allocation lends unused space to history. It is not the configured ceiling
    // of the protected instruction block shown by the inspector.
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
        report.prompt_prefix_message_count = count(fw::INSTRUCTIONS_BLOCK);
        report.prompt_prefix_tokens = tokens(fw::INSTRUCTIONS_BLOCK);
        report.checkpoint_tokens = tokens("current_session_checkpoint");
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
            "current_session_checkpoint" => &config.context.providers.checkpoint,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> fw::ContextAssemblyReport {
        fw::ContextAssemblyReport {
            model: fw::ModelRef::new("test", "test"),
            stage: "test".into(),
            step: 1,
            attempt: 1,
            node_execution: None,
            input_capacity: 100_000,
            protocol_tokens: 100,
            estimated_input_tokens: 1100,
            blocks: vec![fw::ContextBlockReport {
                kind: fw::ContextSourceKind::Data,
                sources: vec![],
                id: fw::HISTORY_BLOCK.into(),
                position: 0,
                source_version: None,
                original_tokens: Some(90_000),
                configured_tokens: Some(90_000),
                allocated_tokens: 90_000,
                reduction_budget_tokens: Some(90_000),
                final_tokens: 1000,
                final_message_count: 2,
                outcome: fw::ContextBlockOutcome::Reduced,
                compression_trigger: fw::ContextCompressionTrigger::Global,
            }],
        }
    }

    #[test]
    fn context_health_uses_final_usage_instead_of_past_manual_or_automatic_trigger() {
        let config = ShortTermMemoryConfig::default();
        let mut report = report();
        assert!(!current_compaction_pressure(&report, &config));
        report.blocks[0].compression_trigger = fw::ContextCompressionTrigger::Block;
        assert!(!current_compaction_pressure(&report, &config));
        report.blocks[0].outcome = fw::ContextBlockOutcome::NoGain;
        report.blocks[0].final_tokens = 0;
        assert!(!current_compaction_pressure(&report, &config));
    }

    #[test]
    fn context_health_checks_configured_total_and_individual_block_thresholds() {
        let mut config = ShortTermMemoryConfig::default();
        config.compaction.trigger_ratio_bps = 7500;
        let mut report = report();
        report.estimated_input_tokens = 74_999;
        assert!(!current_compaction_pressure(&report, &config));
        report.estimated_input_tokens = 75_000;
        assert!(current_compaction_pressure(&report, &config));
        report.estimated_input_tokens = 1100;
        let block = &mut report.blocks[0];
        block.id = "page_context".into();
        block.configured_tokens = Some(2000);
        block.allocated_tokens = 1000;
        config
            .context
            .providers
            .data_blocks
            .get_mut("page_context")
            .unwrap()
            .policy
            .trigger_ratio_bps = 9000;
        report.blocks[0].final_tokens = 1799;
        assert!(!current_compaction_pressure(&report, &config));
        report.blocks[0].final_tokens = 1800;
        assert!(current_compaction_pressure(&report, &config));
        config
            .context
            .providers
            .data_blocks
            .get_mut("page_context")
            .unwrap()
            .policy
            .strategy = crate::application::short_term_memory::ContextReductionStrategy::Keep;
        assert!(!current_compaction_pressure(&report, &config));
    }

    #[test]
    fn context_health_history_uses_remaining_quota_and_respects_disabled_auto_compaction() {
        let mut config = ShortTermMemoryConfig::default();
        let mut report = report();
        report.blocks[0].allocated_tokens = 1000;
        assert!(current_compaction_pressure(&report, &config));
        config.compaction.auto_enabled = false;
        report.estimated_input_tokens = 99_000;
        assert!(!current_compaction_pressure(&report, &config));
    }
}
