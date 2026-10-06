use super::*;

#[derive(Clone)]
pub struct ProviderAwareLlmRuntimeResolver {
    models: ModelService,
    providers: ModelProviderService,
}

impl ProviderAwareLlmRuntimeResolver {
    pub fn new(models: ModelService, providers: ModelProviderService) -> Self {
        Self { models, providers }
    }
}

impl LlmRuntimeModelResolver for ProviderAwareLlmRuntimeResolver {
    fn vision_cache_identity(&self) -> Result<Option<String>, ModelError> {
        // Stable account identity across process restarts; never hash or persist credentials.
        let connection = self.providers.connection.lock().map_err(|_| ModelError::StorageFailure)?;
        let row = read_connection(&connection, ModelProviderCode::PersonAi).map_err(|_| ModelError::StorageFailure)?.ok_or(ModelError::StorageFailure)?;
        let account = row.person_ai_user_id.filter(|id| !id.trim().is_empty()).ok_or(ModelError::StorageFailure)?;
        let identity = serde_json::to_vec(&(row.provider_code.storage_value(), account, row.server_base_url)).map_err(|_| ModelError::StorageFailure)?;
        use sha2::{Digest, Sha256};
        Ok(Some(format!("{:x}", Sha256::digest(identity))))
    }

    fn vision_runtime_model_config(&self, workspace: &str, model: &str) -> Result<Option<RuntimeModelConfig>, ModelError> {
        // A concrete High-quality model makes description cache lineage stable. No silent Auto fallback.
        if !model.starts_with("PERSON_AI:FIXED:High:") { return Ok(None); }
        self.providers.text_runtime_model_config(workspace, Some(model)).map_err(|_| ModelError::StorageFailure)
    }

    fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        match self
            .providers
            .embedding_runtime_model_config(workspace_code, model_code)
        {
            Ok(Some(config)) => Ok(Some(config)),
            Ok(None) => self
                .models
                .embedding_runtime_model_config(workspace_code, model_code),
            Err(_) => Err(ModelError::StorageFailure),
        }
    }

    fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        match self
            .providers
            .rerank_runtime_model_config(workspace_code, model_code)
        {
            Ok(Some(config)) => Ok(Some(config)),
            Ok(None) => self
                .models
                .rerank_runtime_model_config(workspace_code, model_code),
            Err(_) => Err(ModelError::StorageFailure),
        }
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
            }
            .into(),
        ),
        api_key: String::new(),
        model_name: model_name.into(),
        thinking_enabled: reasoning_effort.is_some(),
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

pub(super) fn runtime_config_from_agent(config: AgentModelConfig) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: config.model_code,
        protocol: match config.protocol {
            AgentModelProtocol::Responses => ModelProtocol::Responses,
            AgentModelProtocol::ChatCompletions => ModelProtocol::OpenAiCompatible,
            AgentModelProtocol::Embeddings => ModelProtocol::OpenAiCompatibleEmbedding,
        },
        model_type: ModelType::Text,
        base_url: config.base_url,
        api_path: config.api_path,
        api_key: config.api_key,
        model_name: config.model_name,
        thinking_enabled: config.thinking_enabled,
        reasoning_effort: config.reasoning_effort,
        responses_stateful_enabled: config.responses_stateful_enabled,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        context_window_tokens: config.context_window_tokens,
        top_p: config.top_p,
        max_retries: config.max_retries,
    }
}
