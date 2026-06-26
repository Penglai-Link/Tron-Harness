use serde::Deserialize;

use crate::application::short_term_memory::{
    CompactionPolicyConfig, ContextBlockBudgetConfig, ContextBlockBudgets, ContextBudgetConfig,
    EventLogConfig, FamilyLoadConfig, GoalLoopConfig, ShortTermMemoryConfig, ToolOutputConfig,
};

impl ShortTermMemoryConfig {
    pub(super) fn apply_overrides(&mut self, overrides: ShortTermMemoryFileConfig) {
        if let Some(value) = overrides.tool_output {
            self.tool_output.apply_overrides(value);
        }
        if let Some(value) = overrides.event_log {
            self.event_log.apply_overrides(value);
        }
        if let Some(value) = overrides.context {
            self.context.apply_overrides(value);
        }
        if let Some(value) = overrides.family_load {
            self.family_load.apply_overrides(value);
        }
        if let Some(value) = overrides.compaction {
            self.compaction.apply_overrides(value);
        }
        if let Some(value) = overrides.goal_loop {
            self.goal_loop.apply_overrides(value);
        }
    }
}

impl EventLogConfig {
    fn apply_overrides(&mut self, overrides: EventLogFileConfig) {
        if let Some(value) = overrides.mode {
            self.mode = match value.as_str() {
                "full" | "lifecycleOnly" => value,
                _ => self.mode.clone(),
            };
        }
        if let Some(value) = overrides.retention_days {
            self.retention_days = value.max(1);
        }
        if let Some(value) = overrides.max_payload_bytes {
            self.max_payload_bytes = value.max(256);
        }
        if let Some(value) = overrides.cleanup_on_startup {
            self.cleanup_on_startup = value;
        }
        if let Some(value) = overrides.cleanup_batch_size {
            self.cleanup_batch_size = value.max(1);
        }
        if let Some(value) = overrides.cleanup_max_rows_per_startup {
            self.cleanup_max_rows_per_startup = value.max(1);
        }
    }
}

impl ToolOutputConfig {
    fn apply_overrides(&mut self, overrides: ToolOutputFileConfig) {
        if let Some(value) = overrides.root {
            self.root = value;
        }
        if let Some(value) = overrides.externalize_above_bytes {
            self.externalize_above_bytes = value;
        }
    }
}

impl ContextBudgetConfig {
    fn apply_overrides(&mut self, overrides: ContextBudgetFileConfig) {
        if let Some(value) = overrides.default_context_window_tokens {
            self.default_context_window_tokens = value.max(1);
        }
        if let Some(value) = overrides.family_context_max_ratio_bps {
            let ratio = value.min(10_000);
            self.family_context_max_ratio_bps = ratio;
            self.blocks.session_family_context.max_ratio_bps = Some(ratio);
        }
        if let Some(value) = overrides.family_context_hard_cap_tokens {
            let max_tokens = value.max(1);
            self.family_context_hard_cap_tokens = max_tokens;
            self.blocks.session_family_context.max_tokens = Some(max_tokens);
        }
        if let Some(value) = overrides.family_context_max_chars {
            self.family_context_max_chars = value.max(1);
        }
        if let Some(value) = overrides.reasoning_summary_replay_enabled {
            self.reasoning_summary_replay_enabled = value;
        }
        if let Some(value) = overrides.blocks {
            self.blocks.apply_overrides(value);
            if let Some(ratio) = self.blocks.session_family_context.max_ratio_bps {
                self.family_context_max_ratio_bps = ratio;
            }
            if let Some(max_tokens) = self.blocks.session_family_context.max_tokens {
                self.family_context_hard_cap_tokens = max_tokens;
            }
        }
    }
}

impl FamilyLoadConfig {
    fn apply_overrides(&mut self, overrides: FamilyLoadFileConfig) {
        if let Some(value) = overrides.default_max_snapshots {
            self.default_max_snapshots = value.max(1);
        }
        if let Some(value) = overrides.default_max_sessions {
            self.default_max_sessions = value.max(1);
        }
        if let Some(value) = overrides.default_snapshot_max_chars {
            self.default_snapshot_max_chars = value.max(1);
        }
        if let Some(value) = overrides.default_ttl_hours {
            self.default_ttl_hours = value.max(1);
        }
        if let Some(value) = overrides.task_weight_bps {
            self.task_weight_bps = value;
        }
        if let Some(value) = overrides.requirement_weight_bps {
            self.requirement_weight_bps = value;
        }
        if let Some(value) = overrides.workspace_chat_weight_bps {
            self.workspace_chat_weight_bps = value;
        }
        if let Some(value) = overrides.workspace_heartbeat_weight_bps {
            self.workspace_heartbeat_weight_bps = value;
        }
        if let Some(value) = overrides.other_weight_bps {
            self.other_weight_bps = value;
        }
    }
}

