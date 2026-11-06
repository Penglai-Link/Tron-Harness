use std::sync::Arc;

use crate::{
    application::model::{ModelService, RuntimeModelConfig},
    domain::model::ModelError,
};

use super::{
    LlmChunkedEmbeddingCallCommand, LlmChunkedEmbeddingCallResult, LlmEmbeddingBatchCallCommand,
    LlmEmbeddingBatchCallResult, LlmEmbeddingCallCommand, LlmEmbeddingCallResult,
    LlmEmbeddingChunkResultItem, LlmEmbeddingChunkStrategy, LlmRerankCallCommand,
    LlmRerankCallResult, LlmRerankResultItem, LlmRuntimeClient, LlmRuntimeError,
    LlmRuntimeModelResolver, LlmTextGenerationCallResult,
};

impl LlmRuntimeModelResolver for ModelService {
    fn auxiliary_runtime_model_config(
        &self,
        workspace: &str,
        kind: crate::domain::model::AutoRouterKind,
        choice: &eliteforge_agent_core::ModelChoice,
        dimensions: Option<usize>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        self.route_auxiliary_model(workspace, kind, choice, dimensions)
            .map(Some)
    }

    fn embedding_runtime_model_config(
        &self,
        workspace: &str,
        code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        ModelService::embedding_runtime_model_config(self, workspace, code)
    }
    fn rerank_runtime_model_config(
        &self,
        workspace: &str,
        code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        ModelService::rerank_runtime_model_config(self, workspace, code)
    }
}

#[derive(Clone)]
pub struct LlmRuntimeService {
    pub(super) model_resolver: Arc<dyn LlmRuntimeModelResolver>,
    pub(super) client: Arc<dyn LlmRuntimeClient>,
}

impl LlmRuntimeService {
    pub async fn transcribe_with_config(
        &self, config: &RuntimeModelConfig, audio: Vec<u8>, mime: &str, file_name: &str,
    ) -> Result<String, String> {
        self.client.transcribe(config, audio, mime, file_name).await
    }

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

    /// Pin the existing resolved model before indexing any document chunks.
    pub fn resolve_embedding_index_model_code(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<String, LlmRuntimeError> {
        let config = self.resolve_embedding_model_config(workspace, model)?;
        Ok(config.model_code)
    }

    /// Stable, secret-free identity for a concrete locally indexed vector space.
    pub fn embedding_space_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimensions: usize,
    ) -> Result<String, LlmRuntimeError> {
        use sha2::{Digest, Sha256};
        let config = self.resolve_auxiliary(
            workspace,
            crate::domain::model::AutoRouterKind::Embedding,
            model.map(str::to_owned),
            None,
            vec![eliteforge_agent_core::Modality::Text],
            &[],
            Some(dimensions),
        )?;
        let identity =
            if let Some(route) = config.route.as_ref().filter(|route| route.shared_identity) {
                serde_json::to_vec(&(
                    &route.model_code,
                    &route.model_code,
                    dimensions,
                    "knowledge-base-local-v1",
                ))
            } else {
                serde_json::to_vec(&(
                    &config.model_code,
                    &config.model_name,
                    config.protocol,
                    &config.base_url,
                    &config.api_path,
                    dimensions,
                    "knowledge-base-local-v1",
                ))
            }
            .map_err(|error| LlmRuntimeError::invalid_provider_response(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(identity)))
    }

    pub async fn embedding_query_in_run(
        &self,
        command: LlmEmbeddingCallCommand,
        dimensions: usize,
        run: &eliteforge_agent_core::RunContext,
    ) -> Result<LlmEmbeddingCallResult, LlmRuntimeError> {
        let workspace =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        let input = required_trimmed(command.input, LlmRuntimeError::InputRequired)?;
        let model = optional_trimmed(command.model_code);
        let config = self.resolve_auxiliary(
            &workspace,
            crate::domain::model::AutoRouterKind::Embedding,
            model,
            command.provider_id,
            command.required_modalities,
            &[input.clone().into()],
            Some(dimensions),
        )?;
        if let Some(allowance) = run.extensions.get::<ContextQueryAllowance>() {
            allowance
                .admit()
                .map_err(LlmRuntimeError::provider_failed)?;
        }
        let (embedding, provider_model) = self
            .client
            .embed_query_with_model_in_run(&config, &input, dimensions, run)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        if dimensions == 0
            || embedding.len() != dimensions
            || embedding.iter().any(|value| !value.is_finite())
        {
            return Err(LlmRuntimeError::invalid_provider_response(
                "query embedding does not match index space",
            ));
        }
        Ok(LlmEmbeddingCallResult {
            model_code: config.model_code,
            provider_model,
            embedding,
        })
    }

