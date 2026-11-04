//! Count the model-visible projection, using a pinned tokenizer when known.
//! Framing remains an estimate; provider usage is the authority after dispatch.
use crate::domain::agent::{AgentModelConfig, AgentModelProtocol, EffectiveTool};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_model_http::tokenization::builtin_tokenizer_registry;

/// One model binding owns one protocol and tokenizer selection. The same
/// instance prices block assembly and the Framework's final model admission.
pub(crate) struct TronTokenEstimator {
    protocol: AgentModelProtocol,
    tokenizer: fw::TokenizerSelection,
}

impl TronTokenEstimator {
    pub(crate) fn new(config: &AgentModelConfig) -> Self {
        Self::with_registry(config, builtin_tokenizer_registry())
    }

    pub(crate) fn with_registry(
        config: &AgentModelConfig,
        registry: &fw::TokenizerRegistry,
    ) -> Self {
        Self {
            protocol: config.protocol,
            tokenizer: registry.select(&config.model_name),
        }
    }

    pub(crate) fn source(config: &AgentModelConfig) -> String {
        let selection = builtin_tokenizer_registry().select(&config.model_name);
        match selection.accuracy {
            fw::TokenizerAccuracy::ModelTokenizer => selection.tokenizer_id,
            fw::TokenizerAccuracy::FamilyEstimate => {
                format!("{}_family_estimate", selection.tokenizer_id)
            }
            fw::TokenizerAccuracy::Heuristic if selection.tokenizer_id == "unicode_estimate" => {
                selection.tokenizer_id
            }
            fw::TokenizerAccuracy::Heuristic => format!("{}_estimate", selection.tokenizer_id),
        }
    }

    fn text_tokens(&self, text: &str) -> Result<usize, fw::ModelError> {
        self.tokenizer
            .count_tokens(text)
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))
    }

    fn json_tokens<T: serde::Serialize>(&self, value: &T) -> Result<usize, fw::ModelError> {
        let json =
            serde_json::to_string(value).map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
        self.text_tokens(&json)
    }
}

#[async_trait]
impl fw::TokenEstimator for TronTokenEstimator {
    async fn message(
        &self,
        _: &fw::ModelRef,
        message: &fw::Message,
    ) -> Result<usize, fw::ModelError> {
        let native = super::messages::decode(message).map_err(fw::ModelError::Fatal)?;
        let text = if message.role == fw::Role::Tool {
            if message
                .tool_call_id
                .as_deref()
                .is_some_and(|id| !id.starts_with("generated_call_"))
            {
                native.first_tool_output().unwrap_or_default().to_owned()
            } else {
                native.text_content()
            }
        } else {
            native.plain_text_content()
        };
        let mut tokens = 16usize.saturating_add(self.text_tokens(&text)?);
        if message.role == fw::Role::User {
            for block in &message.content.0 {
                if let fw::ContentBlock::Media { media } = block {
                    tokens = tokens.saturating_add(media.estimated_tokens.unwrap_or(usize::MAX));
                }
            }
        }
        for call in &message.tool_calls {
            tokens = tokens
                .saturating_add(16)
                .saturating_add(self.text_tokens(&call.id)?)
                .saturating_add(self.text_tokens(&call.call_name)?)
                .saturating_add(self.json_tokens(&call.arguments)?);
        }
        // structured/tool_result, annotations and model_context are persistence
        // sidecars. The HTTP bridge sends the decoded content above, and restores
        // Responses continuation separately; serializing those copies counts the
        // same transcript again. Reasoning is also not replayed as prompt text.
        Ok(tokens)
    }

    async fn options(
        &self,
        _: &fw::ModelRef,
        options: &fw::GenerationOptions,
    ) -> Result<usize, fw::ModelError> {
        options
            .output_schema
            .as_ref()
            .map_or(Ok(0), |schema| self.json_tokens(schema))
    }

    async fn tools(
        &self,
        _: &fw::ModelRef,
        tools: &[fw::ToolDescriptor],
    ) -> Result<usize, fw::ModelError> {
        if tools.is_empty() {
            return Ok(0);
        }
        let native: Vec<_> = tools
            .iter()
            .map(|tool| EffectiveTool {
                tool_name: tool.call_name.clone(),
                domain_code: None,
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                skill_name: None,
                required_variable_keys: Vec::new(),
            })
            .collect();
        // Count the selected SDK wire format, never the maximum of four
        // unrelated provider formats or internal ToolDescriptor metadata.
        let payload = crate::application::agent::tool_schema_payload(self.protocol, &native);
        self.json_tokens(&payload)
    }
}
