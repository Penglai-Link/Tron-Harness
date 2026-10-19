mod dto;
mod error;
mod port;
mod service;
mod vision;
pub use vision::VisionModelBinding;
#[cfg(test)]
mod service_tests;

pub use dto::{
    LlmEmbeddingContentBatchCallCommand,
    LlmChunkedEmbeddingCallCommand, LlmChunkedEmbeddingCallResult, LlmEmbeddingBatchCallCommand,
    LlmEmbeddingBatchCallResult, LlmEmbeddingCallCommand, LlmEmbeddingCallResult,
    LlmEmbeddingChunkResultItem, LlmRerankCallCommand, LlmRerankCallResult, LlmRerankResultItem,
    LlmTextGenerationCallResult,
};
pub use error::LlmRuntimeError;
pub use port::{
    CharLengthChunkStrategy, LlmEmbeddingChunkStrategy, LlmRuntimeClient, LlmRuntimeModelResolver,
    LlmRuntimeRerankHit,
};
pub use service::LlmRuntimeService;

pub(crate) use service::ContextQueryAllowance;
