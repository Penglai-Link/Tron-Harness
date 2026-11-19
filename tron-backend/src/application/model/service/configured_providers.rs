use std::collections::{BTreeMap, HashSet};

use super::*;
use crate::domain::model::{
    ConfiguredModelProvider, ConfiguredProviderError, ConfiguredProviderModel,
    ConfiguredProviderModelType, ConfiguredProviderProtocol, ModelInputCapability,
};
use crate::domain::reasoning::ReasoningEffort;
use url::Url;

#[derive(Clone)]
pub struct ConfiguredProviderCommand {
    pub provider_type: Option<String>,
    pub display_name: String,
    pub protocol: ConfiguredProviderProtocol,
    pub base_url: String,
    pub api_key: Option<String>,
    pub request_headers: BTreeMap<String, String>,
    pub enabled: bool,
    pub default_reasoning_efforts: Vec<ReasoningEffort>,
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
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .list_configured_providers()
    }

    pub fn get_configured_provider(
        &self,
        provider_id: &str,
    ) -> Result<ConfiguredModelProvider, ConfiguredProviderError> {
        let mut provider = self
            .list_configured_providers()?
            .into_iter()
            .find(|provider| provider.provider_id == provider_id)
            .ok_or(ConfiguredProviderError::NotFound)?;
        provider.api_key_summary = self.saved_key_summary(&provider);
        Ok(provider)
    }

    fn saved_key_summary(&self, provider: &ConfiguredModelProvider) -> Option<String> {
        if provider.encrypted_api_key.is_empty() {
            return None;
        }
        provider.api_key_summary.clone().or_else(|| {
            self.reveal_api_key(&provider.encrypted_api_key)
                .ok()
                .map(|key| api_key_summary(&key))
        })
    }

    pub fn create_configured_provider(
        &self,
        command: ConfiguredProviderCommand,
    ) -> Result<ConfiguredModelProvider, ConfiguredProviderError> {
        let provider_id = format!("provider-{:032x}", rand::random::<u128>());
        self.save_configured_provider(&provider_id, command, true)
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
        for model in &command.models {
            if model.logical_model_code.as_ref().is_some_and(|code| {
                code.trim().is_empty()
                    || code.len() > 256
                    || code.chars().any(char::is_control)
                    || code.starts_with("MODEL-")
                    || code.starts_with("AUTO:")
                    || code.starts_with("PERSON_AI:")
            }) {
                return Err(ConfiguredProviderError::InvalidModels);
            }
        }
        command.display_name = command.display_name.trim().to_owned();
        command.base_url = command.base_url.trim().trim_end_matches('/').to_owned();
        let mut repository = self
            .repository
            .lock()
            .expect("model repository mutex poisoned");
        let existing = repository.list_configured_providers()?;
        for model in &command.models {
            let Some(code) = model.logical_model_code.as_deref().map(str::trim) else {
                continue;
            };
            for other in existing
                .iter()
                .filter(|provider| provider.provider_id != provider_id)
                .flat_map(|provider| &provider.models)
                .chain(command.models.iter())
            {
                if other.logical_model_code.as_deref().map(str::trim) == Some(code) {
                    let family =
                        |model: &ConfiguredProviderModel| match model.model_type.model_type() {
                            ModelType::TextEmbedding | ModelType::MultimodalEmbedding => 1,
                            ModelType::TextRerank | ModelType::MultimodalRerank => 2,
                            _ => 3,
                        };
                    if family(model) != family(other)
                        || (family(model) == 1 && model.dimension != other.dimension)
                    {
                        return Err(ConfiguredProviderError::InvalidModels);
                    }
                }
            }
        }
        let previous = existing
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
        let now = chrono::Utc::now().timestamp_millis();
        let updated_at = previous
            .as_ref()
            .map_or(now, |provider| now.max(provider.updated_at + 1));
        let api_key_summary = command
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(api_key_summary)
            .or_else(|| {
                previous
                    .as_ref()
                    .and_then(|provider| self.saved_key_summary(provider))
            });
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
        for config in &mut command.models {
            config.logical_model_code = config
                .logical_model_code
                .as_ref()
                .map(|code| code.trim().to_owned())
                .filter(|code| !code.is_empty());
            config.model_id = config.model_id.trim().to_owned();
            config.name = config.name.trim().to_owned();
            config.supported_reasoning_efforts =
                ReasoningEffort::normalize(&config.supported_reasoning_efforts);
            if config.model_code.is_none() {
                config.model_code = Some(format!("MODEL-{:032x}", rand::random::<u128>()));
            }
        }
        let provider = ConfiguredModelProvider {
            provider_id: provider_id.to_owned(),
            provider_type: command
                .provider_type
                .or_else(|| previous.as_ref().and_then(|p| p.provider_type.clone())),
            display_name: command.display_name,
            protocol: command.protocol,
            base_url: command.base_url,
            encrypted_api_key,
            api_key_summary,
            request_headers: command.request_headers,
            enabled: command.enabled,
            default_reasoning_efforts: ReasoningEffort::normalize(
                &command.default_reasoning_efforts,
            ),
            models: command.models,
            created_at: previous
                .as_ref()
                .map_or(now, |provider| provider.created_at),
            updated_at,
        };
        repository.save_configured_provider(
            &provider,
            previous.as_ref().map(|provider| provider.updated_at),
        )?;
        Ok(provider)
    }
}

