use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelViewSourceKind {
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelProtocol {
    #[serde(rename = "OPENAI_COMPATIBLE")]
    OpenAiCompatible,
    #[serde(rename = "OPENAI_COMPATIBLE_EMBEDDING")]
    OpenAiCompatibleEmbedding,
    Responses,
    Anthropic,
    Gemini,
    Rerank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelType {
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
