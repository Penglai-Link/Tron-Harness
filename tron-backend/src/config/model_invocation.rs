use serde::{Deserialize, Serialize};

/// Budgets for one model call and the enclosing operation (including retries).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelInvocationTimeouts {
    pub call_timeout_seconds: u64,
    pub operation_timeout_seconds: u64,
}

impl ModelInvocationTimeouts {
    fn validate(&self, scope: &str) -> Result<(), String> {
        if self.call_timeout_seconds == 0
            || self.operation_timeout_seconds < self.call_timeout_seconds
        {
            return Err(format!(
                "{scope}: callTimeoutSeconds必须大于零，operationTimeoutSeconds不能小于单次调用超时"
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct EmbeddingInvocationConfig {
    pub call_timeout_seconds: u64,
    pub operation_timeout_seconds: u64,
    pub max_batch_inputs: usize,
    pub max_batch_tokens: usize,
}

impl Default for EmbeddingInvocationConfig {
    fn default() -> Self {
        Self {
            call_timeout_seconds: 180,
            operation_timeout_seconds: 600,
            max_batch_inputs: 32,
            max_batch_tokens: 32_768,
        }
    }
}

impl EmbeddingInvocationConfig {
    pub fn timeouts(&self) -> ModelInvocationTimeouts {
        ModelInvocationTimeouts {
            call_timeout_seconds: self.call_timeout_seconds,
            operation_timeout_seconds: self.operation_timeout_seconds,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ModelInvocationConfig {
    pub default_output_tokens: u32,
    pub text: ModelInvocationTimeouts,
    pub embedding: EmbeddingInvocationConfig,
    pub rerank: ModelInvocationTimeouts,
    pub verification: ModelInvocationTimeouts,
    pub agent_model_timeout_seconds: u64,
    pub agent_run_timeout_seconds: u64,
    pub external_agent: ModelInvocationTimeouts,
}

impl Default for ModelInvocationConfig {
    fn default() -> Self {
        Self {
            default_output_tokens: 16_384,
            text: ModelInvocationTimeouts {
                call_timeout_seconds: 300,
                operation_timeout_seconds: 900,
            },
            embedding: EmbeddingInvocationConfig::default(),
            rerank: ModelInvocationTimeouts {
                call_timeout_seconds: 60,
                operation_timeout_seconds: 180,
            },
            verification: ModelInvocationTimeouts {
                call_timeout_seconds: 60,
                operation_timeout_seconds: 180,
            },
            agent_model_timeout_seconds: 300,
            agent_run_timeout_seconds: 3 * 24 * 60 * 60,
            external_agent: ModelInvocationTimeouts {
                call_timeout_seconds: 3600,
                operation_timeout_seconds: 3600,
            },
        }
    }
}

impl ModelInvocationConfig {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.default_output_tokens == 0 {
            return Err("defaultOutputTokens必须大于零".into());
        }
        for (scope, limits) in [
            ("text", &self.text),
            ("rerank", &self.rerank),
            ("verification", &self.verification),
            ("externalAgent", &self.external_agent),
            ("embedding", &self.embedding.timeouts()),
        ] {
            limits.validate(scope)?;
        }
        if self.embedding.max_batch_inputs == 0 || self.embedding.max_batch_tokens == 0 {
            return Err("embedding: maxBatchInputs和maxBatchTokens必须大于零".into());
        }
        if self.agent_model_timeout_seconds == 0
            || self.agent_run_timeout_seconds < self.agent_model_timeout_seconds
        {
            return Err(
                "agentModelTimeoutSeconds必须大于零，agentRunTimeoutSeconds不能小于单次调用超时"
                    .into(),
            );
        }
        if tokio::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(self.agent_run_timeout_seconds))
            .is_none()
        {
            return Err("agentRunTimeoutSeconds超出支持范围".into());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ModelInvocationFileConfig {
    default_output_tokens: Option<u32>,
    text: Option<TimeoutFileConfig>,
    embedding: Option<EmbeddingFileConfig>,
    rerank: Option<TimeoutFileConfig>,
    verification: Option<TimeoutFileConfig>,
    agent_model_timeout_seconds: Option<u64>,
    agent_run_timeout_seconds: Option<u64>,
    // Existing local application.yaml files may still contain the old derived
    // budget. Ignore it so the new three-day default takes effect.
    agent_step_timeout_seconds: Option<u64>,
    external_agent: Option<TimeoutFileConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TimeoutFileConfig {
    call_timeout_seconds: Option<u64>,
    operation_timeout_seconds: Option<u64>,
}
impl TimeoutFileConfig {
    fn apply(self, current: &mut ModelInvocationTimeouts) {
        if let Some(value) = self.call_timeout_seconds {
            current.call_timeout_seconds = value;
        }
        if let Some(value) = self.operation_timeout_seconds {
            current.operation_timeout_seconds = value;
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EmbeddingFileConfig {
    call_timeout_seconds: Option<u64>,
    operation_timeout_seconds: Option<u64>,
    max_batch_inputs: Option<usize>,
    max_batch_tokens: Option<usize>,
}

impl ModelInvocationConfig {
    pub(super) fn apply_overrides(&mut self, file: ModelInvocationFileConfig) {
        let _ = file.agent_step_timeout_seconds;
        if let Some(value) = file.default_output_tokens {
            self.default_output_tokens = value;
        }
        for (overrides, current) in [
            (file.text, &mut self.text),
            (file.rerank, &mut self.rerank),
            (file.verification, &mut self.verification),
            (file.external_agent, &mut self.external_agent),
        ] {
            if let Some(overrides) = overrides {
                overrides.apply(current);
            }
        }
        if let Some(value) = file.agent_model_timeout_seconds {
            self.agent_model_timeout_seconds = value;
        }
        if let Some(value) = file.agent_run_timeout_seconds {
            self.agent_run_timeout_seconds = value;
        }
        if let Some(value) = file.embedding {
            if let Some(v) = value.call_timeout_seconds {
                self.embedding.call_timeout_seconds = v;
            }
            if let Some(v) = value.operation_timeout_seconds {
                self.embedding.operation_timeout_seconds = v;
            }
            if let Some(v) = value.max_batch_inputs {
                self.embedding.max_batch_inputs = v;
            }
            if let Some(v) = value.max_batch_tokens {
                self.embedding.max_batch_tokens = v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_default_is_configurable_and_zero_is_rejected() {
        let mut config = ModelInvocationConfig::default();
        assert_eq!(config.default_output_tokens, 16_384);
        config.apply_overrides(serde_yaml::from_str("defaultOutputTokens: 8192").unwrap());
        config.validate().unwrap();
        assert_eq!(config.default_output_tokens, 8192);
        config.apply_overrides(serde_yaml::from_str("defaultOutputTokens: 0").unwrap());
        assert!(
            config
                .validate()
                .unwrap_err()
                .contains("defaultOutputTokens")
        );
    }
}
