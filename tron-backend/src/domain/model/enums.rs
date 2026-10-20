use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelScopeKind {
    Platform,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelSourceKind {
    Platform,
    WorkspaceLocal,
    WorkspaceOverride,
    WorkspaceStatusPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelViewSourceKind {
    Platform,
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelProtocol {
    #[serde(rename = "OPENAI_COMPATIBLE")]
    OpenAiCompatible,
    #[serde(rename = "OPENAI_COMPATIBLE_EMBEDDING")]
    OpenAiCompatibleEmbedding,
    Responses,
    Rerank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelType {
    Text,
    Embedding,
    Rerank,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelCapability {
    MultimodalAll,
    TextGeneration,
    DeepThinking,
    SupportsDeveloperRole,
    VisionUnderstanding,
    ImageGeneration,
    VideoGeneration,
    ThreedGeneration,
    SpeechRecognition,
    SpeechSynthesis,
    MultimodalVector,
    TextVector,
    RealtimeMultimodal,
    RealtimeSpeechSynthesis,
    RealtimeSpeechRecognition,
    RealtimeSpeechTranslation,
    Rerank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelVerifyStatus {
    Succeeded,
    Failed,
}
