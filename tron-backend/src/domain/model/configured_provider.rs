use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ModelProtocol, ModelType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfiguredProviderProtocol {
    OpenAiChatCompletions,
    OpenAiResponses,
    AnthropicMessages,
    GoogleGemini,
}

impl ConfiguredProviderProtocol {
    pub const fn model_protocol(self) -> ModelProtocol {
        match self {
            Self::OpenAiChatCompletions => ModelProtocol::OpenAiCompatible,
            Self::OpenAiResponses => ModelProtocol::Responses,
            Self::AnthropicMessages => ModelProtocol::Anthropic,
            Self::GoogleGemini => ModelProtocol::Gemini,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelInputCapability {
    Text,
    Image,
    Audio,
    Video,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfiguredProviderModelType {
    #[default]
    Text,
    TextEmbedding,
    MultimodalEmbedding,
    TextRerank,
    MultimodalRerank,
    Image,
    Video,
    SpeechSynthesis,
    SpeechRecognition,
    RealtimeSpeech,
}
impl ConfiguredProviderModelType {
    pub const fn model_type(self) -> ModelType {
        match self {
            Self::Text => ModelType::Text,
            Self::TextEmbedding => ModelType::TextEmbedding,
            Self::MultimodalEmbedding => ModelType::MultimodalEmbedding,
            Self::TextRerank => ModelType::TextRerank,
            Self::MultimodalRerank => ModelType::MultimodalRerank,
            Self::Image => ModelType::Image,
            Self::Video => ModelType::Video,
            Self::SpeechSynthesis => ModelType::SpeechSynthesis,
            Self::SpeechRecognition => ModelType::SpeechRecognition,
            Self::RealtimeSpeech => ModelType::RealtimeSpeech,
        }
    }
    pub const fn is_multimodal(self) -> bool {
        matches!(self, Self::MultimodalEmbedding | Self::MultimodalRerank)
    }
    pub const fn is_media(self) -> bool {
        matches!(
            self,
            Self::Image
                | Self::Video
                | Self::SpeechSynthesis
                | Self::SpeechRecognition
                | Self::RealtimeSpeech
        )
    }
    pub fn protocol(self, provider: ConfiguredProviderProtocol) -> Option<ModelProtocol> {
        match self {
            Self::Text => Some(provider.model_protocol()),
            Self::TextEmbedding | Self::MultimodalEmbedding
                if matches!(
                    provider,
                    ConfiguredProviderProtocol::OpenAiChatCompletions
                        | ConfiguredProviderProtocol::OpenAiResponses
                ) =>
            {
                Some(ModelProtocol::OpenAiCompatibleEmbedding)
            }
            Self::TextRerank | Self::MultimodalRerank
                if matches!(
                    provider,
                    ConfiguredProviderProtocol::OpenAiChatCompletions
                        | ConfiguredProviderProtocol::OpenAiResponses
                ) =>
            {
                Some(ModelProtocol::Rerank)
            }
            Self::Image
            | Self::Video
            | Self::SpeechSynthesis
            | Self::SpeechRecognition
            | Self::RealtimeSpeech
                if !matches!(provider, ConfiguredProviderProtocol::AnthropicMessages) =>
            {
                Some(provider.model_protocol())
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfiguredProviderModel {
    #[serde(default)]
    pub model_type: ConfiguredProviderModelType,
    pub model_code: Option<String>,
    pub model_id: String,
    pub name: String,
    #[serde(default)]
    pub dimension: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub reasoning: bool,
    pub supports_developer_role: bool,
    pub input_capabilities: Vec<ModelInputCapability>,
    pub enabled: bool,
}

/// Product-owned connection configuration. Credentials are encrypted before storage
/// and never become part of a public provider or Framework catalog descriptor.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelProvider {
    pub provider_id: String,
    #[serde(default)]
    pub provider_type: Option<String>,
    pub display_name: String,
    pub protocol: ConfiguredProviderProtocol,
    pub base_url: String,
    pub encrypted_api_key: String,
    #[serde(default)]
    pub api_key_summary: Option<String>,
    pub request_headers: BTreeMap<String, String>,
    pub enabled: bool,
    pub models: Vec<ConfiguredProviderModel>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfiguredProviderError {
    NotFound,
    AlreadyExists,
    InvalidId,
    InvalidName,
    InvalidUrl,
    InvalidHeaders,
    InvalidModels,
    InvalidModelOwnership,
    Conflict,
    Storage,
}

impl std::fmt::Display for ConfiguredProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotFound => "configured provider not found",
            Self::AlreadyExists => "provider ID already exists",
            Self::InvalidId => "invalid provider ID",
            Self::InvalidName => "invalid provider name",
            Self::InvalidUrl => "invalid provider URL",
            Self::InvalidHeaders => "invalid or reserved request headers",
            Self::InvalidModels => "invalid provider models",
            Self::InvalidModelOwnership => "model does not belong to this provider",
            Self::Conflict => "provider configuration changed; reload before saving",
            Self::Storage => "provider storage failed",
        })
    }
}

impl std::error::Error for ConfiguredProviderError {}
