//! Tron scope checks, business state and display projections over Framework stores.
mod assembler;
mod config;
mod dto;
mod native_service;
mod provider_config;
mod redaction;
pub use config::{
    CompactionPolicyConfig, ContextBlockBudgetConfig, ContextBlockBudgets, ContextBudgetConfig,
    FamilyLoadConfig, GoalLoopConfig, ModelContextBudgetPlan, ShortTermMemoryConfig,
};
pub use dto::{
    ContextBudgetBreakdown, GoalSessionCommand, PreparedContextSources, PreparedModelContext,
    ReplaceTodosCommand, SessionSummaryJobContext, SessionSummaryRunOutcome, SetGoalCommand,
    TodoUpdateItem, UpdateGoalCommand,
};
pub use native_service::ShortTermMemoryService;
pub use provider_config::{
    ContextPriceInputs, ContextProviderPolicy, ContextProvidersConfig, ContextReductionStrategy,
};
pub(crate) use redaction::redact_sensitive_text;

pub(crate) fn memory_provider_usage(
    result: &eliteforge_agent_memory::MemoryCompactionResult,
) -> Option<crate::domain::agent::ModelTokenUsage> {
    result
        .provider_usage
        .as_ref()
        .map(|u| crate::domain::agent::ModelTokenUsage {
            input_tokens: u.input_tokens.min(u32::MAX as u64) as u32,
            output_tokens: u.output_tokens.min(u32::MAX as u64) as u32,
            total_tokens: u
                .input_tokens
                .saturating_add(u.output_tokens)
                .min(u32::MAX as u64) as u32,
            cached_input_tokens: result
                .provider_metrics
                .as_ref()
                .and_then(|m| m.cached_input_tokens)
                .map(|v| v.min(u32::MAX as u64) as u32),
            reasoning_tokens: result
                .provider_metrics
                .as_ref()
                .and_then(|m| m.reasoning_tokens)
                .map(|v| v.min(u32::MAX as u64) as u32),
        })
}

pub(crate) use assembler::{estimate_messages_tokens, truncate_checkpoint_message};
