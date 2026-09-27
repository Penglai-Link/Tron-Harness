use crate::{
    application::{model::RuntimeModelConfig, model_provider::ModelProviderService},
    domain::{
        agent::AgentModelProtocol,
        model::ModelProtocol,
        model_provider::{
            ModelSelection, ModelSelectionKind, ProviderCatalogModelStatus, ProviderModelQuality,
        },
    },
};

/// Resolves a text-generation route independently of the model transport.
pub trait TextModelRouter: Send + Sync {
    fn route_text(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<RuntimeModelConfig, String>;
}

#[derive(Clone)]
pub struct LlmModelRouter {
    providers: ModelProviderService,
}

impl LlmModelRouter {
    pub fn new(providers: ModelProviderService) -> Self {
        Self { providers }
    }

    pub fn route_text(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<RuntimeModelConfig, String> {
        if let Some(model_code) = model_code.map(str::trim).filter(|value| !value.is_empty()) {
            if model_code.starts_with("PERSON_AI:") {
                return self.route_person_ai(workspace_code, model_code);
            }
            return self.route_person_ai(workspace_code, model_code);
        }

        let mut person_ai = self
            .providers
            .list_text_select_options(workspace_code)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|option| {
                option.enabled
                    && option.remote_status == ProviderCatalogModelStatus::Available
                    && option.selection.kind == ModelSelectionKind::Fixed
                    && temporary_text_quality_rank(option.quality_type).is_some()
            })
            .collect::<Vec<_>>();
        person_ai.sort_by_key(|option| {
            (
                temporary_text_quality_rank(option.quality_type).unwrap_or(u8::MAX),
                option.option_code.clone(),
            )
        });
        if let Some(option) = person_ai.into_iter().next() {
            return self.route_selection(workspace_code, &option.selection);
        }

        Err("no available middle- or low-quality Person-AI Text model".into())
    }

    fn route_person_ai(
        &self,
        workspace_code: &str,
        model_code: &str,
    ) -> Result<RuntimeModelConfig, String> {
        let (requested_quality, catalog_model_code) = model_code
            .strip_prefix("PERSON_AI:FIXED:")
            .and_then(|value| {
                value
                    .split_once(':')
                    .map(|(quality, code)| (ProviderModelQuality::from_storage(quality), code))
            })
            .unwrap_or((None, model_code));
        let options = self
            .providers
            .list_text_select_options(workspace_code)
            .map_err(|error| error.to_string())?;
        let mut matches = options
            .into_iter()
            .filter(|option| {
                option.selection.model_code.as_deref() == Some(catalog_model_code)
                    && option.enabled
                    && option.remote_status == ProviderCatalogModelStatus::Available
                    && temporary_text_quality_rank(option.quality_type).is_some()
                    && requested_quality.is_none_or(|quality| option.quality_type == Some(quality))
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|option| {
            u8::from(option.quality_type != Some(ProviderModelQuality::Middle))
        });
        let option = matches.into_iter().next().ok_or_else(|| {
            "explicit LLM model must be an available middle- or low-quality Text model".to_string()
        })?;
        self.route_selection(workspace_code, &option.selection)
    }

    fn route_selection(
        &self,
        workspace_code: &str,
        selection: &ModelSelection,
    ) -> Result<RuntimeModelConfig, String> {
        let config = self
            .providers
            .resolve_chat_selection(workspace_code, Some(selection), None, None)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "LLM Text model is unavailable".to_string())?;
        Ok(runtime_from_agent(config))
    }
}

impl super::router::ModelRouter for LlmModelRouter {}

#[cfg(test)]
mod tests {
    use super::temporary_text_quality_rank;
    use crate::domain::model_provider::ProviderModelQuality;

    #[test]
    fn temporary_text_router_only_accepts_middle_and_low_quality() {
        assert_eq!(
            temporary_text_quality_rank(Some(ProviderModelQuality::Middle)),
            Some(0)
        );
        assert_eq!(
            temporary_text_quality_rank(Some(ProviderModelQuality::Low)),
            Some(1)
        );
        assert_eq!(
            temporary_text_quality_rank(Some(ProviderModelQuality::High)),
            None
        );
        assert_eq!(temporary_text_quality_rank(None), None);
    }
}

fn temporary_text_quality_rank(quality: Option<ProviderModelQuality>) -> Option<u8> {
    match quality {
        Some(ProviderModelQuality::Middle) => Some(0),
        Some(ProviderModelQuality::Low) => Some(1),
        Some(ProviderModelQuality::High) | None => None,
    }
}

fn runtime_from_agent(config: crate::domain::agent::AgentModelConfig) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: config.model_code,
        protocol: match config.protocol {
            AgentModelProtocol::Responses => ModelProtocol::Responses,
            AgentModelProtocol::ChatCompletions => ModelProtocol::OpenAiCompatible,
            AgentModelProtocol::Embeddings => ModelProtocol::OpenAiCompatibleEmbedding,
        },
        model_type: crate::domain::model::ModelType::Text,
        base_url: config.base_url,
        api_path: config.api_path,
        api_key: config.api_key,
        model_name: config.model_name,
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: config.responses_stateful_enabled,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        context_window_tokens: config.context_window_tokens,
        top_p: config.top_p,
        max_retries: config.max_retries,
    }
}

impl TextModelRouter for LlmModelRouter {
    fn route_text(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<RuntimeModelConfig, String> {
        LlmModelRouter::route_text(self, workspace_code, model_code)
    }
}
