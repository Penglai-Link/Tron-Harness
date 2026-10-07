pub use eliteforge_agent_memory::{
    ContextBlockBudgetConfig, ContextBlockBudgets, ModelContextBudgetPlan,
};
use serde::Serialize;

const DEFAULT_CONTEXT_WINDOW_TOKENS: u32 = 32_000;
const DEFAULT_FAMILY_CONTEXT_MAX_RATIO_BPS: u32 = 1_000;
const DEFAULT_FAMILY_CONTEXT_HARD_CAP_TOKENS: u32 = 12_000;
const DEFAULT_FAMILY_CONTEXT_MAX_CHARS: usize = 6_000;
const DEFAULT_MAX_FAMILY_SESSIONS: usize = 3;
pub(super) const DEFAULT_COMPACTION_TRIGGER_RATIO_BPS: u32 = 8_000;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ShortTermMemoryConfig {
    pub context: ContextBudgetConfig,
    pub family_load: FamilyLoadConfig,
    pub compaction: CompactionPolicyConfig,
    pub goal_loop: GoalLoopConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBudgetConfig {
    pub default_context_window_tokens: u32,
    pub family_context_max_ratio_bps: u32,
    pub family_context_hard_cap_tokens: u32,
    pub family_context_max_chars: usize,
    pub blocks: ContextBlockBudgets,
    pub providers: super::ContextProvidersConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FamilyLoadConfig {
    pub default_max_sessions: usize,
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
                family_context_max_ratio_bps: DEFAULT_FAMILY_CONTEXT_MAX_RATIO_BPS,
                family_context_hard_cap_tokens: DEFAULT_FAMILY_CONTEXT_HARD_CAP_TOKENS,
                family_context_max_chars: DEFAULT_FAMILY_CONTEXT_MAX_CHARS,
                blocks: ContextBlockBudgets::default(),
                providers: Default::default(),
            },
            family_load: FamilyLoadConfig {
                default_max_sessions: DEFAULT_MAX_FAMILY_SESSIONS,
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
        self.context.blocks.plan(
            context_window_tokens.unwrap_or(self.context.default_context_window_tokens),
            max_output_tokens.unwrap_or(0),
        )
    }
}
