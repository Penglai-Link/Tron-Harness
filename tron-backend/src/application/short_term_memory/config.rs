pub use eliteforge_agent_memory::{
    ContextBlockBudgetConfig, ContextBlockBudgets, ModelContextBudgetPlan,
};
use serde::Serialize;

const DEFAULT_CONTEXT_WINDOW_TOKENS: u32 = 32_000;
pub(super) const DEFAULT_COMPACTION_TRIGGER_RATIO_BPS: u32 = 8_000;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ShortTermMemoryConfig {
    pub context: ContextBudgetConfig,
    pub compaction: CompactionPolicyConfig,
    pub goal_loop: GoalLoopConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBudgetConfig {
    pub default_context_window_tokens: u32,
    pub blocks: ContextBlockBudgets,
    pub providers: super::ContextProvidersConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CompactionPolicyConfig {
    pub auto_enabled: bool,
    pub trigger_ratio_bps: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct GoalLoopConfig {
    pub max_runs: u32,
}

impl Default for ShortTermMemoryConfig {
    fn default() -> Self {
        Self {
            context: ContextBudgetConfig {
                default_context_window_tokens: DEFAULT_CONTEXT_WINDOW_TOKENS,
                blocks: ContextBlockBudgets::default(),
                providers: Default::default(),
            },
            compaction: CompactionPolicyConfig {
                auto_enabled: true,
                trigger_ratio_bps: DEFAULT_COMPACTION_TRIGGER_RATIO_BPS,
            },
            goal_loop: GoalLoopConfig { max_runs: 20 },
        }
    }
}

impl ShortTermMemoryConfig {
    pub fn model_context_budget_plan(
        &self,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
    ) -> ModelContextBudgetPlan {
        let shares = &self.context.providers.shares;
        let ratio = |id: &str| ContextBlockBudgetConfig {
            max_tokens: None,
            max_ratio_bps: shares.get(id).copied(),
        };
        ContextBlockBudgets {
            seed_prompt_prefix: ratio("seed_prompt_prefix"),
            conversation_history_summary: ratio("conversation_history_summary"),
            current_session_working_state: ratio("current_session_working_state"),
            tool_schema: ContextBlockBudgetConfig {
                max_tokens: None,
                max_ratio_bps: Some(10_000),
            },
        }
        .plan(
            context_window_tokens.unwrap_or(self.context.default_context_window_tokens),
            max_output_tokens.unwrap_or(0),
        )
    }
}
