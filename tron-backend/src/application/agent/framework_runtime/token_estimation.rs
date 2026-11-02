//! Count the model-visible projection, using a pinned tokenizer when known.
//! Framing remains an estimate; provider usage is the authority after dispatch.
use crate::domain::agent::{AgentModelConfig, AgentModelProtocol, EffectiveTool};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenProfile {
    Unicode,
    GlmFlash,
    DeepSeekFlash,
}

impl TokenProfile {
    fn for_name(name: &str) -> Self {
        // Match the routed vendor name, including known dated aliases. A UUID
        // in the product catalog is not a vendor model name.
        let name = name.rsplit('/').next().unwrap_or(name).to_ascii_lowercase();
        match name.as_str() {
            "glm-5.3-flash" => Self::GlmFlash,
            "deepseek-v4.1-flash" | "deepseek-v4.1-flash-1017" => Self::DeepSeekFlash,
            _ => Self::Unicode,
        }
    }

    fn tokenizer(self) -> Option<&'static tokenizers::Tokenizer> {
        fn load(bytes: &[u8]) -> Option<tokenizers::Tokenizer> {
            use std::io::Read;
            let mut json = Vec::new();
            flate2::read::GzDecoder::new(bytes)
                .read_to_end(&mut json)
                .ok()?;
            tokenizers::Tokenizer::from_bytes(json).ok()
        }
        static GLM: OnceLock<Option<tokenizers::Tokenizer>> = OnceLock::new();
        static DEEPSEEK: OnceLock<Option<tokenizers::Tokenizer>> = OnceLock::new();
        match self {
            Self::Unicode => None,
            Self::GlmFlash => GLM
                .get_or_init(|| {
                    load(include_bytes!(
                        "../../../../resources/tokenizers/glm-5.3-flash.tokenizer.json.gz"
                    ))
                })
                .as_ref(),
            Self::DeepSeekFlash => DEEPSEEK
                .get_or_init(|| {
                    load(include_bytes!(
                        "../../../../resources/tokenizers/deepseek-v4.1-flash.tokenizer.json.gz"
                    ))
                })
                .as_ref(),
        }
    }

    fn text_tokens(self, text: &str) -> usize {
        self.tokenizer()
            .and_then(|tokenizer| tokenizer.encode(text, false).ok())
            .map_or_else(
                || fw::estimate_unicode_text_tokens(text),
                |encoding| encoding.len(),
            )
    }

    fn json_tokens<T: serde::Serialize>(self, value: &T) -> usize {
        serde_json::to_string(value).map_or(usize::MAX, |text| self.text_tokens(&text))
    }
}

/// One model binding owns one protocol and tokenizer selection. The same
/// instance prices block assembly and the Framework's final model admission.
pub(crate) struct TronTokenEstimator {
    protocol: AgentModelProtocol,
    profile: TokenProfile,
}

impl TronTokenEstimator {
    pub(crate) fn new(config: &AgentModelConfig) -> Self {
        Self {
            protocol: config.protocol,
            profile: TokenProfile::for_name(&config.model_name),
        }
    }

    pub(crate) fn source(config: &AgentModelConfig) -> &'static str {
        match TokenProfile::for_name(&config.model_name) {
            TokenProfile::DeepSeekFlash if TokenProfile::DeepSeekFlash.tokenizer().is_some() => {
                "deepseek-v4.1-flash"
            }
            TokenProfile::GlmFlash if TokenProfile::GlmFlash.tokenizer().is_some() => {
                "glm-5.3-flash"
            }
            _ => "unicode_estimate",
        }
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
        let mut tokens = 16usize.saturating_add(self.profile.text_tokens(&text));
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
                .saturating_add(self.profile.text_tokens(&call.id))
                .saturating_add(self.profile.text_tokens(&call.call_name))
                .saturating_add(self.profile.json_tokens(&call.arguments));
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
        Ok(options
            .output_schema
            .as_ref()
            .map_or(0, |schema| self.profile.json_tokens(schema)))
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
        Ok(self.profile.json_tokens(&payload))
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
    fn resolves_vendor_tokenizer_from_routed_name_not_catalog_uuid() {
        let glm = estimator("glm_plan/glm-5.3-flash");
        let deepseek = estimator("alibaba_token_plan/deepseek/deepseek-v4.1-flash-1017");
        assert!(glm.profile.tokenizer().is_some());
        assert!(deepseek.profile.tokenizer().is_some());
        assert_eq!(estimator("deepseek-v3.2").profile, TokenProfile::Unicode);
        assert_eq!(
            estimator("unrelated-deepseek-v4.1-flash").profile,
            TokenProfile::Unicode
        );
        let sample = "中文上下文估算应该使用对应模型的分词器。 JSON: {\"items\":[1,2,3]}";
        assert_eq!(
            deepseek.profile.text_tokens(sample),
            deepseek
                .profile
                .tokenizer()
                .unwrap()
                .encode(sample, false)
                .unwrap()
                .len()
        );
        assert!(deepseek.profile.text_tokens(sample) < fw::estimate_unicode_text_tokens(sample));
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
            estimator.profile.json_tokens(&payload)
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
