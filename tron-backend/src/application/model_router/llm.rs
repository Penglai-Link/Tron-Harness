use crate::{
    application::{model::RuntimeModelConfig, model_provider::ModelProviderService},
    domain::{
        agent::AgentModelProtocol,
        model::ModelProtocol,
        model_provider::{ModelSelection, ProviderModelQuality, ProviderRouteCode},
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
    models: Option<crate::application::model::ModelService>,
}

impl LlmModelRouter {
    pub fn new(providers: ModelProviderService) -> Self {
        Self {
            providers,
            models: None,
        }
    }

    pub fn with_configured_providers(
        mut self,
        models: crate::application::model::ModelService,
    ) -> Self {
        self.models = Some(models);
        self
    }

    pub fn route_text(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<RuntimeModelConfig, String> {
        let requested = model_code.map(str::trim).filter(|value| !value.is_empty());
        if let Some(models) = &self.models {
            if requested == Some("AUTO:Chat") {
                return models
                    .route_automatic_model(
                        workspace_code,
                        crate::domain::model::AutoRouterKind::Chat,
                    )
                    .map_err(|e| e.to_string());
            }
            if requested.is_none()
                || matches!(
                    requested,
                    Some("AUTO:TextGeneration" | "PERSON_AI:ROUTE:TextGenerationAuto")
                )
            {
                return models
                    .route_automatic_model(
                        workspace_code,
                        crate::domain::model::AutoRouterKind::TextGeneration,
                    )
                    .map_err(|e| e.to_string());
            }
            if let Some(code) = requested.filter(|code| !code.starts_with("PERSON_AI:")) {
                return models
                    .text_agent_model_config(workspace_code, Some(code))
                    .map_err(|e| e.to_string())?
                    .map(runtime_from_agent)
                    .ok_or_else(|| "selected model is unavailable".into());
            }
        }
        if let Some(model_code) = model_code.map(str::trim).filter(|value| !value.is_empty()) {
            if model_code == "PERSON_AI:ROUTE:TextGenerationAuto" {
                return self
                    .providers
                    .text_generation_auto_config(workspace_code)
                    .map_err(|error| error.to_string());
            }
            if model_code == "PERSON_AI:ROUTE:ChatAuto" {
                return self.route_selection(
                    workspace_code,
                    &ModelSelection::person_ai_route(ProviderRouteCode::ChatAuto),
                );
            }
            return self.route_person_ai(workspace_code, model_code);
        }

        self.providers
            .text_generation_auto_config(workspace_code)
            .map_err(|error| error.to_string())
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
        let candidates = self
            .providers
            .text_generation_candidates(workspace_code)
            .map_err(|error| error.to_string())?;
        let matches = self
            .providers
            .list_temporary_text_select_options(workspace_code)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|option| {
                option.selection.model_code.as_deref() == Some(catalog_model_code)
                    && requested_quality.is_none_or(|quality| option.quality_type == Some(quality))
                    && option.quality_type.is_some_and(|quality| {
                        candidates.iter().any(|candidate| {
                            candidate.model.model
                                == format!(
                                    "PERSON_AI:FIXED:{}:{}",
                                    quality.storage_value(),
                                    catalog_model_code
                                )
                        })
                    })
            })
            .collect::<Vec<_>>();
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
            .resolve_temporary_text_selection(workspace_code, selection)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "LLM Text model is unavailable".to_string())?;
        Ok(runtime_from_agent(config))
    }
}

impl super::router::ModelRouter for LlmModelRouter {}

pub(crate) fn runtime_from_agent(
    config: crate::domain::agent::AgentModelConfig,
) -> RuntimeModelConfig {
    RuntimeModelConfig {
        route: None,
        model_code: config.model_code,
        protocol: match config.protocol {
            AgentModelProtocol::Responses => ModelProtocol::Responses,
            AgentModelProtocol::ChatCompletions => ModelProtocol::OpenAiCompatible,
            AgentModelProtocol::Embeddings => ModelProtocol::OpenAiCompatibleEmbedding,
            AgentModelProtocol::Anthropic => ModelProtocol::Anthropic,
            AgentModelProtocol::Gemini => ModelProtocol::Gemini,
        },
        model_type: crate::domain::model::ModelType::Text,
        base_url: config.base_url,
        api_path: config.api_path,
        api_key: config.api_key,
        request_headers: config.request_headers,
        model_name: config.model_name,
        thinking_enabled: config.thinking_enabled,
        supports_developer_role: config.supports_developer_role,
        reasoning_effort: config.reasoning_effort,
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
