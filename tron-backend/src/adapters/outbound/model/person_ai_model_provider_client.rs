use super::HttpModelProviderClient;
use crate::{application::model_provider::ModelProviderService, domain::agent::AgentModelConfig};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_model_http as sdk;
use std::sync::Arc;

#[derive(Clone)]
pub struct PersonAiModelProviderClient {
    fallback: HttpModelProviderClient,
    providers: ModelProviderService,
}
impl PersonAiModelProviderClient {
    pub fn new(fallback: HttpModelProviderClient, providers: ModelProviderService) -> Self {
        Self {
            fallback,
            providers,
        }
    }
}
#[async_trait]
impl sdk::ModelTransportFactory for PersonAiModelProviderClient {
    fn authorization_scope(
        &self,
        config: &sdk::AgentModelConfig,
    ) -> Result<Option<String>, fw::ModelError> {
        if self.providers.is_person_ai_config(&config.model_code) {
            self.providers.sdk_model_authorization_scope().map(Some)
        } else {
            self.fallback.authorization_scope(config)
        }
    }
    fn on_failure(&self, config: &sdk::AgentModelConfig, error: &fw::ModelError) {
        if self.providers.is_person_ai_config(&config.model_code) {
            self.providers.sdk_final_model_failure(error);
        }
    }
    async fn create(
        &self,
        config: &sdk::AgentModelConfig,
    ) -> Result<Arc<dyn sdk::ModelTransport>, fw::ModelError> {
        if self.providers.is_person_ai_config(&config.model_code) {
            self.providers
                .sdk_model_transport(&crate::domain::agent::AgentModelConfig::from(config))
                .await
        } else {
            self.fallback.create(config).await
        }
    }
    async fn recover_authentication(
        &self,
        config: &sdk::AgentModelConfig,
        error: &fw::ModelError,
    ) -> Result<bool, fw::ModelError> {
        if self.providers.is_person_ai_config(&config.model_code) {
            self.providers.sdk_recover_model_authentication(error).await
        } else {
            Ok(false)
        }
    }
}

const PERSON_AI_CONVERSATION_PATH_PREFIX: &str = "person-ai/conversations/";
pub fn with_person_ai_conversation(
    mut config: AgentModelConfig,
    session_code: &str,
    run_code: Option<&str>,
) -> AgentModelConfig {
    if config.model_code.starts_with("PERSON_AI:") {
        config.api_path = Some(
            match run_code.map(str::trim).filter(|value| !value.is_empty()) {
                Some(run_code) => {
                    format!("{PERSON_AI_CONVERSATION_PATH_PREFIX}{session_code}?runCode={run_code}")
                }
                None => format!("{PERSON_AI_CONVERSATION_PATH_PREFIX}{session_code}"),
            },
        );
    }
    config
}
pub(crate) fn person_ai_runtime_metadata(
    config: &AgentModelConfig,
) -> (Option<&str>, Option<&str>) {
    let Some(path) = config
        .api_path
        .as_deref()
        .and_then(|path| path.strip_prefix(PERSON_AI_CONVERSATION_PATH_PREFIX))
    else {
        return (None, None);
    };
    let (conversation, query) = path.split_once('?').unwrap_or((path, ""));
    (
        (!conversation.trim().is_empty()).then_some(conversation),
        query
            .split('&')
            .find_map(|entry| entry.strip_prefix("runCode=")),
    )
}
