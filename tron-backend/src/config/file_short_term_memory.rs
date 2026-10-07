use serde::Deserialize;

use crate::config::percentage::optional_ratio;

use crate::application::short_term_memory::{
    CompactionPolicyConfig, ContextBlockBudgetConfig, ContextBlockBudgets, ContextBudgetConfig,
    FamilyLoadConfig, GoalLoopConfig, ShortTermMemoryConfig,
};

impl ShortTermMemoryConfig {
    pub(super) fn apply_overrides(&mut self, overrides: ShortTermMemoryFileConfig) {
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
        if let Some(value) = overrides.providers {
            value.apply_to(&mut self.providers);
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
        if let Some(value) = overrides.default_max_sessions {
            self.default_max_sessions = value.max(1);
        }
    }
}

trait ApplyMemoryOverrides<T> {
    fn apply_overrides(&mut self, overrides: T);
}

impl ApplyMemoryOverrides<ContextBlockBudgetsFileConfig> for ContextBlockBudgets {
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
        if let Some(value) = overrides.tool_schema {
            self.tool_schema.apply_overrides(value);
        }
    }
}

impl ApplyMemoryOverrides<ContextBlockBudgetFileConfig> for ContextBlockBudgetConfig {
    fn apply_overrides(&mut self, overrides: ContextBlockBudgetFileConfig) {
        if let Some(value) = overrides.max_ratio_bps {
            self.max_ratio_bps = Some(value.min(10_000));
        }
        if let Some(value) = overrides.max_tokens {
            self.max_tokens = value.map(|value| value.max(1));
        }
    }
}

impl CompactionPolicyConfig {
    fn apply_overrides(&mut self, overrides: CompactionPolicyFileConfig) {
        if let Some(enabled) = overrides.auto_enabled {
            self.auto_enabled = enabled;
        }
        if let Some(value) = overrides.trigger_ratio_bps {
            self.trigger_ratio_bps = value.min(10_000);
        }
    }
}

