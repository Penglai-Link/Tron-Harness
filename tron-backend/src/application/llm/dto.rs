#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmTextGenerationCallCommand {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmTextGenerationCallResult {
    pub model_code: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmEmbeddingCallCommand {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmEmbeddingCallResult {
    pub model_code: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmChunkedEmbeddingCallCommand {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmChunkedEmbeddingCallResult {
    pub model_code: String,
    pub chunks: Vec<LlmEmbeddingChunkResultItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmEmbeddingChunkResultItem {
    pub chunk_index: usize,
    pub content: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRerankCallCommand {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub query: String,
    pub documents: Vec<String>,
    pub top_n: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRerankCallResult {
    pub model_code: String,
    pub items: Vec<LlmRerankResultItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRerankResultItem {
    pub document_index: usize,
    pub score: f64,
    pub document: String,
}
