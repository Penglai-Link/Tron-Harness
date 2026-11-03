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

#[cfg(test)]
mod token_estimator_tests {
    use super::*;
    use fw::TokenEstimator;
    fn estimator(name: &str) -> TronTokenEstimator {
        TronTokenEstimator::new(
            &serde_json::from_value(serde_json::json!({
                "modelCode":"catalog-uuid", "modelName":name, "protocol":"CHAT_COMPLETIONS",
                "baseUrl":"https://example.test", "apiKey":"unused"
            }))
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn excludes_history_only_parts_but_counts_visible_content_and_calls() {
        let model = fw::ModelRef::new("test", "test");
        let mut clean = fw::Message::new(
            fw::Role::Assistant,
            fw::Content(vec![fw::ContentBlock::Text {
                text: "visible answer".into(),
            }]),
        );
        clean.tool_calls.push(fw::ToolCall::new(
            "call-1",
            "lookup",
            serde_json::json!({"query":"a2ui"}),
        ));
        let mut recorded = clean.clone();
        recorded.content.0.push(fw::ContentBlock::Json { value: serde_json::json!({"tronMessagePart": {"type":"reasoning", "text":"x".repeat(20000)}}) });
        recorded.content.0.push(fw::ContentBlock::Json { value: serde_json::json!({"tronMessagePart": {"type":"toolCall", "call_id":"call-1", "tool_name":"lookup", "input":{"query":"a2ui"}}}) });
        recorded.model_context = Some(fw::ModelMessageContext {
            model: model.clone(),
            protocol: "test".into(),
            format: "framework.provider-metrics.v1".into(),
            data: serde_json::json!({"padding":"x".repeat(20000)}),
        });
        let baseline = estimator("unknown").message(&model, &clean).await.unwrap();
        assert_eq!(
            estimator("unknown")
                .message(&model, &recorded)
                .await
                .unwrap(),
            baseline
        );
        clean.tool_calls.clear();
        assert!(baseline > estimator("unknown").message(&model, &clean).await.unwrap());
        recorded.content.0.push(fw::ContentBlock::Text {
            text: "more visible content".into(),
        });
        assert!(
            estimator("unknown")
                .message(&model, &recorded)
                .await
                .unwrap()
                > baseline
        );
    }

    #[test]
    fn resolves_families_from_vendor_name_and_records_estimate_provenance() {
        for (name, id) in [
            ("provider/DeepSeek V4.1 Flash", "deepseek-v4.1-flash"),
            ("provider/GLM-5.3", "glm-5.3-flash"),
            ("provider/QWEN3.5-plus", "qwen3.5"),
            ("provider/Kimi-K2.5", "kimi-k2.5"),
            ("provider/GPT-5.4", "o200k_base"),
            ("provider/Claude Opus", "claude-legacy"),
        ] {
            assert_eq!(estimator(name).tokenizer.tokenizer_id, id);
        }
        assert_eq!(
            estimator("deepseek/unrelated").tokenizer.tokenizer_id,
            "unicode_estimate"
        );
        assert_eq!(
            TronTokenEstimator::source(&config("DeepSeek-new")),
            "deepseek-v4.1-flash_family_estimate"
        );
        assert_eq!(
            TronTokenEstimator::source(&config("deepseek-v4.1-flash")),
            "deepseek-v4.1-flash"
        );
        assert_eq!(
            TronTokenEstimator::source(&config("claude-opus-4.6")),
            "claude-legacy_estimate"
        );
    }

    fn config(name: &str) -> AgentModelConfig {
        serde_json::from_value(serde_json::json!({
            "modelCode":"catalog-uuid", "modelName":name, "protocol":"CHAT_COMPLETIONS",
            "baseUrl":"https://example.test", "apiKey":"unused"
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn same_registered_counter_prices_messages_tools_and_schema() {
        let mut registry = builtin_tokenizer_registry().clone();
        registry
            .bind(
                fw::TokenizerMatcher::Exact("host-model".into()),
                "qwen3.5",
                fw::TokenizerAccuracy::ModelTokenizer,
                0,
            )
            .unwrap();
        let estimator = TronTokenEstimator::with_registry(&config("host-model"), &registry);
        let model = fw::ModelRef::new("test", "catalog-uuid");
        let text = "中文 JSON {\"query\": \"test\"}";
        assert_eq!(
            estimator
                .message(&model, &fw::Message::user(text))
                .await
                .unwrap(),
            16 + registry.select("host-model").count_tokens(text).unwrap()
        );
        let options = fw::GenerationOptions {
            output_schema: Some(fw::OutputSchema {
                name: "answer".into(),
                schema: serde_json::json!({"type":"object"}),
            }),
            ..Default::default()
        };
        assert_eq!(
            estimator.options(&model, &options).await.unwrap(),
            estimator
                .json_tokens(options.output_schema.as_ref().unwrap())
                .unwrap()
        );
        let tool = fw::ToolDescriptor::new(
            "lookup",
            "lookup",
            "查询资料",
            serde_json::json!({"type":"object"}),
        );
        let native = EffectiveTool {
            tool_name: "lookup".into(),
            domain_code: None,
            description: "查询资料".into(),
            input_schema: serde_json::json!({"type":"object"}),
            skill_name: None,
            required_variable_keys: vec![],
        };
        let payload = crate::application::agent::tool_schema_payload(estimator.protocol, &[native]);
        assert_eq!(
            estimator.tools(&model, &[tool]).await.unwrap(),
            registry
                .select("host-model")
                .count_tokens(&serde_json::to_string(&payload).unwrap())
                .unwrap()
        );
    }

    #[tokio::test]
    async fn counts_selected_wire_schema_and_ignores_persistence_sidecars() {
        let model = fw::ModelRef::new("test", "catalog-uuid");
        let estimator = estimator("deepseek-v4.1-flash-1017");
        let mut message = fw::Message::user("中文上下文，包含代码和 JSON。");
        let baseline = estimator.message(&model, &message).await.unwrap();
        message.structured = Some(serde_json::json!({"history":"x".repeat(10000)}));
        message.model_context = Some(fw::ModelMessageContext {
            model: model.clone(),
            protocol: "responses".into(),
            format: "tron.responses.v2".into(),
            data: serde_json::json!({"continuation":"x".repeat(10000)}),
        });
        assert_eq!(estimator.message(&model, &message).await.unwrap(), baseline);
        assert_eq!(estimator.tools(&model, &[]).await.unwrap(), 0);
        let mut tool = fw::ToolDescriptor::new(
            "lookup",
            "lookup",
            "查询资料",
            serde_json::json!({"type":"object"}),
        );
        tool.description = "查询资料".into();
        let native = EffectiveTool {
            tool_name: tool.call_name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
            domain_code: None,
            skill_name: None,
            required_variable_keys: vec![],
        };
        let payload = crate::application::agent::tool_schema_payload(estimator.protocol, &[native]);
        assert_eq!(
            estimator.tools(&model, &[tool]).await.unwrap(),
            estimator.json_tokens(&payload).unwrap()
        );
    }

    #[tokio::test]
    async fn counts_tool_output_without_history_metadata() {
        let model = fw::ModelRef::new("test", "test");
        let mut message = fw::Message::new(
            fw::Role::Tool,
            fw::Content(vec![fw::ContentBlock::Text {
                text: "small result".into(),
            }]),
        );
        message.tool_call_id = Some("call-1".into());
        let estimate = estimator("unknown")
            .message(&model, &message)
            .await
            .unwrap();
        assert_eq!(
            estimate,
            16 + fw::estimate_unicode_text_tokens("small result")
        );
    }
}
