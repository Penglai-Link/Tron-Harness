//! Host configuration maps to public Framework context policies.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextReductionStrategy {
    Keep,
    Bound,
    Semantic,
    Checkpoint,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextProviderPolicy {
    pub strategy: ContextReductionStrategy,
    pub trigger_ratio_bps: u32,
    pub target_ratio_bps: u32,
    pub min_gain_tokens: usize,
}
impl ContextProviderPolicy {
    fn new(strategy: ContextReductionStrategy, target: u32) -> Self {
        Self {
            strategy,
            trigger_ratio_bps: 10_000,
            target_ratio_bps: target,
            min_gain_tokens: 1,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ContextProvidersConfig {
    /// Actual message block order. Tools retain their provider-native protocol location.
    pub order: Vec<String>,
    pub checkpoint: ContextProviderPolicy,
    pub family: ContextProviderPolicy,
    pub working: ContextProviderPolicy,
    pub history: ContextProviderPolicy,
    pub prices: ContextPriceInputs,
    pub max_auxiliary_calls_per_step: usize,
}
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPriceInputs {
    /// Optional host-supplied decimal price per million tokens; no default prices.
    pub input: Option<String>,
    pub cached_input: Option<String>,
    pub output: Option<String>,
    pub currency: Option<String>,
}
impl Default for ContextProvidersConfig {
    fn default() -> Self {
        Self {
            order: [
                "seed_prompt_prefix",
                "application_context",
                "current_session_checkpoint",
                "session_family_context",
                "current_session_working_state",
                "knowledge_base",
                "current_session_history",
            ]
            .map(str::to_owned)
            .to_vec(),
            checkpoint: ContextProviderPolicy::new(ContextReductionStrategy::Bound, 8_000),
            family: ContextProviderPolicy::new(ContextReductionStrategy::Bound, 8_000),
            working: ContextProviderPolicy::new(ContextReductionStrategy::Bound, 8_000),
            history: ContextProviderPolicy::new(ContextReductionStrategy::Checkpoint, 2_000),
            prices: ContextPriceInputs::default(),
            max_auxiliary_calls_per_step: 16,
        }
    }
}

impl ContextPriceInputs {
    /// Validated optional estimate inputs, never a billing feed or capacity override.
    pub fn validate(&self) -> Result<(), String> {
        let values = [&self.input, &self.cached_input, &self.output];
        let provided = values.iter().any(|value| value.is_some());
        if provided
            != self
                .currency
                .as_ref()
                .is_some_and(|currency| !currency.trim().is_empty())
        {
            return Err(
                "context price inputs require a nonempty currency and at least one price".into(),
            );
        }
        for value in values.into_iter().flatten() {
            let mut fields = value.split('.');
            let whole = fields.next().unwrap_or_default();
            let fraction = fields.next();
            if whole.is_empty()
                || !whole.bytes().all(|c| c.is_ascii_digit())
                || fraction.is_some_and(|field| {
                    field.is_empty() || !field.bytes().all(|c| c.is_ascii_digit())
                })
                || fields.next().is_some()
                || value.len() > 64
            {
                return Err("context prices must be bounded nonnegative decimal strings".into());
            }
        }
        Ok(())
    }
}

impl ContextProvidersConfig {
    pub fn validate(&self) -> Result<(), String> {
        let expected = ContextProvidersConfig::default().order;
        let actual = self.order.iter().collect::<std::collections::BTreeSet<_>>();
        if self.order.len() != expected.len()
            || actual.len() != expected.len()
            || expected.iter().any(|id| !actual.contains(id))
        {
            return Err("context order must contain every context block exactly once".into());
        }
        for policy in [&self.checkpoint, &self.family, &self.working] {
            if policy.strategy == ContextReductionStrategy::Checkpoint {
                return Err("checkpoint strategy is only supported for history".into());
            }
        }
        if !matches!(
            self.history.strategy,
            ContextReductionStrategy::Keep | ContextReductionStrategy::Checkpoint
        ) {
            return Err("history supports only keep or checkpoint".into());
        }
        for policy in [&self.checkpoint, &self.family, &self.working, &self.history] {
            if policy.strategy != ContextReductionStrategy::Keep
                && policy.target_ratio_bps >= policy.trigger_ratio_bps
            {
                return Err("compression target must be below trigger".into());
            }
        }
        self.prices.validate()
    }
}
