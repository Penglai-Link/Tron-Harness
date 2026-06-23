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
}

impl fmt::Display for LlmRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InputRequired => formatter.write_str("input is required"),
            Self::QueryRequired => formatter.write_str("query is required"),
            Self::DocumentsRequired => formatter.write_str("documents is required"),
            Self::TextModelUnavailable => formatter
                .write_str("text model is unavailable, or no default text model is configured"),
            Self::EmbeddingModelUnavailable => formatter.write_str(
                "embedding model is unavailable, or no default embedding model is configured",
            ),
            Self::RerankModelUnavailable => formatter
                .write_str("rerank model is unavailable, or no default rerank model is configured"),
            Self::ModelResolutionFailed(message)
            | Self::ProviderFailed(message)
            | Self::InvalidProviderResponse(message)
            | Self::InvalidChunkStrategy(message) => formatter.write_str(message),
        }
    }
}

impl Error for LlmRuntimeError {}
