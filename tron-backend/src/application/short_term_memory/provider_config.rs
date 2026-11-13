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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextProviderPolicy {
    pub strategy: ContextReductionStrategy,
    pub trigger_ratio_bps: u32,
    pub target_ratio_bps: u32,
    pub min_gain_tokens: usize,
}
impl ContextProviderPolicy {
    /// Presentation travels with the configured policy, never with the host UI.
    pub fn compression_metadata(&self) -> eliteforge_agent_core::ContextCompressionMetadata {
        use eliteforge_agent_core::ContextCompressionMetadata as Metadata;
        match self.strategy {
            ContextReductionStrategy::Keep => Metadata::keep(),
            ContextReductionStrategy::Bound => Metadata::bound(),
            ContextReductionStrategy::Semantic => Metadata::semantic(),
            ContextReductionStrategy::Checkpoint => Metadata::history(),
        }
    }

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
    /// Percentage shares totaling at most 100%; no independently configured token caps.
    pub shares: std::collections::BTreeMap<String, u32>,
    pub data_blocks: std::collections::BTreeMap<String, ContextDataBlockConfig>,
    /// Optional per-plugin token caps. Missing entries preserve plugin defaults.
    pub plugin_overrides: std::collections::BTreeMap<String, u32>,
    pub checkpoint: ContextProviderPolicy,
    pub working: ContextProviderPolicy,
    pub history: ContextProviderPolicy,
    /// Complete user turns retained verbatim after the history summary boundary.
    pub history_recent_turns: usize,
    pub prices: ContextPriceInputs,
    pub max_auxiliary_calls_per_step: usize,
}
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDataBlockConfig {
    pub max_tokens: Option<u32>,
    #[serde(flatten)]
    pub policy: ContextProviderPolicy,
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
                "conversation_history_summary",
                "runtime_identity",
                "dude_onboarding",
                "tool_domains",
                "current_session_history",
                "agent_directory",
                "skills.catalog",
                "page_context",
                "application_context",
                "topic_memo",
                "current_session_working_state",
                "long_term_memory",
                "knowledge_base",
                "dude_environment",
                "dude_wakeup",
            ]
            .map(str::to_owned)
            .to_vec(),
            shares: [
                ("seed_prompt_prefix", 300),
                ("current_session_history", 6900),
                ("conversation_history_summary", 200),
                ("runtime_identity", 200),
                ("dude_onboarding", 100),
                ("tool_domains", 300),
                ("agent_directory", 50),
                ("skills.catalog", 200),
                ("page_context", 50),
                ("application_context", 400),
                ("topic_memo", 200),
                ("current_session_working_state", 200),
                ("long_term_memory", 400),
                ("knowledge_base", 400),
                ("dude_environment", 50),
                ("dude_wakeup", 50),
            ]
            .into_iter()
            .map(|(id, share)| (id.into(), share))
            .collect(),
            data_blocks: [
                ("page_context", Some(2048)),
                ("application_context", Some(16_000)),
                ("long_term_memory", None),
                ("knowledge_base", Some(1200)),
                ("agent_directory", Some(2048)),
                ("runtime_identity", Some(8192)),
                ("dude_onboarding", Some(1024)),
                ("dude_environment", Some(1536)),
                ("dude_wakeup", Some(8192)),
                ("skills.catalog", Some(8192)),
                ("tool_domains", None),
            ]
            .into_iter()
            .map(|(id, max_tokens)| {
                (
                    id.into(),
                    ContextDataBlockConfig {
                        max_tokens,
                        policy: ContextProviderPolicy::new(
                            if matches!(
                                id,
                                "skills.catalog"
                                    | "runtime_identity"
                                    | "tool_domains"
                                    | "agent_directory"
                            ) {
                                ContextReductionStrategy::Keep
                            } else {
                                ContextReductionStrategy::Bound
                            },
                            8000,
                        ),
                    },
                )
            })
            .collect(),
            plugin_overrides: std::collections::BTreeMap::new(),
            checkpoint: ContextProviderPolicy::new(ContextReductionStrategy::Bound, 8_000),
            working: ContextProviderPolicy::new(ContextReductionStrategy::Bound, 8_000),
            history: ContextProviderPolicy::new(ContextReductionStrategy::Checkpoint, 2_000),
            history_recent_turns: 2,
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
    pub fn effective_shares(&self, order: &[String]) -> std::collections::BTreeMap<String, u32> {
        let mut shares = self.shares.clone();
        for id in order {
            if !shares.contains_key(id) {
                shares.insert(id.clone(), 1);
                if let Some(history) = shares.get_mut("current_session_history") {
                    *history = history.saturating_sub(1);
                }
            }
        }
        shares
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.history_recent_turns == 0 || self.history_recent_turns > 512 {
            return Err("history recent-turn retention must be between 1 and 512".into());
        }
        let required = [
            "seed_prompt_prefix",
            "conversation_history_summary",
            "current_session_history",
            "current_session_working_state",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain(self.data_blocks.keys().cloned())
        .collect::<Vec<_>>();
        let required = required
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let actual = self.order.iter().collect::<std::collections::BTreeSet<_>>();
        if self.order.iter().any(|id| id.trim().is_empty())
            || actual.len() != self.order.len()
            || required.iter().any(|id| !actual.contains(id))
        {
            return Err(
                "context order must contain every configured context block exactly once".into(),
            );
        }
        if self.order.first().map(String::as_str) != Some("seed_prompt_prefix") {
            return Err("fixed instructions must be first".into());
        }
        eliteforge_agent_memory::ContextShareAllocation::new(self.shares.clone())
            .map_err(|error| error.to_string())?;
        let expected_data = Self::default().data_blocks;
        if expected_data
            .keys()
            .any(|id| !self.data_blocks.contains_key(id))
        {
            return Err("every data context block requires an explicit policy".into());
        }
        for (id, block) in &self.data_blocks {
            if id.trim().is_empty()
                || !matches!(
                    block.policy.strategy,
                    ContextReductionStrategy::Keep | ContextReductionStrategy::Bound
                )
            {
                return Err("context data requires an explicit keep or bound strategy".into());
            }
        }
        for policy in [&self.checkpoint, &self.working] {
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
        for policy in [&self.checkpoint, &self.working, &self.history]
            .into_iter()
            .chain(self.data_blocks.values().map(|block| &block.policy))
        {
            if policy.strategy != ContextReductionStrategy::Keep
                && policy.target_ratio_bps >= policy.trigger_ratio_bps
            {
                return Err("compression target must be below trigger".into());
            }
        }
        self.prices.validate()
    }
}
