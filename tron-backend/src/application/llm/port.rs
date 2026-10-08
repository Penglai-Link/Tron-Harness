use std::{future::Future, pin::Pin};

use crate::{application::model::RuntimeModelConfig, domain::model::ModelError};

pub trait LlmEmbeddingChunkStrategy: Send + Sync {
    fn split(&self, input: &str) -> Result<Vec<String>, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLengthChunkStrategy {
    pub max_chars_per_chunk: usize,
}

impl LlmEmbeddingChunkStrategy for CharLengthChunkStrategy {
    fn split(&self, input: &str) -> Result<Vec<String>, String> {
        if self.max_chars_per_chunk == 0 {
            return Err("max_chars_per_chunk must be greater than zero".into());
        }
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut current_chars = 0usize;
        for character in input.chars() {
            if current_chars >= self.max_chars_per_chunk {
                chunks.push(std::mem::take(&mut current));
                current_chars = 0;
            }
            current.push(character);
            current_chars += 1;
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        Ok(chunks)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRuntimeRerankHit {
    pub document_index: usize,
    pub score: f64,
}

pub trait LlmRuntimeModelResolver: Send + Sync {
    fn vision_cache_identity(&self) -> Result<Option<String>, ModelError> {
        Ok(None)
    }
    fn vision_runtime_model_config(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        Ok(None)
    }

    fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError>;

    fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError>;
}

pub trait LlmRuntimeClient: Send + Sync {
    fn model_authorization_scope(&self, _: &RuntimeModelConfig) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn generate_vision<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: &'a str,
        _: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Err("vision generation is unavailable".into()) })
    }

    /// Preserve observed provider identity without fabricating it for older adapters.
    fn embed_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(async move { self.embed(config, input).await.map(|vector| (vector, None)) })
    }
    fn embed_batch_with_model<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a [String],
        _: usize,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        (
                            Vec<eliteforge_agent_framework::EmbeddingVector>,
                            Option<String>,
                        ),
                        String,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Err("batch embedding is unavailable".into()) })
    }
    fn embed_query_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
        _dimensions: usize,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        self.embed_with_model(config, input)
    }
    fn embed_query_with_model_in_run<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
        dimensions: usize,
        run: &'a eliteforge_agent_core::RunContext,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(async move {
            self.embed_query_in_run(config, input, dimensions, run)
                .await
                .map(|vector| (vector, None))
        })
    }

    /// Query embeddings share the caller's SDK admission, usage and cancellation.
    fn embed_query_in_run<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: usize,
        _: &'a eliteforge_agent_core::RunContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async { Err("run-scoped query embedding is unavailable".into()) })
    }

    fn generate_observed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<
        Box<
            dyn Future<Output = eliteforge_agent_framework::ObservedModelResult<String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { unsupported_observed() })
    }
    fn embed_observed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
    ) -> Pin<
        Box<
            dyn Future<Output = eliteforge_agent_framework::ObservedModelResult<Vec<f32>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { unsupported_observed() })
    }
    fn rerank_observed<'a>(
        &'a self,
        _: &'a RuntimeModelConfig,
        _: &'a str,
        _: &'a [String],
        _: Option<usize>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = eliteforge_agent_framework::ObservedModelResult<
                        Vec<LlmRuntimeRerankHit>,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { unsupported_observed() })
    }

    fn generate<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

    fn embed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>>;

    fn rerank<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        query: &'a str,
        documents: &'a [String],
        top_n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_length_chunk_strategy_splits_text_by_character_limit() {
        let strategy = CharLengthChunkStrategy {
            max_chars_per_chunk: 3,
        };

        let chunks = strategy
            .split("abcdefg")
            .expect("character-based chunks should be generated");

        assert_eq!(chunks, vec!["abc", "def", "g"]);
    }

    #[test]
    fn char_length_chunk_strategy_rejects_zero_limit() {
        let strategy = CharLengthChunkStrategy {
            max_chars_per_chunk: 0,
        };

        let error = strategy
            .split("abcdefg")
            .expect_err("zero chunk limit should be rejected");

        assert_eq!(error, "max_chars_per_chunk must be greater than zero");
    }
}

fn unsupported_observed<T>() -> eliteforge_agent_framework::ObservedModelResult<T> {
    eliteforge_agent_framework::ObservedModelResult {
        result: Err(eliteforge_agent_core::AgentError::InvalidConfiguration(
            "observed LLM operations must be explicitly implemented".into(),
        )),
        report: Default::default(),
    }
}
