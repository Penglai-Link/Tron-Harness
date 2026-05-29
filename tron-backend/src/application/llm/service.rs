use std::sync::Arc;

use crate::{
    application::model::{ModelService, RuntimeModelConfig},
    domain::model::ModelError,
};

use super::{
    LlmChunkedEmbeddingCallCommand, LlmChunkedEmbeddingCallResult, LlmEmbeddingCallCommand,
    LlmEmbeddingCallResult, LlmEmbeddingChunkResultItem, LlmEmbeddingChunkStrategy,
    LlmRerankCallCommand, LlmRerankCallResult, LlmRerankResultItem, LlmRuntimeClient,
    LlmRuntimeError, LlmRuntimeModelResolver,
};

impl LlmRuntimeModelResolver for ModelService {
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

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use crate::{
        application::model::RuntimeModelConfig,
        domain::model::{ModelError, ModelProtocol, ModelType},
    };

    use super::*;

    #[derive(Clone)]
    struct FakeModelResolver {
        embedding_config: Option<RuntimeModelConfig>,
        rerank_config: Option<RuntimeModelConfig>,
    }

    impl LlmRuntimeModelResolver for FakeModelResolver {
        fn embedding_runtime_model_config(
            &self,
            _workspace_code: &str,
            _model_code: Option<&str>,
        ) -> Result<Option<RuntimeModelConfig>, ModelError> {
            Ok(self.embedding_config.clone())
        }

        fn rerank_runtime_model_config(
            &self,
            _workspace_code: &str,
            _model_code: Option<&str>,
        ) -> Result<Option<RuntimeModelConfig>, ModelError> {
            Ok(self.rerank_config.clone())
        }
    }

    #[derive(Clone)]
    struct FakeClient {
        embeddings: Arc<Mutex<VecDeque<Result<Vec<f32>, String>>>>,
        embed_inputs: Arc<Mutex<Vec<String>>>,
        reranks: Arc<Mutex<VecDeque<Result<Vec<super::super::LlmRuntimeRerankHit>, String>>>>,
    }

    impl FakeClient {
        fn new() -> Self {
            Self {
                embeddings: Arc::new(Mutex::new(VecDeque::new())),
                embed_inputs: Arc::new(Mutex::new(Vec::new())),
                reranks: Arc::new(Mutex::new(VecDeque::new())),
            }
        }
    }

    impl LlmRuntimeClient for FakeClient {
        fn embed<'a>(
            &'a self,
            _config: &'a RuntimeModelConfig,
            input: &'a str,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Vec<f32>, String>> + Send + 'a>,
        > {
            let input = input.to_string();
            Box::pin(async move {
                self.embed_inputs
                    .lock()
                    .expect("embed input mutex poisoned")
                    .push(input);
                self.embeddings
                    .lock()
                    .expect("embedding queue mutex poisoned")
                    .pop_front()
                    .unwrap_or_else(|| Err("missing fake embedding response".into()))
            })
        }

