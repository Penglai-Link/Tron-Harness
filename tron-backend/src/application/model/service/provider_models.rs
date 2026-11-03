use super::*;
use crate::domain::{
    agent::{AgentModelConfig, AgentModelProtocol},
    model::{ConfiguredProviderModel, ModelSelectOption, ModelViewSourceKind},
    model_provider::{ModelSelection, ProviderModelQuality, ProviderRouteCode},
};

impl ModelService {
    pub fn list_select_options(&self) -> Result<Vec<ModelSelectOption>, ModelError> {
        let providers = self
            .list_configured_providers()
            .map_err(|_| ModelError::StorageFailure)?;
        let mut options = Vec::new();
        for provider in providers {
            for model in &provider.models {
                let code = model
                    .model_code
                    .as_ref()
                    .ok_or(ModelError::StorageFailure)?;
                options.push(ModelSelectOption {
                    option_code: code.clone(),
                    model_code: code.clone(),
                    display_name: model.name.clone(),
                    model_name: model.model_id.clone(),
                    source_kind: ModelViewSourceKind::Provider,
                    source_label: provider.display_name.clone(),
                    vendor: provider.display_name.clone(),
                    protocol: model
                        .model_type
                        .protocol(provider.protocol)
                        .ok_or(ModelError::InvalidProtocolModelType)?,
                    model_type: model.model_type.model_type(),
                    enabled: provider.enabled && model.enabled,
                    model_selection: ModelSelection::fixed(code),
                    provider_code: None,
                    access_platform: Some(provider.display_name.clone()),
                    upstream_vendor_name: None,
                    remote_status: None,
                    context_window_tokens: model.context_window_tokens,
                    quality_type: None,
                    input_coefficient: None,
                    cache_hit_coefficient: None,
                    output_coefficient: None,
                    supported_reasoning_efforts: Vec::new(),
                    default_reasoning_effort: None,
                    is_default: false,
                });
            }
        }
        Ok(options)
    }

    pub fn auto_chat_candidates(
        &self,
        _workspace: &str,
        allowed: &[String],
        reasoning: Option<ReasoningEffort>,
    ) -> Result<Vec<AgentModelConfig>, ModelError> {
        let mut candidates = Vec::new();
        for provider in self
            .list_configured_providers()
            .map_err(|_| ModelError::StorageFailure)?
        {
            if !provider.enabled {
                continue;
            }
            for model in &provider.models {
                let code = model
                    .model_code
                    .as_deref()
                    .ok_or(ModelError::StorageFailure)?;
                if model.model_type.model_type() != ModelType::Text
                    || (reasoning.is_some() && !model.reasoning)
                    || !model.enabled
                    || !model.context_window_tokens.is_some_and(|size| size > 0)
                    || (!allowed.is_empty() && !allowed.iter().any(|value| value == code))
                {
                    continue;
                }
                candidates.push(self.agent_config(&provider, model, reasoning)?);
            }
        }
        Ok(candidates)
    }

    pub fn text_agent_model_config(
        &self,
        workspace: &str,
        code: Option<&str>,
    ) -> Result<Option<AgentModelConfig>, ModelError> {
        self.text_agent_model_config_with_reasoning_effort(workspace, code, None)
    }

    pub fn text_agent_model_config_with_reasoning_effort(
        &self,
        workspace: &str,
        code: Option<&str>,
        reasoning: Option<ReasoningEffort>,
    ) -> Result<Option<AgentModelConfig>, ModelError> {
        let requested = code.map(str::trim).filter(|code| !code.is_empty());
        if let Some(code) = requested.filter(|code| code.starts_with("PERSON_AI:")) {
            let selection = if let Some(encoded) = code.strip_prefix("PERSON_AI:FIXED:") {
                let (quality, model) = encoded.split_once(':').ok_or_else(|| {
                    ModelError::RuntimeResolutionFailed("invalid provider model identity".into())
                })?;
                let quality = ProviderModelQuality::from_storage(quality).ok_or_else(|| {
                    ModelError::RuntimeResolutionFailed("invalid provider model quality".into())
                })?;
                ModelSelection::person_ai_fixed(model, quality)
            } else if code == "PERSON_AI:ROUTE:ChatAuto" {
                ModelSelection::person_ai_route(ProviderRouteCode::ChatAuto)
            } else {
                return Err(ModelError::InvalidProtocolModelType);
            };
            return self
                .providers
                .resolve_chat_selection(workspace, Some(&selection), None, reasoning)
                .map_err(|error| ModelError::RuntimeResolutionFailed(error.to_string()));
        }
        for provider in self
            .list_configured_providers()
            .map_err(|_| ModelError::StorageFailure)?
        {
            if !provider.enabled {
                continue;
            }
            for model in &provider.models {
                if model.model_type.model_type() == ModelType::Text
                    && model.enabled
                    && requested.is_none_or(|code| model.model_code.as_deref() == Some(code))
                {
                    return self.agent_config(&provider, model, reasoning).map(Some);
                }
            }
        }
        if requested.is_none() {
            return self
                .providers
                .resolve_chat_selection(workspace, None, None, reasoning)
                .map_err(|error| ModelError::RuntimeResolutionFailed(error.to_string()));
        }
        Ok(None)
    }

