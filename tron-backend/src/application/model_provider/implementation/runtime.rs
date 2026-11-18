use super::*;

#[derive(Clone)]
pub struct ProviderAwareLlmRuntimeResolver {
    providers: ModelProviderService,
    configured_models: Option<ModelService>,
}

impl ProviderAwareLlmRuntimeResolver {
    pub fn with_configured_providers(mut self, models: ModelService) -> Self {
        self.configured_models = Some(models);
        self
    }
    pub fn new(providers: ModelProviderService) -> Self {
        Self {
            providers,
            configured_models: None,
        }
    }
}

impl LlmRuntimeModelResolver for ProviderAwareLlmRuntimeResolver {
    fn auxiliary_runtime_model_config(
        &self,
        workspace: &str,
        kind: crate::domain::model::AutoRouterKind,
        choice: &eliteforge_agent_core::ModelChoice,
        dimensions: Option<usize>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        if let Some(models) = &self.configured_models {
            return models
                .route_auxiliary_model(workspace, kind, choice, dimensions)
                .map(Some);
        }
        if choice.provider_id.is_some()
            || choice
                .required_modalities
                .iter()
                .any(|m| *m != eliteforge_agent_core::Modality::Text)
        {
            return Err(ModelError::RuntimeResolutionFailed(
                "route constraints require a configured model catalogue".into(),
            ));
        }
        match kind {
            crate::domain::model::AutoRouterKind::Embedding => {
                self.embedding_runtime_model_config(workspace, choice.model_code.as_deref())
            }
            _ => self.rerank_runtime_model_config(workspace, choice.model_code.as_deref()),
        }
    }

    fn embedding_batch_input_limit(
        &self,
        workspace: &str,
        model: &str,
    ) -> Result<Option<usize>, ModelError> {
        if !model.starts_with(PERSON_AI_EMBEDDING_PREFIX) {
            return Ok(None);
        }
        use eliteforge_agent_capability_personai::{
            PersonAiEmbeddingKind, personai_embedding_batch_limit,
        };
        Ok(Some(personai_embedding_batch_limit(
            if self.embedding_accepts_images(workspace, model)? {
                PersonAiEmbeddingKind::Multimodal
            } else {
                PersonAiEmbeddingKind::Text
            },
        )))
    }
    fn embedding_accepts_images(&self, _: &str, model: &str) -> Result<bool, ModelError> {
        if let Some(models) = &self.configured_models {
            return Ok(models.automatic_route_catalog()?.into_iter().any(|record| {
                (record.model_code == model || record.binding_id == model)
                    && record
                        .input_modalities
                        .contains(&eliteforge_agent_core::Modality::Image)
            }));
        }
        self.providers.embedding_accepts_images(model)
    }

    fn auto_vision_runtime_model_config<'a>(
        &'a self,
        workspace: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<RuntimeModelConfig>, ModelError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let before = self
                .providers
                .sdk_model_authorization_scope()
                .map_err(|e| ModelError::RuntimeResolutionFailed(e.to_string()))?;
            let credentials = self
                .providers
                .active_credentials(ModelProviderCode::PersonAi)
                .await
                .map_err(runtime_resolution_error)?;
            let client = crab_serverapi::gateway::ChatCompletionsClient::new(
                credentials.server_base_url.clone(),
            )
            .map_err(|e| ModelError::RuntimeResolutionFailed(e.to_string()))?;
            let mut candidates = client
                .list_models(
                    &credentials.context(),
                    Some(QualityType::High),
                    Some(true),
                    None,
                )
                .await
                .map_err(|e| ModelError::RuntimeResolutionFailed(e.to_string()))?;
            candidates.retain(|model| model.image_understanding == Some(true));
            candidates.sort_by(|a, b| a.model_code.cmp(&b.model_code));
            let after = self
                .providers
                .sdk_model_authorization_scope()
                .map_err(|e| ModelError::RuntimeResolutionFailed(e.to_string()))?;
            if before != after {
                return Err(ModelError::RuntimeResolutionFailed(
                    "image description account changed during resolution".into(),
                ));
            }
            let Some(candidate) = candidates.first() else {
                return Ok(None);
            };
            let code = match candidate
                .provider_code
                .as_deref()
                .filter(|code| !code.is_empty())
            {
                Some(provider) => format!("{provider}/{}", candidate.model_code),
                None => candidate.model_code.clone(),
            };
            self.vision_runtime_model_config(
                workspace,
                &person_ai_fixed_runtime_code(ProviderModelQuality::High, &code),
            )
        })
    }
    fn vision_cache_identity(&self) -> Result<Option<String>, ModelError> {
        // Stable account identity across process restarts; never hash or persist credentials.
        self.providers.connection.call(
            move |connection| {
                let row = read_connection(&connection, ModelProviderCode::PersonAi)
                    .map_err(|_| ModelError::StorageFailure)?
                    .ok_or(ModelError::StorageFailure)?;
                let account = row
                    .person_ai_user_id
                    .filter(|id| !id.trim().is_empty())
                    .ok_or(ModelError::StorageFailure)?;
                let identity = serde_json::to_vec(&(
                    row.provider_code.storage_value(),
                    account,
                    row.server_base_url,
                ))
                .map_err(|_| ModelError::StorageFailure)?;
                use sha2::{Digest, Sha256};
                Ok(Some(format!("{:x}", Sha256::digest(identity))))
            },
            || ModelError::StorageFailure,
        )
    }

    fn vision_runtime_model_config(
        &self,
        workspace: &str,
        model: &str,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        // A concrete High-quality model makes description cache lineage stable. No silent Auto fallback.
        if !model.starts_with("PERSON_AI:FIXED:High:") {
            return Ok(None);
        }
        let code = model
            .strip_prefix("PERSON_AI:FIXED:High:")
            .expect("validated prefix");
        self.providers
            .resolve_chat_selection(
                workspace,
                Some(&ModelSelection::person_ai_fixed(
                    code,
                    ProviderModelQuality::High,
                )),
                None,
                None,
            )
            .map(|config| config.map(runtime_config_from_agent))
            .map_err(runtime_resolution_error)
    }

    fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        if let Some(models) = &self.configured_models {
            return models
                .route_auxiliary_model(
                    workspace_code,
                    crate::domain::model::AutoRouterKind::Embedding,
                    &eliteforge_agent_core::ModelChoice {
                        model_code: model_code.map(str::to_owned),
                        required_modalities: vec![eliteforge_agent_core::Modality::Text],
                        ..Default::default()
                    },
                    None,
                )
                .map(Some);
        }
        match self
            .providers
            .embedding_runtime_model_config(workspace_code, model_code)
            .map_err(runtime_resolution_error)?
        {
            Some(config) => Ok(Some(config)),
            None => match &self.configured_models {
                Some(models) => models.embedding_runtime_model_config(workspace_code, model_code),
                None => Ok(None),
            },
        }
    }

    fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        if let Some(models) = &self.configured_models {
            return models
                .route_auxiliary_model(
                    workspace_code,
                    crate::domain::model::AutoRouterKind::Rerank,
                    &eliteforge_agent_core::ModelChoice {
                        model_code: model_code.map(str::to_owned),
                        required_modalities: vec![eliteforge_agent_core::Modality::Text],
                        ..Default::default()
                    },
                    None,
                )
                .map(Some);
        }
        match self
            .providers
            .rerank_runtime_model_config(workspace_code, model_code)
            .map_err(runtime_resolution_error)?
        {
            Some(config) => Ok(Some(config)),
            None => match &self.configured_models {
                Some(models) => models.rerank_runtime_model_config(workspace_code, model_code),
                None => Ok(None),
            },
        }
    }
}