impl ContextBlockBudgets {
    fn apply_overrides(&mut self, overrides: ContextBlockBudgetsFileConfig) {
        if let Some(value) = overrides.seed_prompt_prefix {
            self.seed_prompt_prefix.apply_overrides(value);
        }
        if let Some(value) = overrides.current_session_checkpoint {
            self.current_session_checkpoint.apply_overrides(value);
        }
        if let Some(value) = overrides.session_family_context {
            self.session_family_context.apply_overrides(value);
        }
        if let Some(value) = overrides.current_session_working_state {
            self.current_session_working_state.apply_overrides(value);
        }
    }
}

impl ContextBlockBudgetConfig {
    fn apply_overrides(&mut self, overrides: ContextBlockBudgetFileConfig) {
        if let Some(value) = overrides.max_ratio_bps {
            self.max_ratio_bps = Some(value.min(10_000));
        }
        if let Some(value) = overrides.max_tokens {
            self.max_tokens = Some(value.max(1));
        }
    }
}

impl CompactionPolicyConfig {
    fn apply_overrides(&mut self, overrides: CompactionPolicyFileConfig) {
        if let Some(value) = overrides.trigger_ratio_bps {
            self.trigger_ratio_bps = value.min(10_000);
        }
        if let Some(value) = overrides.retained_history_ratio_bps {
            self.retained_history_ratio_bps = value.min(10_000);
        }
    }
}

impl GoalLoopConfig {
    fn apply_overrides(&mut self, overrides: GoalLoopFileConfig) {
        if let Some(value) = overrides.max_runs {
            self.max_runs = value.max(1);
        }
        if let Some(value) = overrides.max_consecutive_judge_failures {
            self.max_consecutive_judge_failures = value.max(1);
        }
        if let Some(value) = overrides.judge_max_attempts_per_run {
            self.judge_max_attempts_per_run = value.max(1);
        }
        if let Some(value) = overrides.judge_input_max_chars {
            self.judge_input_max_chars = value.max(1);
        }
        if let Some(value) = overrides.judge_output_max_tokens {
            self.judge_output_max_tokens = value.max(1);
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ShortTermMemoryFileConfig {
    tool_output: Option<ToolOutputFileConfig>,
    event_log: Option<EventLogFileConfig>,
    context: Option<ContextBudgetFileConfig>,
    family_load: Option<FamilyLoadFileConfig>,
    compaction: Option<CompactionPolicyFileConfig>,
    goal_loop: Option<GoalLoopFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EventLogFileConfig {
    mode: Option<String>,
    retention_days: Option<u32>,
    max_payload_bytes: Option<usize>,
    cleanup_on_startup: Option<bool>,
    cleanup_batch_size: Option<usize>,
    cleanup_max_rows_per_startup: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ToolOutputFileConfig {
    root: Option<String>,
    externalize_above_bytes: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBudgetFileConfig {
    default_context_window_tokens: Option<u32>,
    family_context_max_ratio_bps: Option<u32>,
    family_context_hard_cap_tokens: Option<u32>,
    family_context_max_chars: Option<usize>,
    reasoning_summary_replay_enabled: Option<bool>,
    blocks: Option<ContextBlockBudgetsFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBlockBudgetsFileConfig {
    seed_prompt_prefix: Option<ContextBlockBudgetFileConfig>,
    current_session_checkpoint: Option<ContextBlockBudgetFileConfig>,
    session_family_context: Option<ContextBlockBudgetFileConfig>,
    current_session_working_state: Option<ContextBlockBudgetFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBlockBudgetFileConfig {
    max_ratio_bps: Option<u32>,
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FamilyLoadFileConfig {
    default_max_snapshots: Option<usize>,
    default_max_sessions: Option<usize>,
    default_snapshot_max_chars: Option<usize>,
    default_ttl_hours: Option<u32>,
    task_weight_bps: Option<u32>,
    requirement_weight_bps: Option<u32>,
    workspace_chat_weight_bps: Option<u32>,
    workspace_heartbeat_weight_bps: Option<u32>,
    other_weight_bps: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompactionPolicyFileConfig {
    trigger_ratio_bps: Option<u32>,
    retained_history_ratio_bps: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GoalLoopFileConfig {
    max_runs: Option<u32>,
    max_consecutive_judge_failures: Option<u32>,
    judge_max_attempts_per_run: Option<u32>,
    judge_input_max_chars: Option<usize>,
    judge_output_max_tokens: Option<u32>,
}