fn api_key_summary(key: &str) -> String {
    let chars: Vec<_> = key.chars().collect();
    let (prefix, suffix) = match chars.len() {
        0..=4 => return "***".into(),
        5..=8 => (1, 1),
        9..=15 => (2, 2),
        16..=25 => (3, 3),
        _ => (6, 7),
    };
    format!(
        "{}***{}",
        chars[..prefix].iter().collect::<String>(),
        chars[chars.len() - suffix..].iter().collect::<String>()
    )
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
    let provider_default_efforts = ReasoningEffort::normalize(&command.default_reasoning_efforts);
    for model in &command.models {
        if model.model_type.protocol(command.protocol).is_none()
            || (model.model_type != crate::domain::model::ConfiguredProviderModelType::Text
                && (model.reasoning
                    || model.supports_developer_role
                    || (!model.model_type.is_media()
                        && !model.model_type.is_multimodal()
                        && model.input_capabilities != [ModelInputCapability::Text])))
        {
            return Err(ConfiguredProviderError::InvalidModels);
        }
        let supported_efforts = ReasoningEffort::normalize(&model.supported_reasoning_efforts);
        let is_reasoning_chat =
            model.model_type == ConfiguredProviderModelType::Text && model.reasoning;
        if !is_reasoning_chat {
            if !supported_efforts.is_empty() || model.default_reasoning_effort.is_some() {
                return Err(ConfiguredProviderError::InvalidModels);
            }
        } else if let Some(default) = model.default_reasoning_effort {
            let effective = if supported_efforts.is_empty() {
                &provider_default_efforts
            } else {
                &supported_efforts
            };
            if !effective.contains(&default) {
                return Err(ConfiguredProviderError::InvalidModels);
            }
        }
        if model.model_type.is_multimodal()
            && !model
                .input_capabilities
                .iter()
                .any(|c| *c != ModelInputCapability::Text)
        {
            return Err(ConfiguredProviderError::InvalidModels);
        }
        if model.model_id.trim().is_empty()
            || model.model_id.len() > 256
            || model.name.trim().is_empty()
            || model.name.len() > 200
            || !ids.insert(model.model_id.trim())
            || model
                .model_code
                .as_deref()
                .is_some_and(|code| !codes.insert(code))
            || model.dimension == Some(0)
            || (!model.model_type.is_media() && model.context_window_tokens.is_none())
            || model.context_window_tokens == Some(0)
            || model.max_output_tokens == Some(0)
            || matches!((model.context_window_tokens, model.max_output_tokens), (Some(context), Some(output)) if output > context)
            || model.input_capabilities.is_empty()
            || (!model.model_type.is_media()
                && !model.model_type.is_multimodal()
                && !model
                    .input_capabilities
                    .contains(&ModelInputCapability::Text))
            || (!model.model_type.is_media()
                && !model.model_type.is_multimodal()
                && model.input_capabilities.iter().any(|cap| {
                    matches!(
                        cap,
                        ModelInputCapability::Audio | ModelInputCapability::Video
                    )
                }))
        {
            return Err(ConfiguredProviderError::InvalidModels);
        }
    }
    Ok(())
}