impl GoalLoopConfig {
    fn apply_overrides(&mut self, overrides: GoalLoopFileConfig) {
        if let Some(value) = overrides.max_runs {
            self.max_runs = value.max(1);
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ShortTermMemoryFileConfig {
    context: Option<ContextBudgetFileConfig>,
    family_load: Option<FamilyLoadFileConfig>,
    compaction: Option<CompactionPolicyFileConfig>,
    goal_loop: Option<GoalLoopFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBudgetFileConfig {
    default_context_window_tokens: Option<u32>,
    #[serde(
        default,
        rename = "familyContextMaxRatio",
        alias = "familyContextMaxRatioBps",
        deserialize_with = "optional_ratio"
    )]
    family_context_max_ratio_bps: Option<u32>,
    family_context_hard_cap_tokens: Option<u32>,
    family_context_max_chars: Option<usize>,
    blocks: Option<ContextBlockBudgetsFileConfig>,
    providers: Option<ContextProvidersFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBlockBudgetsFileConfig {
    seed_prompt_prefix: Option<ContextBlockBudgetFileConfig>,
    current_session_checkpoint: Option<ContextBlockBudgetFileConfig>,
    session_family_context: Option<ContextBlockBudgetFileConfig>,
    current_session_working_state: Option<ContextBlockBudgetFileConfig>,
    tool_schema: Option<ContextBlockBudgetFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBlockBudgetFileConfig {
    #[serde(
        default,
        rename = "maxRatio",
        alias = "maxRatioBps",
        deserialize_with = "optional_ratio"
    )]
    max_ratio_bps: Option<u32>,
    #[serde(default, deserialize_with = "explicit_optional_tokens")]
    max_tokens: Option<Option<u32>>,
}

// Missing preserves the default; explicit YAML null removes only this cap.
fn explicit_optional_tokens<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<u32>>, D::Error> {
    Option::<u32>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FamilyLoadFileConfig {
    default_max_sessions: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompactionPolicyFileConfig {
    auto_enabled: Option<bool>,
    #[serde(
        default,
        rename = "triggerRatio",
        alias = "triggerRatioBps",
        deserialize_with = "optional_ratio"
    )]
    trigger_ratio_bps: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GoalLoopFileConfig {
    max_runs: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextProvidersFileConfig {
    order: Option<Vec<String>>,
    checkpoint: Option<ContextProviderFilePolicy>,
    family: Option<ContextProviderFilePolicy>,
    working: Option<ContextProviderFilePolicy>,
    history: Option<ContextProviderFilePolicy>,
    prices: Option<crate::application::short_term_memory::ContextPriceInputs>,
    max_auxiliary_calls_per_step: Option<usize>,
}
impl ContextProvidersFileConfig {
    fn apply_to(self, config: &mut crate::application::short_term_memory::ContextProvidersConfig) {
        if let Some(order) = self.order {
            config.order = order;
        }
        if let Some(policy) = self.checkpoint {
            policy.apply_to(&mut config.checkpoint);
        }
        if let Some(policy) = self.family {
            policy.apply_to(&mut config.family);
        }
        if let Some(policy) = self.working {
            policy.apply_to(&mut config.working);
        }
        if let Some(policy) = self.history {
            policy.apply_to(&mut config.history);
        }
        if let Some(prices) = self.prices {
            config.prices = prices;
        }
        if let Some(calls) = self.max_auxiliary_calls_per_step {
            config.max_auxiliary_calls_per_step = calls;
        }
    }
}
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextProviderFilePolicy {
    strategy: Option<crate::application::short_term_memory::ContextReductionStrategy>,
    #[serde(
        default,
        rename = "triggerRatio",
        alias = "triggerRatioBps",
        deserialize_with = "optional_ratio"
    )]
    trigger_ratio_bps: Option<u32>,
    #[serde(
        default,
        rename = "targetRatio",
        alias = "targetRatioBps",
        deserialize_with = "optional_ratio"
    )]
    target_ratio_bps: Option<u32>,
    min_gain_tokens: Option<usize>,
}
impl ContextProviderFilePolicy {
    fn apply_to(self, config: &mut crate::application::short_term_memory::ContextProviderPolicy) {
        if let Some(strategy) = self.strategy {
            config.strategy = strategy;
        }
        if let Some(trigger) = self.trigger_ratio_bps {
            config.trigger_ratio_bps = trigger.min(10_000);
        }
        if let Some(target) = self.target_ratio_bps {
            config.target_ratio_bps = target.min(10_000);
        }
        if let Some(gain) = self.min_gain_tokens {
            config.min_gain_tokens = gain;
        }
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;
    #[test]
    fn provider_percent_strings_and_legacy_bps_preserve_layout_and_strategy() {
        for ratio in [
            "triggerRatio: '75%'\n      targetRatio: '25%'",
            "triggerRatioBps: 7500\n      targetRatioBps: 2500",
        ] {
            let text = format!(
                "context:\n  providers:\n    order: [application_context, seed_prompt_prefix, current_session_checkpoint, session_family_context, current_session_working_state, current_session_history]\n    working:\n      strategy: semantic\n      {ratio}\n"
            );
            let overrides: ShortTermMemoryFileConfig = serde_yaml::from_str(&text).unwrap();
            let mut config = ShortTermMemoryConfig::default();
            config.apply_overrides(overrides);
            assert_eq!(config.context.providers.order[0], "application_context");
            assert_eq!(
                config.context.providers.working.strategy,
                crate::application::short_term_memory::ContextReductionStrategy::Semantic
            );
            assert_eq!(config.context.providers.working.trigger_ratio_bps, 7500);
            assert_eq!(config.context.providers.working.target_ratio_bps, 2500);
        }
    }

    #[test]
    fn provider_optional_prices_are_bounded_validated_estimate_inputs() {
        use crate::application::short_term_memory::ContextPriceInputs;
        assert!(ContextPriceInputs::default().validate().is_ok());
        let price = ContextPriceInputs {
            input: Some("1.25".into()),
            currency: Some("USD".into()),
            ..Default::default()
        };
        assert!(price.validate().is_ok());
        for input in ["-1", "nan", "1e3", "1.2.3", "", "1."] {
            let mut invalid = price.clone();
            invalid.input = Some(input.into());
            assert!(invalid.validate().is_err());
        }
        let mut invalid = price;
        invalid.currency = None;
        assert!(invalid.validate().is_err());
    }
}
