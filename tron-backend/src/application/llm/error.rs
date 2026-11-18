use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmRuntimeError {
    WorkspaceRequired,
    InputRequired,
    QueryRequired,
    DocumentsRequired,
    TextModelUnavailable,
    EmbeddingModelUnavailable,
    RerankModelUnavailable,
    ModelResolutionFailed(String),
    ProviderFailed(String),
    InvalidProviderResponse(String),
    InvalidChunkStrategy(String),
}

impl LlmRuntimeError {
    pub fn model_resolution(message: impl Into<String>) -> Self {
        Self::ModelResolutionFailed(message.into())
    }

    pub fn provider_failed(message: impl Into<String>) -> Self {
        Self::ProviderFailed(message.into())
    }

    pub fn invalid_provider_response(message: impl Into<String>) -> Self {
        Self::InvalidProviderResponse(message.into())
    }

    pub fn invalid_chunk_strategy(message: impl Into<String>) -> Self {
        Self::InvalidChunkStrategy(message.into())
    }
    fn encoded(key: &'static str) -> String {
        crate::shared::i18n_message::MessageText::new(key).encode()
    }
    /// Locale-independent user-facing model-resolution failure.
    pub fn localized_model_resolution(key: &'static str) -> Self {
        Self::ModelResolutionFailed(Self::encoded(key))
    }
    /// Locale-independent user-facing provider failure.
    pub fn localized_provider_failed(key: &'static str) -> Self {
        Self::ProviderFailed(Self::encoded(key))
    }
    /// Locale-independent user-facing invalid-response failure.
    pub fn localized_invalid_provider_response(key: &'static str) -> Self {
        Self::InvalidProviderResponse(Self::encoded(key))
    }
    /// Locale-independent user-facing chunk-strategy failure.
    pub fn localized_invalid_chunk_strategy(key: &'static str) -> Self {
        Self::InvalidChunkStrategy(Self::encoded(key))
    }
    /// Locale-independent user-facing invalid response with arguments.
    pub fn localized_invalid_provider_response_message(
        message: crate::shared::i18n_message::MessageText,
    ) -> Self {
        Self::InvalidProviderResponse(message.encode())
    }
    fn payload(&self) -> Option<&str> {
        match self {
            Self::ModelResolutionFailed(message)
            | Self::ProviderFailed(message)
            | Self::InvalidProviderResponse(message)
            | Self::InvalidChunkStrategy(message) => Some(message),
            _ => None,
        }
    }
    fn fixed_key(&self) -> &'static str {
        match self {
            Self::WorkspaceRequired => "llm.workspace_required",
            Self::InputRequired => "llm.input_required",
            Self::QueryRequired => "llm.query_required",
            Self::DocumentsRequired => "llm.documents_required",
            Self::TextModelUnavailable => "llm.text_model_unavailable",
            Self::EmbeddingModelUnavailable => "llm.embedding_model_unavailable",
            Self::RerankModelUnavailable => "llm.rerank_model_unavailable",
            Self::ModelResolutionFailed(_) => "llm.model_resolution_failed",
            Self::ProviderFailed(_) => "llm.provider_failed",
            Self::InvalidProviderResponse(_) => "llm.invalid_provider_response",
            Self::InvalidChunkStrategy(_) => "llm.invalid_chunk_strategy",
        }
    }
    /// Display-layer conversion: structured payloads stay key + arguments;
    /// technical provider detail is wrapped into the matching message.
    pub fn localized_detail(&self) -> crate::shared::i18n_message::MessageText {
        use crate::shared::i18n_message::MessageText;
        if let Some(payload) = self.payload() {
            if let Some(message) = MessageText::decode(payload) {
                return message;
            }
            return MessageText::new(match self {
                Self::ModelResolutionFailed(_) => "llm.model_resolution_failed",
                Self::ProviderFailed(_) => "llm.provider_failed",
                Self::InvalidProviderResponse(_) => "llm.invalid_provider_response",
                Self::InvalidChunkStrategy(_) => "llm.invalid_chunk_strategy",
                _ => unreachable!(),
            })
            .arg("detail", payload);
        }
        MessageText::new(self.fixed_key())
    }
}

impl fmt::Display for LlmRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Diagnostics only; user-facing output renders `localized_detail(locale)`.
        match self.payload() {
            Some(message) => match crate::shared::i18n_message::MessageText::decode(message) {
                Some(decoded) => formatter.write_str(decoded.key()),
                None => formatter.write_str(message),
            },
            None => formatter.write_str(self.fixed_key()),
        }
    }
}

impl Error for LlmRuntimeError {}