    pub async fn embedding_query_call(
        &self,
        command: LlmEmbeddingCallCommand,
        dimensions: usize,
    ) -> Result<LlmEmbeddingCallResult, LlmRuntimeError> {
        let workspace =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        let input = required_trimmed(command.input, LlmRuntimeError::InputRequired)?;
        let model = optional_trimmed(command.model_code);
        let config = self.resolve_auxiliary(
            &workspace,
            crate::domain::model::AutoRouterKind::Embedding,
            model,
            command.provider_id,
            command.required_modalities,
            &[input.clone().into()],
            Some(dimensions),
        )?;
        let (embedding, provider_model) = self
            .client
            .embed_query_with_model(&config, &input, dimensions)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        if dimensions == 0
            || embedding.len() != dimensions
            || embedding.iter().any(|v| !v.is_finite())
        {
            return Err(LlmRuntimeError::invalid_provider_response(
                "query embedding does not match index space",
            ));
        }
        Ok(LlmEmbeddingCallResult {
            model_code: config.model_code,
            provider_model,
            embedding,
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
        let config = self.resolve_auxiliary(
            &workspace_code,
            crate::domain::model::AutoRouterKind::Embedding,
            model_code,
            command.provider_id,
            command.required_modalities,
            &[input.clone().into()],
            None,
        )?;

        let (embedding, provider_model) = self
            .client
            .embed_with_model(&config, &input)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        if embedding.is_empty() {
            return Err(LlmRuntimeError::invalid_provider_response(
                "embedding response contained an empty vector",
            ));
        }
        Ok(LlmEmbeddingCallResult {
            model_code: config.model_code,
            provider_model,
            embedding,
        })
    }

    pub async fn embedding_batch_call(
        &self,
        command: LlmEmbeddingBatchCallCommand,
    ) -> Result<LlmEmbeddingBatchCallResult, LlmRuntimeError> {
        let workspace =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        if command.inputs.is_empty() || command.dimensions == 0 {
            return Err(LlmRuntimeError::invalid_provider_response(
                "empty embedding batch or dimension",
            ));
        }
        let inputs = command
            .inputs
            .into_iter()
            .map(|input| required_trimmed(input, LlmRuntimeError::InputRequired))
            .collect::<Result<Vec<_>, _>>()?;
        let model = optional_trimmed(command.model_code);
        let contents = inputs
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<eliteforge_agent_core::Content>>();
        let config = self.resolve_auxiliary(
            &workspace,
            crate::domain::model::AutoRouterKind::Embedding,
            model,
            command.provider_id,
            command.required_modalities,
            &contents,
            Some(command.dimensions),
        )?;
        let (vectors, provider_model) = self
            .client
            .embed_batch_with_model(&config, &inputs, command.dimensions)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        let response = eliteforge_agent_framework::EmbeddingResponse {
            model: eliteforge_agent_framework::ModelRef::new("http-embedding", &config.model_code),
            provider_model,
            vectors,
            input_tokens: None,
        };
        response
            .validate(inputs.len(), Some(command.dimensions))
            .map_err(|error| {
                LlmRuntimeError::invalid_provider_response(format!(
                    "Embedding批次返回的数量、索引或向量无效（要求{}维）: {error}",
                    command.dimensions
                ))
            })?;
        Ok(LlmEmbeddingBatchCallResult {
            model_code: config.model_code,
            provider_model: response.provider_model,
            vectors: response.vectors,
        })
    }

    pub async fn embedding_content_batch_call(
        &self,
        command: super::LlmEmbeddingContentBatchCallCommand,
    ) -> Result<LlmEmbeddingBatchCallResult, LlmRuntimeError> {
        let workspace =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        if command.inputs.is_empty() || command.dimensions == 0 {
            return Err(LlmRuntimeError::invalid_provider_response(
                "empty embedding batch or dimension",
            ));
        }
        let inputs = command.inputs;
        if inputs.iter().any(|input| input.is_empty()) {
            return Err(LlmRuntimeError::InputRequired);
        }
        let model = optional_trimmed(command.model_code);
        let config = self.resolve_auxiliary(
            &workspace,
            crate::domain::model::AutoRouterKind::Embedding,
            model,
            command.provider_id,
            command.required_modalities,
            &inputs,
            Some(command.dimensions),
        )?;
        let (vectors, provider_model) = self
            .client
            .embed_content_batch_with_model(&config, &inputs, command.dimensions)
            .await
            .map_err(LlmRuntimeError::provider_failed)?;
        let response = eliteforge_agent_framework::EmbeddingResponse {
            model: eliteforge_agent_framework::ModelRef::new("http-embedding", &config.model_code),
            provider_model,
            vectors,
            input_tokens: None,
        };
        response
            .validate(inputs.len(), Some(command.dimensions))
            .map_err(|error| {
                LlmRuntimeError::invalid_provider_response(format!(
                    "Embedding批次返回的数量、索引或向量无效（要求{}维）: {error}",
                    command.dimensions
                ))
            })?;
        Ok(LlmEmbeddingBatchCallResult {
            model_code: config.model_code,
            provider_model: response.provider_model,
            vectors: response.vectors,
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
        let config = self.resolve_auxiliary(
            &workspace_code,
            crate::domain::model::AutoRouterKind::Embedding,
            model_code,
            command.provider_id,
            command.required_modalities,
            &[input.clone().into()],
            None,
        )?;
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
        let config = self.resolve_auxiliary(
            workspace_code,
            crate::domain::model::AutoRouterKind::Embedding,
            model_code.map(str::to_owned),
            None,
            vec![eliteforge_agent_core::Modality::Text],
            &[],
            Some(crate::shared::sqlite_extensions::TRON_VECTOR_INDEX_DIMENSION),
        )?;
        Ok(config)
    }

    fn resolve_auxiliary(
        &self,
        workspace: &str,
        kind: crate::domain::model::AutoRouterKind,
        model_code: Option<String>,
        provider_id: Option<String>,
        mut required_modalities: Vec<eliteforge_agent_core::Modality>,
        inputs: &[eliteforge_agent_core::Content],
        dimensions: Option<usize>,
    ) -> Result<RuntimeModelConfig, LlmRuntimeError> {
        for modality in inputs
            .iter()
            .flat_map(eliteforge_agent_core::Content::modalities)
        {
            if !required_modalities.contains(&modality) {
                required_modalities.push(modality);
            }
        }
        let choice = eliteforge_agent_core::ModelChoice {
            model_code,
            provider_id,
            required_modalities,
            ..Default::default()
        };
        self.model_resolver
            .auxiliary_runtime_model_config(workspace, kind, &choice, dimensions)
            .map_err(map_model_error)?
            .ok_or(match kind {
                crate::domain::model::AutoRouterKind::Embedding => {
                    LlmRuntimeError::EmbeddingModelUnavailable
                }
                _ => LlmRuntimeError::RerankModelUnavailable,
            })
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

    pub async fn rerank_content_call(
        &self,
        command: super::LlmContentRerankCallCommand,
    ) -> Result<Vec<super::LlmRuntimeRerankHit>, LlmRuntimeError> {
        let workspace =
            required_trimmed(command.workspace_code, LlmRuntimeError::WorkspaceRequired)?;
        if command.query.is_empty() {
            return Err(LlmRuntimeError::QueryRequired);
        }
        if command.documents.is_empty()
            || command
                .documents
                .iter()
                .any(eliteforge_agent_core::Content::is_empty)
        {
            return Err(LlmRuntimeError::DocumentsRequired);
        }
        let mut inputs = command.documents.clone();
        inputs.push(command.query.clone());
        let config = self.resolve_auxiliary(
            &workspace,
            crate::domain::model::AutoRouterKind::Rerank,
            command.model_code,
            command.provider_id,
            command.required_modalities,
            &inputs,
            None,
        )?;
        self.client
            .rerank_content_with_model(&config, &command.query, &command.documents, command.top_n)
            .await
            .map_err(LlmRuntimeError::provider_failed)
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

        let mut contents = documents
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<eliteforge_agent_core::Content>>();
        contents.push(query.clone().into());
        let config = self.resolve_auxiliary(
            &workspace_code,
            crate::domain::model::AutoRouterKind::Rerank,
            model_code,
            command.provider_id,
            command.required_modalities,
            &contents,
            None,
        )?;
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

/// Query retrieval shares the existing context auxiliary allowance.
#[derive(Clone)]
pub(crate) struct ContextQueryAllowance {
    pub calls: Arc<std::sync::atomic::AtomicUsize>,
    pub limit: usize,
}
impl ContextQueryAllowance {
    fn admit(&self) -> Result<(), String> {
        use std::sync::atomic::Ordering;
        self.calls
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |calls| {
                (calls < self.limit).then(|| calls + 1)
            })
            .map(|_| ())
            .map_err(|_| "shared context auxiliary call allowance exhausted".into())
    }
}
