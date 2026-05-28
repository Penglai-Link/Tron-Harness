mod chat_completions;
mod content;
mod embeddings;
mod registry;
mod responses;
#[cfg(test)]
mod tests;
mod usage;

pub use chat_completions::{ChatCompletionsAdapter, chat_completions_request};
pub use embeddings::{EmbeddingsAdapter, embeddings_request};
pub use registry::{ModelProtocolAdapter, adapter_for_protocol};
pub use responses::{ResponsesAdapter, responses_request};
