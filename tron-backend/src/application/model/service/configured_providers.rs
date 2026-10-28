use std::collections::{BTreeMap, HashMap, HashSet};

use super::*;
use crate::domain::model::{
    ConfiguredModelProvider, ConfiguredProviderError, ConfiguredProviderModel,
    ConfiguredProviderProtocol, ModelInputCapability,
};

#[derive(Clone)]
pub struct ConfiguredProviderCommand {
    pub display_name: String,
    pub protocol: ConfiguredProviderProtocol,
    pub base_url: String,
    pub api_key: Option<String>,
    pub request_headers: BTreeMap<String, String>,
    pub enabled: bool,
    pub models: Vec<ConfiguredProviderModel>,
}

pub struct ApplicationErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}
pub fn configured_provider_preset_not_found_descriptor() -> ApplicationErrorDescriptor {
    ApplicationErrorDescriptor {
        code: "101032011",
        message_key: "tron.model_provider.preset_not_found",
    }
}
impl ConfiguredProviderError {
    pub fn descriptor(&self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::NotFound => ("101032001", "model_provider.not_found"),
            Self::AlreadyExists => ("101032002", "model_provider.already_exists"),
            Self::InvalidId => ("101032003", "model_provider.invalid_id"),
            Self::InvalidName => ("101032004", "model_provider.invalid_name"),
            Self::InvalidUrl => ("101032005", "model_provider.invalid_url"),
            Self::InvalidHeaders => ("101032006", "model_provider.invalid_headers"),
            Self::InvalidModels => ("101032007", "model_provider.invalid_models"),
            Self::InvalidModelOwnership => ("101032008", "model_provider.invalid_model_ownership"),
            Self::Conflict => ("101032009", "model_provider.conflict"),
            Self::Storage => ("101032010", "model_provider.storage_failed"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}

impl ModelService {
    pub fn list_configured_providers(
        &self,
    ) -> Result<Vec<ConfiguredModelProvider>, ConfiguredProviderError> {
        let mut repository = self
            .repository
            .lock()
            .expect("model repository mutex poisoned");
        let (models, mut providers) = repository
            .model_configuration_snapshot()
            .map_err(|_| ConfiguredProviderError::Storage)?;
        let statuses: HashMap<_, _> = models
            .iter()
            .map(|model| (model.model_code.as_str(), model.enabled))
            .collect();
        for provider in &mut providers {
            for model in &mut provider.models {
                if let Some(enabled) = model
                    .model_code
                    .as_deref()
                    .and_then(|code| statuses.get(code))
                {
                    model.enabled = *enabled;
                }
            }
        }
        Ok(providers)
    }

    pub fn get_configured_provider(
        &self,
        provider_id: &str,
    ) -> Result<ConfiguredModelProvider, ConfiguredProviderError> {
        self.list_configured_providers()?
            .into_iter()
            .find(|provider| provider.provider_id == provider_id)
            .ok_or(ConfiguredProviderError::NotFound)
    }

    pub fn create_configured_provider(
        &self,
        provider_id: &str,
        command: ConfiguredProviderCommand,
    ) -> Result<ConfiguredModelProvider, ConfiguredProviderError> {
        self.save_configured_provider(provider_id, command, true)
    }

    pub fn update_configured_provider(
        &self,
        provider_id: &str,
        command: ConfiguredProviderCommand,
    ) -> Result<ConfiguredModelProvider, ConfiguredProviderError> {
        self.save_configured_provider(provider_id, command, false)
    }

    pub fn delete_configured_provider(
        &self,
        provider_id: &str,
    ) -> Result<(), ConfiguredProviderError> {
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .delete_configured_provider(provider_id)
    }

    fn save_configured_provider(
        &self,
        provider_id: &str,
        mut command: ConfiguredProviderCommand,
        create: bool,
    ) -> Result<ConfiguredModelProvider, ConfiguredProviderError> {
        validate_provider(provider_id, &command)?;
        command.display_name = command.display_name.trim().to_owned();
        command.base_url = command.base_url.trim().trim_end_matches('/').to_owned();
        let mut repository = self
            .repository
            .lock()
            .expect("model repository mutex poisoned");
        let previous = repository
            .list_configured_providers()?
            .into_iter()
            .find(|provider| provider.provider_id == provider_id);
        if create && previous.is_some() {
            return Err(ConfiguredProviderError::AlreadyExists);
        }
        if !create && previous.is_none() {
            return Err(ConfiguredProviderError::NotFound);
        }
        let old_codes: HashSet<_> = previous
            .iter()
            .flat_map(|provider| provider.models.iter())
            .filter_map(|model| model.model_code.as_deref())
            .collect();
        if command
            .models
            .iter()
            .filter_map(|model| model.model_code.as_deref())
            .any(|code| !old_codes.contains(code))
        {
            return Err(ConfiguredProviderError::InvalidModelOwnership);
        }
        let existing_models = repository
            .list_models()
            .map_err(|_| ConfiguredProviderError::Storage)?;
        let existing: HashMap<_, _> = existing_models
            .iter()
            .map(|model| (model.model_code.as_str(), model))
            .collect();
        let mut next_id = repository
            .next_id()
            .map_err(|_| ConfiguredProviderError::Storage)?;
        let now = chrono::Utc::now().timestamp_millis();
        let updated_at = previous
            .as_ref()
            .map_or(now, |provider| now.max(provider.updated_at + 1));
        let encrypted_api_key = match command
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
        {
            Some(key) => {
                let encrypted = self
                    .cipher
                    .encrypt(key, b"model.apiKeyRef")
                    .map_err(|_| ConfiguredProviderError::Storage)?;
                format!(
                    "enc:v1:{}",
                    serde_json::to_string(&encrypted)
                        .map_err(|_| ConfiguredProviderError::Storage)?
                )
            }
            None => previous
                .as_ref()
                .map(|provider| provider.encrypted_api_key.clone())
                .unwrap_or_default(),
        };
        let mut models = Vec::with_capacity(command.models.len());
        for config in &mut command.models {
            config.model_id = config.model_id.trim().to_owned();
            config.name = config.name.trim().to_owned();
            let old = config
                .model_code
                .as_deref()
                .and_then(|code| existing.get(code))
                .copied();
            let id = old.map_or_else(
                || {
                    let id = next_id;
                    next_id += 1;
                    id
                },
                |model| model.id.value(),
            );
            let model_code =
                old.map_or_else(|| build_model_code(id), |model| model.model_code.clone());
            config.model_code = Some(model_code.clone());
            let mut capabilities = vec![ModelCapability::TextGeneration];
            if config.reasoning {
                capabilities.push(ModelCapability::DeepThinking);
            }
            if config.supports_developer_role {
                capabilities.push(ModelCapability::SupportsDeveloperRole);
            }
            if config
                .input_capabilities
                .contains(&ModelInputCapability::Image)
            {
                capabilities.push(ModelCapability::VisionUnderstanding);
            }
            models.push(Model {
                provider_id: None,
                request_headers: Default::default(),
                id: SnowflakeId::new(id),
                scope_kind: ModelScopeKind::Platform,
                workspace_code: None,
                model_code,
                display_name: format!("{} · {}/{}", config.name, provider_id, config.model_id),
                vendor: command.display_name.clone(),
                protocol: command.protocol.model_protocol(),
                model_type: ModelType::Text,
                model_capabilities: capabilities,
                // Only the provider owns the connection and its encrypted credential.
                base_url: String::new(),
                api_path: None,
                api_key_ref: String::new(),
                model_name: config.model_id.clone(),
                enabled: config.enabled,
                description: None,
                thinking_enabled: config.reasoning,
                supported_reasoning_efforts: Vec::new(),
                default_reasoning_effort: None,
                structured_output_enabled: false,
                responses_stateful_enabled: false,
                temperature: None,
                max_tokens: config.max_output_tokens,
                context_window_tokens: config.context_window_tokens,
                top_p: None,
                max_retries: None,
                last_verified_at: None,
                last_verify_status: None,
                last_verify_message: None,
                source_kind: ModelSourceKind::Platform,
                source_model_code: None,
                allow_inheritance: None,
                created_at: old
                    .map(|model| model.created_at.clone())
                    .unwrap_or_else(current_timestamp),
                updated_at: current_timestamp(),
            });
        }
        let provider = ConfiguredModelProvider {
            provider_id: provider_id.to_owned(),
            display_name: command.display_name,
            protocol: command.protocol,
            base_url: command.base_url,
            encrypted_api_key,
            request_headers: command.request_headers,
            enabled: command.enabled,
            models: command.models,
            created_at: previous
                .as_ref()
                .map_or(now, |provider| provider.created_at),
            updated_at,
        };
        repository.save_configured_provider(
            &provider,
            previous.as_ref().map(|provider| provider.updated_at),
            &models,
        )?;
        Ok(provider)
    }

    pub(super) fn configured_headers(
        &self,
    ) -> Result<HashMap<String, BTreeMap<String, String>>, ModelError> {
        Ok(self
            .repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_configured_providers()
            .map_err(|_| ModelError::StorageFailure)?
            .into_iter()
            .flat_map(|provider| {
                provider.models.into_iter().filter_map(move |model| {
                    model
                        .model_code
                        .map(|code| (code, provider.request_headers.clone()))
                })
            })
            .collect())
    }

    pub(super) fn ensure_independent_model(&self, model_code: &str) -> Result<(), ModelError> {
        if self.configured_headers()?.contains_key(model_code) {
            return Err(ModelError::PlatformReferenceMutationDenied);
        }
        Ok(())
    }
}

fn validate_provider(
    provider_id: &str,
    command: &ConfiguredProviderCommand,
) -> Result<(), ConfiguredProviderError> {
    if provider_id.is_empty()
        || provider_id.len() > 80
        || !provider_id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        || ["configured", "presets", "personai"].contains(&provider_id)
    {
        return Err(ConfiguredProviderError::InvalidId);
    }
    if command.display_name.trim().is_empty() || command.display_name.len() > 200 {
        return Err(ConfiguredProviderError::InvalidName);
    }
    let url =
        Url::parse(command.base_url.trim()).map_err(|_| ConfiguredProviderError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfiguredProviderError::InvalidUrl);
    }
    let mut header_names = HashSet::new();
    if command.request_headers.len() > 32
        || command.request_headers.iter().any(|(name, value)| {
            let normalized = name.to_ascii_lowercase();
            !header_names.insert(normalized.clone())
                || name.is_empty()
                || reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err()
                || value.parse::<reqwest::header::HeaderValue>().is_err()
                || value.len() > 4096
                || [
                    "authorization",
                    "proxy-authorization",
                    "x-api-key",
                    "api-key",
                    "x-goog-api-key",
                    "host",
                    "content-length",
                    "transfer-encoding",
                    "connection",
                    "cookie",
                    "set-cookie",
                ]
                .contains(&normalized.as_str())
        })
    {
        return Err(ConfiguredProviderError::InvalidHeaders);
    }
    if command
        .api_key
        .as_ref()
        .is_some_and(|key| key.len() > 16384 || key.contains(['\r', '\n']))
    {
        return Err(ConfiguredProviderError::InvalidHeaders);
    }
    if command.models.is_empty() || command.models.len() > 500 {
        return Err(ConfiguredProviderError::InvalidModels);
    }
    let mut ids = HashSet::new();
    let mut codes = HashSet::new();
    for model in &command.models {
        if model.model_id.trim().is_empty()
            || model.model_id.len() > 256
            || model.name.trim().is_empty()
            || model.name.len() > 200
            || !ids.insert(model.model_id.trim())
            || model
                .model_code
                .as_deref()
                .is_some_and(|code| !codes.insert(code))
            || model.context_window_tokens == Some(0)
            || model.max_output_tokens == Some(0)
            || matches!((model.context_window_tokens, model.max_output_tokens), (Some(context), Some(output)) if output > context)
            || !model
                .input_capabilities
                .contains(&ModelInputCapability::Text)
            || model.input_capabilities.iter().any(|cap| {
                matches!(
                    cap,
                    ModelInputCapability::Audio | ModelInputCapability::Video
                )
            })
        {
            return Err(ConfiguredProviderError::InvalidModels);
        }
    }
    Ok(())
}
