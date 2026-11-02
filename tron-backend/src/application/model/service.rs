use crate::domain::reasoning::ReasoningEffort;
use crate::{
    application::variable::VariableCipher,
    domain::model::{
        ConfiguredModelProvider, ConfiguredProviderError, ModelError, ModelProtocol, ModelType,
    },
};
use std::sync::{Arc, Mutex};
mod configured_providers;
pub use configured_providers::*;
mod automatic_routing;
mod provider_models;
pub use automatic_routing::{AutomaticRouteModel, AutomaticRoutingSettingsError};

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeModelConfig {
    pub request_headers: std::collections::BTreeMap<String, String>,
    pub model_code: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key: String,
    pub model_name: String,
    pub thinking_enabled: bool,
    pub supports_developer_role: bool,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub responses_stateful_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub max_retries: Option<u32>,
}

pub trait ModelRepository: Send {
    fn list_configured_providers(
        &mut self,
    ) -> Result<Vec<ConfiguredModelProvider>, ConfiguredProviderError>;
    fn save_configured_provider(
        &mut self,
        provider: &ConfiguredModelProvider,
        expected_updated_at: Option<i64>,
    ) -> Result<(), ConfiguredProviderError>;
    fn delete_configured_provider(
        &mut self,
        provider_id: &str,
    ) -> Result<(), ConfiguredProviderError>;
}

#[derive(Clone)]
pub struct ModelService {
    repository: Arc<Mutex<Box<dyn ModelRepository>>>,
    cipher: Arc<dyn VariableCipher>,
    providers: crate::application::model_provider::ModelProviderService,
}
impl ModelService {
    pub fn new(
        repository: impl ModelRepository + 'static,
        cipher: Arc<dyn VariableCipher>,
        providers: crate::application::model_provider::ModelProviderService,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            cipher,
            providers,
        }
    }
    fn reveal_api_key(&self, value: &str) -> Result<String, ModelError> {
        let Some(payload) = value.strip_prefix("enc:v1:") else {
            return Ok(value.to_owned());
        };
        let encrypted = serde_json::from_str(payload).map_err(|_| ModelError::StorageFailure)?;
        self.cipher
            .decrypt(&encrypted, b"model.apiKeyRef")
            .map_err(|_| ModelError::StorageFailure)
    }
}