fn runtime_resolution_error(error: ModelProviderError) -> ModelError {
    match error {
        ModelProviderError::Storage => ModelError::StorageFailure,
        error => ModelError::RuntimeResolutionFailed(error.to_string()),
    }
}

pub(super) fn person_ai_agent_config(
    runtime_model_code: impl Into<String>,
    model_name: impl Into<String>,
    context_window_tokens: Option<u32>,
    reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    protocol: AgentModelProtocol,
) -> AgentModelConfig {
    AgentModelConfig {
        model_code: runtime_model_code.into(),
        protocol,
        base_url: "https://person-ai.invalid".into(),
        api_path: Some(
            match protocol {
                AgentModelProtocol::Responses => "responses",
                AgentModelProtocol::ChatCompletions => "chat/completions",
                AgentModelProtocol::Embeddings => "embeddings",
                AgentModelProtocol::Anthropic | AgentModelProtocol::Gemini => {
                    unreachable!("PersonAI exposes only OpenAI-compatible protocols")
                }
            }
            .into(),
        ),
        api_key: String::new(),
        request_headers: Default::default(),
        model_name: model_name.into(),
        thinking_enabled: reasoning_effort.is_some(),
        supports_developer_role: false,
        reasoning_effort,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens,
        top_p: None,
        max_retries: Some(0),
    }
}

pub(super) fn preferred_protocol(protocols: &[AgentModelProtocol]) -> AgentModelProtocol {
    if protocols.contains(&AgentModelProtocol::Responses) {
        AgentModelProtocol::Responses
    } else {
        AgentModelProtocol::ChatCompletions
    }
}

pub(super) fn person_ai_fixed_runtime_code(
    quality_type: ProviderModelQuality,
    model_code: &str,
) -> String {
    format!(
        "PERSON_AI:FIXED:{}:{model_code}",
        quality_type.storage_value(),
    )
}

pub(super) fn person_ai_runtime_quality(config: &AgentModelConfig) -> Option<ProviderModelQuality> {
    if config.model_code == "PERSON_AI:ROUTE:ChatAuto" {
        return Some(ProviderModelQuality::High);
    }
    let encoded = config.model_code.strip_prefix("PERSON_AI:FIXED:")?;
    let (quality, _) = encoded.split_once(':')?;
    ProviderModelQuality::from_storage(quality)
}

pub(super) fn require_high_chat_quality(
    quality: Option<ProviderModelQuality>,
) -> Result<(), ModelProviderError> {
    if quality == Some(ProviderModelQuality::High) {
        return Ok(());
    }
    Err(ModelProviderError::InvalidSelection(
        tron_i18n::Message::new("tron.model_provider.high_quality_chat_required"),
    ))
}

pub(super) fn runtime_config_from_agent(config: AgentModelConfig) -> RuntimeModelConfig {
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
        model_type: ModelType::Text,
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