    fn agent_config(
        &self,
        provider: &ConfiguredModelProvider,
        model: &ConfiguredProviderModel,
        reasoning: Option<ReasoningEffort>,
    ) -> Result<AgentModelConfig, ModelError> {
        if reasoning.is_some() && !model.reasoning {
            return Err(ModelError::ReasoningEffortUnsupported);
        }
        let protocol = match provider.protocol.model_protocol() {
            ModelProtocol::OpenAiCompatible => AgentModelProtocol::ChatCompletions,
            ModelProtocol::Responses => AgentModelProtocol::Responses,
            ModelProtocol::Anthropic => AgentModelProtocol::Anthropic,
            ModelProtocol::Gemini => AgentModelProtocol::Gemini,
            _ => return Err(ModelError::InvalidProtocolModelType),
        };
        Ok(AgentModelConfig {
            model_code: model.model_code.clone().ok_or(ModelError::StorageFailure)?,
            protocol,
            base_url: provider.base_url.clone(),
            api_path: None,
            api_key: self.reveal_api_key(&provider.encrypted_api_key)?,
            request_headers: provider.request_headers.clone(),
            model_name: model.model_id.clone(),
            thinking_enabled: model.reasoning,
            supports_developer_role: model.supports_developer_role,
            reasoning_effort: reasoning,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: model.max_output_tokens,
            context_window_tokens: model.context_window_tokens,
            top_p: None,
            max_retries: None,
        })
    }

    pub fn embedding_runtime_model_config(
        &self,
        workspace: &str,
        code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        self.route_auxiliary_model(
            workspace,
            crate::domain::model::AutoRouterKind::Embedding,
            &eliteforge_agent_core::ModelChoice {
                model_code: code.map(str::to_owned),
                required_modalities: vec![eliteforge_agent_core::Modality::Text],
                ..Default::default()
            },
            None,
        )
        .map(Some)
    }
    pub fn rerank_runtime_model_config(
        &self,
        workspace: &str,
        code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        self.route_auxiliary_model(
            workspace,
            crate::domain::model::AutoRouterKind::Rerank,
            &eliteforge_agent_core::ModelChoice {
                model_code: code.map(str::to_owned),
                required_modalities: vec![eliteforge_agent_core::Modality::Text],
                ..Default::default()
            },
            None,
        )
        .map(Some)
    }
    pub(super) fn auxiliary_runtime_model(
        &self,
        code: Option<&str>,
        kind: ModelType,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        let requested = code.map(str::trim).filter(|code| !code.is_empty());
        for provider in self
            .list_configured_providers()
            .map_err(|_| ModelError::StorageFailure)?
        {
            if !provider.enabled {
                continue;
            }
            for model in &provider.models {
                if !model.enabled
                    || model.model_type.model_type() != kind
                    || requested.is_some_and(|code| model.model_code.as_deref() != Some(code))
                {
                    continue;
                }
                return Ok(Some(RuntimeModelConfig {
                    route: None,
                    model_code: model.model_code.clone().ok_or(ModelError::StorageFailure)?,
                    protocol: model
                        .model_type
                        .protocol(provider.protocol)
                        .ok_or(ModelError::InvalidProtocolModelType)?,
                    model_type: kind,
                    base_url: provider.base_url.clone(),
                    api_path: None,
                    api_key: self.reveal_api_key(&provider.encrypted_api_key)?,
                    request_headers: provider.request_headers.clone(),
                    model_name: model.model_id.clone(),
                    thinking_enabled: false,
                    supports_developer_role: false,
                    reasoning_effort: None,
                    responses_stateful_enabled: false,
                    temperature: None,
                    max_tokens: None,
                    context_window_tokens: model.context_window_tokens,
                    top_p: None,
                    max_retries: None,
                }));
            }
        }
        Ok(None)
    }
}
