use super::{ModelCapability, ModelError, ModelProtocol, ModelType};

pub fn validate_model_configuration(
    display_name: &str,
    vendor: &str,
    base_url: &str,
    api_key_ref: &str,
    model_name: &str,
    protocol: ModelProtocol,
    model_type: ModelType,
    model_capabilities: &[ModelCapability],
    responses_stateful_enabled: bool,
) -> Result<(), ModelError> {
    if display_name.trim().is_empty() {
        return Err(ModelError::DisplayNameRequired);
    }
    if vendor.trim().is_empty() {
        return Err(ModelError::VendorRequired);
    }
    if base_url.trim().is_empty() {
        return Err(ModelError::BaseUrlRequired);
    }
    if url::Url::parse(base_url.trim()).is_err() {
        return Err(ModelError::InvalidBaseUrl);
    }
    if api_key_ref.trim().is_empty() {
        return Err(ModelError::ApiKeyRefRequired);
    }
    if model_name.trim().is_empty() {
        return Err(ModelError::ModelNameRequired);
    }
    if !protocol_model_type_allowed(protocol, model_type) {
        return Err(ModelError::InvalidProtocolModelType);
    }
    if !model_capabilities
        .iter()
        .any(|value| model_capability_model_type(*value) == model_type)
    {
        return Err(ModelError::InvalidProtocolModelType);
    }
    if model_capabilities
        .iter()
        .any(|value| !protocol_model_type_allowed(protocol, model_capability_model_type(*value)))
    {
        return Err(ModelError::InvalidProtocolModelType);
    }
    if responses_stateful_enabled && protocol != ModelProtocol::Responses {
        return Err(ModelError::ResponsesStatefulModeUnsupported);
    }
    Ok(())
}

pub fn protocol_model_type_allowed(protocol: ModelProtocol, model_type: ModelType) -> bool {
    matches!(
        (protocol, model_type),
        (ModelProtocol::OpenAiCompatible, ModelType::Text)
            | (
                ModelProtocol::OpenAiCompatibleEmbedding,
                ModelType::Embedding
            )
            | (ModelProtocol::Responses, ModelType::Text)
            | (ModelProtocol::Anthropic, ModelType::Text)
            | (ModelProtocol::Gemini, ModelType::Text)
            | (ModelProtocol::Responses, ModelType::Image)
            | (ModelProtocol::Rerank, ModelType::Rerank)
    )
}

pub fn default_model_capabilities_for_type(model_type: ModelType) -> Vec<ModelCapability> {
    match model_type {
        ModelType::Text => vec![ModelCapability::TextGeneration],
        ModelType::Embedding => vec![ModelCapability::TextVector],
        ModelType::Rerank => vec![ModelCapability::Rerank],
        ModelType::Image => vec![ModelCapability::ImageGeneration],
    }
}

pub fn model_capability_model_type(value: ModelCapability) -> ModelType {
    match value {
        ModelCapability::MultimodalAll
        | ModelCapability::TextGeneration
        | ModelCapability::DeepThinking
        | ModelCapability::SupportsDeveloperRole
        | ModelCapability::SpeechRecognition
        | ModelCapability::SpeechSynthesis
        | ModelCapability::RealtimeMultimodal
        | ModelCapability::RealtimeSpeechSynthesis
        | ModelCapability::RealtimeSpeechRecognition
        | ModelCapability::RealtimeSpeechTranslation => ModelType::Text,
        ModelCapability::VisionUnderstanding
        | ModelCapability::ImageGeneration
        | ModelCapability::VideoGeneration
        | ModelCapability::ThreedGeneration => ModelType::Image,
        ModelCapability::MultimodalVector | ModelCapability::TextVector => ModelType::Embedding,
        ModelCapability::Rerank => ModelType::Rerank,
    }
}
