mod chat_completions;
#[cfg(test)]
mod chat_completions_tests;
#[cfg(test)]
mod chat_tool_observation_tests;
mod content;
mod embeddings;
#[cfg(test)]
mod externalized_tool_tests;
mod registry;
mod responses;
#[cfg(test)]
mod responses_stateful_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_call_id_tests;
mod usage;
#[cfg(test)]
mod usage_tests;

pub use chat_completions::{ChatCompletionsAdapter, chat_completions_request};
pub use embeddings::{EmbeddingsAdapter, embeddings_request};
pub use registry::{ModelProtocolAdapter, adapter_for_protocol};
pub use responses::{ResponsesAdapter, responses_request};
