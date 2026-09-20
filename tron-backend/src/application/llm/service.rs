use std::sync::Arc;

use crate::{
    application::model::{ModelService, RuntimeModelConfig},
    domain::model::ModelError,
};

use super::{
    LlmChunkedEmbeddingCallCommand, LlmChunkedEmbeddingCallResult, LlmEmbeddingCallCommand,
    LlmEmbeddingCallResult, LlmEmbeddingChunkResultItem, LlmEmbeddingChunkStrategy,
    LlmRerankCallCommand, LlmRerankCallResult, LlmRerankResultItem, LlmRuntimeClient,
    LlmRuntimeError, LlmRuntimeModelResolver, LlmTextGenerationCallResult,
};

impl LlmRuntimeModelResolver for ModelService {
    fn text_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        ModelService::text_runtime_model_config(self, workspace_code, model_code)
    }

    fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        ModelService::embedding_runtime_model_config(self, workspace_code, model_code)
    }

    fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        ModelService::rerank_runtime_model_config(self, workspace_code, model_code)
    }
}

#[derive(Clone)]
pub struct LlmRuntimeService {
    model_resolver: Arc<dyn LlmRuntimeModelResolver>,
    client: Arc<dyn LlmRuntimeClient>,
}

impl LlmRuntimeService {
    pub fn new(
        model_resolver: Arc<dyn LlmRuntimeModelResolver>,
        client: Arc<dyn LlmRuntimeClient>,
    ) -> Self {
        Self {
            model_resolver,
            client,
        }
    }

    pub async fn text_generation_call_with_config(
        &self,
        config: RuntimeModelConfig,
        prompt: String,
    ) -> Result<LlmTextGenerationCallResult, LlmRuntimeError> {
        let prompt = required_trimmed(prompt, LlmRuntimeError::InputRequired)?;
        let output = self
            .client
            .generate(&config, prompt.as_str())
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        if output.trim().is_empty() {
            return Err(LlmRuntimeError::invalid_provider_response(
                "text generation response was empty",
            ));
        }
        Ok(LlmTextGenerationCallResult {
            model_code: config.model_code,
            output,
        })
    }

    pub async fn embedding_call(
        &self,
        command: LlmEmbeddingCallCommand,
    ) -> Result<LlmEmbeddingCallResult, LlmRuntimeError> {
        let workspace_code =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        let input = required_trimmed(command.input, LlmRuntimeError::InputRequired)?;
        let model_code = optional_trimmed(command.model_code);
        let config =
            self.resolve_embedding_model_config(workspace_code.as_str(), model_code.as_deref())?;

        let embedding = self.embed_chunk(&config, input.as_str(), None).await?;
        Ok(LlmEmbeddingCallResult {
            model_code: config.model_code,
            embedding,
        })
    }

    pub async fn embedding_call_with_chunking(
        &self,
        command: LlmChunkedEmbeddingCallCommand,
        chunk_strategy: &dyn LlmEmbeddingChunkStrategy,
    ) -> Result<LlmChunkedEmbeddingCallResult, LlmRuntimeError> {
        let workspace_code =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        let input = required_trimmed(command.input, LlmRuntimeError::InputRequired)?;
        let model_code = optional_trimmed(command.model_code);
        let config =
            self.resolve_embedding_model_config(workspace_code.as_str(), model_code.as_deref())?;
        let chunks = chunk_strategy
            .split(input.as_str())
            .map_err(LlmRuntimeError::invalid_chunk_strategy)?;
        if chunks.is_empty() {
            return Err(LlmRuntimeError::invalid_chunk_strategy(
                "chunk strategy returned no slices",
            ));
        }

        let mut chunk_results = Vec::with_capacity(chunks.len());
        for (chunk_index, content) in chunks.into_iter().enumerate() {
            if content.is_empty() {
                return Err(LlmRuntimeError::invalid_chunk_strategy(format!(
                    "chunk strategy produced empty slice at index {chunk_index}"
                )));
            }
            let embedding = self
                .embed_chunk(&config, content.as_str(), Some(chunk_index))
                .await?;
            chunk_results.push(LlmEmbeddingChunkResultItem {
                chunk_index,
                content,
                embedding,
            });
        }

        Ok(LlmChunkedEmbeddingCallResult {
            model_code: config.model_code,
            chunks: chunk_results,
        })
    }

    fn resolve_embedding_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<RuntimeModelConfig, LlmRuntimeError> {
        let config = self
            .model_resolver
            .embedding_runtime_model_config(workspace_code, model_code)
            .map_err(map_model_error)?
            .ok_or(LlmRuntimeError::EmbeddingModelUnavailable)?;
        Ok(config)
    }

    async fn embed_chunk(
        &self,
        config: &RuntimeModelConfig,
        input: &str,
        chunk_index: Option<usize>,
    ) -> Result<Vec<f32>, LlmRuntimeError> {
        let embedding = self
            .client
            .embed(config, input)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        if embedding.is_empty() {
            let message = if let Some(chunk_index) = chunk_index {
                format!("embedding response contained an empty vector for chunk {chunk_index}")
            } else {
                "embedding response contained an empty vector".into()
            };
            return Err(LlmRuntimeError::invalid_provider_response(message));
        }
        Ok(embedding)
    }

    pub async fn rerank_call(
        &self,
        command: LlmRerankCallCommand,
    ) -> Result<LlmRerankCallResult, LlmRuntimeError> {
        let workspace_code =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        let query = required_trimmed(command.query, LlmRuntimeError::QueryRequired)?;
        let model_code = optional_trimmed(command.model_code);
        let documents = command
            .documents
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if documents.is_empty() {
            return Err(LlmRuntimeError::DocumentsRequired);
        }

        let config = self
            .model_resolver
            .rerank_runtime_model_config(workspace_code.as_str(), model_code.as_deref())
            .map_err(map_model_error)?
            .ok_or(LlmRuntimeError::RerankModelUnavailable)?;
        let hits = self
            .client
            .rerank(&config, query.as_str(), documents.as_slice(), command.top_n)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        let items = hits
            .into_iter()
            .filter_map(|hit| {
                documents
                    .get(hit.document_index)
                    .map(|document| LlmRerankResultItem {
                        document_index: hit.document_index,
                        score: hit.score,
                        document: document.clone(),
                    })
            })
            .collect::<Vec<_>>();

        Ok(LlmRerankCallResult {
            model_code: config.model_code,
            items,
        })
    }
}

fn required_trimmed(value: String, error: LlmRuntimeError) -> Result<String, LlmRuntimeError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(error)
    } else {
        Ok(trimmed.to_string())
    }
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn map_model_error(error: ModelError) -> LlmRuntimeError {
    LlmRuntimeError::model_resolution(error.to_string())
}
