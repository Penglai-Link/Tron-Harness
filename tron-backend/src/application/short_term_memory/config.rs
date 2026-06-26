use serde::Serialize;

const DEFAULT_TOOL_OUTPUT_ROOT: &str = "~/.tron/artifacts/short-term-memory/tool-results";
const DEFAULT_TOOL_OUTPUT_EXTERNALIZE_BYTES: usize = 32_768;
const DEFAULT_CONTEXT_WINDOW_TOKENS: u32 = 32_000;
const DEFAULT_FAMILY_CONTEXT_MAX_RATIO_BPS: u32 = 1_000;
const DEFAULT_FAMILY_CONTEXT_HARD_CAP_TOKENS: u32 = 12_000;
const DEFAULT_FAMILY_CONTEXT_MAX_CHARS: usize = 6_000;
const DEFAULT_SEED_PROMPT_PREFIX_MAX_RATIO_BPS: u32 = 5_000;
const DEFAULT_SEED_PROMPT_PREFIX_HARD_CAP_TOKENS: u32 = 16_000;
const DEFAULT_CHECKPOINT_CONTEXT_MAX_RATIO_BPS: u32 = 2_000;
const DEFAULT_CHECKPOINT_CONTEXT_HARD_CAP_TOKENS: u32 = 6_000;
const DEFAULT_WORKING_STATE_CONTEXT_MAX_RATIO_BPS: u32 = 1_000;
const DEFAULT_WORKING_STATE_CONTEXT_HARD_CAP_TOKENS: u32 = 2_000;
const DEFAULT_MAX_FAMILY_SNAPSHOTS: usize = 3;
const DEFAULT_MAX_FAMILY_SESSIONS: usize = 3;
const DEFAULT_SNAPSHOT_MAX_CHARS: usize = 2_000;
const DEFAULT_FAMILY_TTL_HOURS: u32 = 72;
const DEFAULT_TASK_WEIGHT_BPS: u32 = 6_500;
const DEFAULT_REQUIREMENT_WEIGHT_BPS: u32 = 2_500;
const DEFAULT_WORKSPACE_CHAT_WEIGHT_BPS: u32 = 1_000;
const DEFAULT_WORKSPACE_HEARTBEAT_WEIGHT_BPS: u32 = 1_000;
const DEFAULT_OTHER_WEIGHT_BPS: u32 = 500;
pub(super) const DEFAULT_COMPACTION_TRIGGER_RATIO_BPS: u32 = 8_000;
pub(super) const DEFAULT_COMPACTION_RETAINED_HISTORY_RATIO_BPS: u32 = 2_000;
const DEFAULT_GOAL_MAX_CONSECUTIVE_JUDGE_FAILURES: u32 = 3;
const DEFAULT_GOAL_JUDGE_INPUT_MAX_CHARS: usize = 12_000;
const DEFAULT_GOAL_JUDGE_OUTPUT_MAX_TOKENS: u32 = 1_024;
const DEFAULT_EVENT_LOG_RETENTION_DAYS: u32 = 5;
const DEFAULT_EVENT_LOG_MAX_PAYLOAD_BYTES: usize = 8_192;
const DEFAULT_EVENT_LOG_CLEANUP_BATCH_SIZE: usize = 5_000;
const DEFAULT_EVENT_LOG_CLEANUP_MAX_ROWS: usize = 50_000;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ShortTermMemoryConfig {
    pub tool_output: ToolOutputConfig,
    pub event_log: EventLogConfig,
    pub context: ContextBudgetConfig,
    pub family_load: FamilyLoadConfig,
    pub compaction: CompactionPolicyConfig,
    pub goal_loop: GoalLoopConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct EventLogConfig {
    pub mode: String,
    pub retention_days: u32,
    pub max_payload_bytes: usize,
    pub cleanup_on_startup: bool,
    pub cleanup_batch_size: usize,
    pub cleanup_max_rows_per_startup: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ToolOutputConfig {
    pub root: String,
    pub externalize_above_bytes: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBudgetConfig {
    pub default_context_window_tokens: u32,
    pub family_context_max_ratio_bps: u32,
    pub family_context_hard_cap_tokens: u32,
    pub family_context_max_chars: usize,
    pub reasoning_summary_replay_enabled: bool,
    pub blocks: ContextBlockBudgets,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBlockBudgets {
    pub seed_prompt_prefix: ContextBlockBudgetConfig,
    pub current_session_checkpoint: ContextBlockBudgetConfig,
    pub session_family_context: ContextBlockBudgetConfig,
    pub current_session_working_state: ContextBlockBudgetConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBlockBudgetConfig {
    pub max_ratio_bps: Option<u32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FamilyLoadConfig {
    pub default_max_snapshots: usize,
    pub default_max_sessions: usize,
    pub default_snapshot_max_chars: usize,
    pub default_ttl_hours: u32,
    pub task_weight_bps: u32,
    pub requirement_weight_bps: u32,
    pub workspace_chat_weight_bps: u32,
    pub workspace_heartbeat_weight_bps: u32,
    pub other_weight_bps: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CompactionPolicyConfig {
    pub auto_enabled: bool,
    pub trigger_ratio_bps: u32,
    pub retained_history_ratio_bps: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct GoalLoopConfig {
    pub max_runs: u32,
    pub max_consecutive_judge_failures: u32,
    pub judge_input_max_chars: usize,
    pub judge_output_max_tokens: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ModelContextBudgetPlan {
    pub model_context_window: u32,
    pub output_token_reserve: u32,
    pub effective_input_budget: u32,
    pub block_budgets: ContextBlockBudgetPlans,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBlockBudgetPlans {
    pub seed_prompt_prefix: ContextBlockBudgetPlan,
    pub current_session_checkpoint: ContextBlockBudgetPlan,
    pub session_family_context: ContextBlockBudgetPlan,
    pub current_session_working_state: ContextBlockBudgetPlan,
    pub current_session_history: ContextBlockBudgetPlan,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextBlockBudgetPlan {
    pub block_kind: &'static str,
    pub enabled: bool,
    pub effective_limit_tokens: u32,
    pub priority: u8,
    pub allow_borrow_unused_budget: bool,
}

impl Default for ShortTermMemoryConfig {
    fn default() -> Self {
        Self {
            tool_output: ToolOutputConfig {
                root: DEFAULT_TOOL_OUTPUT_ROOT.into(),
                externalize_above_bytes: DEFAULT_TOOL_OUTPUT_EXTERNALIZE_BYTES,
            },
            event_log: EventLogConfig {
                mode: "full".into(),
                retention_days: DEFAULT_EVENT_LOG_RETENTION_DAYS,
                max_payload_bytes: DEFAULT_EVENT_LOG_MAX_PAYLOAD_BYTES,
                cleanup_on_startup: true,
                cleanup_batch_size: DEFAULT_EVENT_LOG_CLEANUP_BATCH_SIZE,
                cleanup_max_rows_per_startup: DEFAULT_EVENT_LOG_CLEANUP_MAX_ROWS,
            },
            context: ContextBudgetConfig {
                default_context_window_tokens: DEFAULT_CONTEXT_WINDOW_TOKENS,
                family_context_max_ratio_bps: DEFAULT_FAMILY_CONTEXT_MAX_RATIO_BPS,
                family_context_hard_cap_tokens: DEFAULT_FAMILY_CONTEXT_HARD_CAP_TOKENS,
                family_context_max_chars: DEFAULT_FAMILY_CONTEXT_MAX_CHARS,
                reasoning_summary_replay_enabled: false,
                blocks: ContextBlockBudgets::default(),
            },
            family_load: FamilyLoadConfig {
                default_max_snapshots: DEFAULT_MAX_FAMILY_SNAPSHOTS,
                default_max_sessions: DEFAULT_MAX_FAMILY_SESSIONS,
                default_snapshot_max_chars: DEFAULT_SNAPSHOT_MAX_CHARS,
                default_ttl_hours: DEFAULT_FAMILY_TTL_HOURS,
                task_weight_bps: DEFAULT_TASK_WEIGHT_BPS,
                requirement_weight_bps: DEFAULT_REQUIREMENT_WEIGHT_BPS,
                workspace_chat_weight_bps: DEFAULT_WORKSPACE_CHAT_WEIGHT_BPS,
                workspace_heartbeat_weight_bps: DEFAULT_WORKSPACE_HEARTBEAT_WEIGHT_BPS,
                other_weight_bps: DEFAULT_OTHER_WEIGHT_BPS,
            },
            compaction: CompactionPolicyConfig {
                auto_enabled: true,
                trigger_ratio_bps: DEFAULT_COMPACTION_TRIGGER_RATIO_BPS,
                retained_history_ratio_bps: DEFAULT_COMPACTION_RETAINED_HISTORY_RATIO_BPS,
            },
            goal_loop: GoalLoopConfig {
                max_runs: 20,
                max_consecutive_judge_failures: DEFAULT_GOAL_MAX_CONSECUTIVE_JUDGE_FAILURES,
                judge_input_max_chars: DEFAULT_GOAL_JUDGE_INPUT_MAX_CHARS,
                judge_output_max_tokens: DEFAULT_GOAL_JUDGE_OUTPUT_MAX_TOKENS,
            },
        }
    }
}

impl Default for ContextBlockBudgets {
    fn default() -> Self {
        Self {
            seed_prompt_prefix: ContextBlockBudgetConfig {
                max_ratio_bps: Some(DEFAULT_SEED_PROMPT_PREFIX_MAX_RATIO_BPS),
                max_tokens: Some(DEFAULT_SEED_PROMPT_PREFIX_HARD_CAP_TOKENS),
            },
            current_session_checkpoint: ContextBlockBudgetConfig {
                max_ratio_bps: Some(DEFAULT_CHECKPOINT_CONTEXT_MAX_RATIO_BPS),
                max_tokens: Some(DEFAULT_CHECKPOINT_CONTEXT_HARD_CAP_TOKENS),
            },
            session_family_context: ContextBlockBudgetConfig {
                max_ratio_bps: Some(DEFAULT_FAMILY_CONTEXT_MAX_RATIO_BPS),
                max_tokens: Some(DEFAULT_FAMILY_CONTEXT_HARD_CAP_TOKENS),
            },
            current_session_working_state: ContextBlockBudgetConfig {
                max_ratio_bps: Some(DEFAULT_WORKING_STATE_CONTEXT_MAX_RATIO_BPS),
                max_tokens: Some(DEFAULT_WORKING_STATE_CONTEXT_HARD_CAP_TOKENS),
            },
        }
    }
}

impl ShortTermMemoryConfig {
    pub fn model_context_budget_plan(
        &self,
        context_window_tokens: Option<u32>,
        max_output_tokens: Option<u32>,
    ) -> ModelContextBudgetPlan {
        let model_context_window =
            context_window_tokens.unwrap_or(self.context.default_context_window_tokens);
        let output_token_reserve = max_output_tokens.unwrap_or(0);
        let effective_input_budget = model_context_window.saturating_sub(output_token_reserve);
        ModelContextBudgetPlan {
            model_context_window,
            output_token_reserve,
            effective_input_budget,
            block_budgets: ContextBlockBudgetPlans {
                seed_prompt_prefix: block_plan(
                    "seed_prompt_prefix",
                    &self.context.blocks.seed_prompt_prefix,
                    effective_input_budget,
                ),
                current_session_checkpoint: block_plan(
                    "current_session_checkpoint",
                    &self.context.blocks.current_session_checkpoint,
                    effective_input_budget,
                ),
                session_family_context: block_plan(
                    "session_family_context",
                    &self.context.blocks.session_family_context,
                    effective_input_budget,
                ),
                current_session_working_state: block_plan(
                    "current_session_working_state",
                    &self.context.blocks.current_session_working_state,
                    effective_input_budget,
                ),
                current_session_history: ContextBlockBudgetPlan {
                    block_kind: "current_session_history",
                    enabled: true,
                    effective_limit_tokens: effective_input_budget,
                    priority: 60,
                    allow_borrow_unused_budget: true,
                },
            },
        }
    }
}

fn block_plan(
    block_kind: &'static str,
    config: &ContextBlockBudgetConfig,
    effective_input_budget: u32,
) -> ContextBlockBudgetPlan {
    let ratio_cap = config
        .max_ratio_bps
        .map(|ratio| effective_input_budget.saturating_mul(ratio.min(10_000)) / 10_000);
    let effective_limit_tokens = match (ratio_cap, config.max_tokens) {
        (Some(ratio), Some(max_tokens)) => ratio.min(max_tokens),
        (Some(ratio), None) => ratio,
        (None, Some(max_tokens)) => max_tokens,
        (None, None) => effective_input_budget,
    }
    .min(effective_input_budget);
    ContextBlockBudgetPlan {
        block_kind,
        enabled: true,
        effective_limit_tokens,
        priority: 0,
        allow_borrow_unused_budget: false,
    }
}