        fn rerank<'a>(
            &'a self,
            _config: &'a RuntimeModelConfig,
            _query: &'a str,
            _documents: &'a [String],
            _top_n: Option<usize>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<Vec<super::super::LlmRuntimeRerankHit>, String>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                self.reranks
                    .lock()
                    .expect("rerank queue mutex poisoned")
                    .pop_front()
                    .unwrap_or_else(|| Err("missing fake rerank response".into()))
            })
        }
    }

    fn embedding_config() -> RuntimeModelConfig {
        RuntimeModelConfig {
            model_code: "MODEL-EMBED".into(),
            protocol: ModelProtocol::OpenAiCompatible,
            model_type: ModelType::Embedding,
            base_url: "https://provider.test/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "embed-test".into(),
            timeout_ms: Some(1000),
            max_retries: Some(0),
        }
    }

    fn rerank_config() -> RuntimeModelConfig {
        RuntimeModelConfig {
            model_code: "MODEL-RERANK".into(),
            protocol: ModelProtocol::Rerank,
            model_type: ModelType::Rerank,
            base_url: "https://provider.test/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "rerank-test".into(),
            timeout_ms: Some(1000),
            max_retries: Some(0),
        }
    }

    #[tokio::test]
    async fn embedding_call_uses_resolved_model_and_returns_embedding_vector() {
        let client = FakeClient::new();
        client
            .embeddings
            .lock()
            .expect("embedding queue mutex poisoned")
            .push_back(Ok(vec![0.1, 0.2, 0.3]));
        let service = LlmRuntimeService::new(
            Arc::new(FakeModelResolver {
                embedding_config: Some(embedding_config()),
                rerank_config: Some(rerank_config()),
            }),
            Arc::new(client),
        );

        let result = service
            .embedding_call(LlmEmbeddingCallCommand {
                workspace_code: "TRON_CORE".into(),
                model_code: None,
                input: "memory chunk".into(),
            })
            .await
            .expect("embedding call should succeed");

        assert_eq!(result.model_code, "MODEL-EMBED");
        assert_eq!(result.embedding, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn embedding_call_with_chunking_embeds_every_slice() {
        let client = FakeClient::new();
        client
            .embeddings
            .lock()
            .expect("embedding queue mutex poisoned")
            .extend([Ok(vec![0.1, 0.2]), Ok(vec![0.3, 0.4]), Ok(vec![0.5, 0.6])]);
        let shared_client = Arc::new(client.clone());
        let service = LlmRuntimeService::new(
            Arc::new(FakeModelResolver {
                embedding_config: Some(embedding_config()),
                rerank_config: Some(rerank_config()),
            }),
            shared_client,
        );
        let strategy = super::super::CharLengthChunkStrategy {
            max_chars_per_chunk: 4,
        };

        let result = service
            .embedding_call_with_chunking(
                LlmChunkedEmbeddingCallCommand {
                    workspace_code: "TRON_CORE".into(),
                    model_code: None,
                    input: "abcdefghij".into(),
                },
                &strategy,
            )
            .await
            .expect("chunked embedding call should succeed");

        assert_eq!(result.model_code, "MODEL-EMBED");
        assert_eq!(result.chunks.len(), 3);
        assert_eq!(result.chunks[0].chunk_index, 0);
        assert_eq!(result.chunks[0].content, "abcd");
        assert_eq!(result.chunks[1].chunk_index, 1);
        assert_eq!(result.chunks[1].content, "efgh");
        assert_eq!(result.chunks[2].chunk_index, 2);
        assert_eq!(result.chunks[2].content, "ij");
        let embed_inputs = client
            .embed_inputs
            .lock()
            .expect("embed input mutex poisoned")
            .clone();
        assert_eq!(embed_inputs, vec!["abcd", "efgh", "ij"]);
    }

    #[tokio::test]
    async fn rerank_call_returns_ranked_documents() {
        let client = FakeClient::new();
        client
            .reranks
            .lock()
            .expect("rerank queue mutex poisoned")
            .push_back(Ok(vec![
                super::super::LlmRuntimeRerankHit {
                    document_index: 1,
                    score: 0.99,
                },
                super::super::LlmRuntimeRerankHit {
                    document_index: 0,
                    score: 0.12,
                },
            ]));
        let service = LlmRuntimeService::new(
            Arc::new(FakeModelResolver {
                embedding_config: Some(embedding_config()),
                rerank_config: Some(rerank_config()),
            }),
            Arc::new(client),
        );

        let result = service
            .rerank_call(LlmRerankCallCommand {
                workspace_code: "TRON_CORE".into(),
                model_code: None,
                query: "best candidate".into(),
                documents: vec!["alpha".into(), "beta".into()],
                top_n: Some(2),
            })
            .await
            .expect("rerank call should succeed");

        assert_eq!(result.model_code, "MODEL-RERANK");
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].document_index, 1);
        assert_eq!(result.items[0].document, "beta");
        assert_eq!(result.items[0].score, 0.99);
    }

    #[tokio::test]
    async fn embedding_call_rejects_blank_input() {
        let service = LlmRuntimeService::new(
            Arc::new(FakeModelResolver {
                embedding_config: Some(embedding_config()),
                rerank_config: Some(rerank_config()),
            }),
            Arc::new(FakeClient::new()),
        );

        let error = service
            .embedding_call(LlmEmbeddingCallCommand {
                workspace_code: "TRON_CORE".into(),
                model_code: None,
                input: "  ".into(),
            })
            .await
            .expect_err("blank input should fail");

        assert_eq!(error, LlmRuntimeError::InputRequired);
    }

    #[tokio::test]
    async fn embedding_call_with_chunking_rejects_invalid_strategy() {
        let client = FakeClient::new();
        client
            .embeddings
            .lock()
            .expect("embedding queue mutex poisoned")
            .push_back(Ok(vec![0.1, 0.2, 0.3]));
        let service = LlmRuntimeService::new(
            Arc::new(FakeModelResolver {
                embedding_config: Some(embedding_config()),
                rerank_config: Some(rerank_config()),
            }),
            Arc::new(client),
        );
        let strategy = super::super::CharLengthChunkStrategy {
            max_chars_per_chunk: 0,
        };

        let error = service
            .embedding_call_with_chunking(
                LlmChunkedEmbeddingCallCommand {
                    workspace_code: "TRON_CORE".into(),
                    model_code: None,
                    input: "abcdef".into(),
                },
                &strategy,
            )
            .await
            .expect_err("invalid strategy should fail");

        assert_eq!(
            error,
            LlmRuntimeError::InvalidChunkStrategy(
                "max_chars_per_chunk must be greater than zero".into()
            )
        );
    }
}
