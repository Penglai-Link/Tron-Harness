use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

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
    generations: Arc<Mutex<VecDeque<Result<String, String>>>>,
    generate_inputs: Arc<Mutex<Vec<String>>>,
    embeddings: Arc<Mutex<VecDeque<Result<Vec<f32>, String>>>>,
    embed_inputs: Arc<Mutex<Vec<String>>>,
    reranks: Arc<Mutex<VecDeque<Result<Vec<super::LlmRuntimeRerankHit>, String>>>>,
}

impl FakeClient {
    fn new() -> Self {
        Self {
            generations: Arc::new(Mutex::new(VecDeque::new())),
            generate_inputs: Arc::new(Mutex::new(Vec::new())),
            embeddings: Arc::new(Mutex::new(VecDeque::new())),
            embed_inputs: Arc::new(Mutex::new(Vec::new())),
            reranks: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl LlmRuntimeClient for FakeClient {
    fn generate<'a>(
        &'a self,
        _config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>
    {
        let prompt = prompt.to_string();
        Box::pin(async move {
            self.generate_inputs
                .lock()
                .expect("generate input mutex poisoned")
                .push(prompt);
            self.generations
                .lock()
                .expect("generation queue mutex poisoned")
                .pop_front()
                .unwrap_or_else(|| Err("missing fake generation response".into()))
        })
    }

    fn embed<'a>(
        &'a self,
        _config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<f32>, String>> + Send + 'a>>
    {
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
            dyn std::future::Future<Output = Result<Vec<super::LlmRuntimeRerankHit>, String>>
                + Send
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

fn text_config() -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: "MODEL-TEXT".into(),
        protocol: ModelProtocol::OpenAiCompatible,
        model_type: ModelType::Text,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        request_headers: Default::default(),
        model_name: "text-test".into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    }
}

fn embedding_config() -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: "MODEL-EMBED".into(),
        protocol: ModelProtocol::OpenAiCompatibleEmbedding,
        model_type: ModelType::TextEmbedding,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        request_headers: Default::default(),
        model_name: "embed-test".into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
        max_retries: Some(0),
    }
}

fn rerank_config() -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: "MODEL-RERANK".into(),
        protocol: ModelProtocol::Rerank,
        model_type: ModelType::TextRerank,
        base_url: "https://provider.test/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        request_headers: Default::default(),
        model_name: "rerank-test".into(),
        thinking_enabled: false,
        supports_developer_role: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens: None,
        top_p: None,
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
    let strategy = super::CharLengthChunkStrategy {
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
            super::LlmRuntimeRerankHit {
                document_index: 1,
                score: 0.99,
            },
            super::LlmRuntimeRerankHit {
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
async fn text_generation_call_uses_resolved_model_and_returns_output() {
    let client = FakeClient::new();
    client
        .generations
        .lock()
        .expect("generation queue mutex poisoned")
        .push_back(Ok("Short title".into()));
    let shared_client = Arc::new(client.clone());
    let service = LlmRuntimeService::new(
        Arc::new(FakeModelResolver {
            embedding_config: Some(embedding_config()),
            rerank_config: Some(rerank_config()),
        }),
        shared_client,
    );

    let result = service
        .text_generation_call_with_config(text_config(), "summarize this conversation".into())
        .await
        .expect("text generation call should succeed");

    assert_eq!(result.model_code, "MODEL-TEXT");
    assert_eq!(result.output, "Short title");
    let generate_inputs = client
        .generate_inputs
        .lock()
        .expect("generate input mutex poisoned")
        .clone();
    assert_eq!(generate_inputs, vec!["summarize this conversation"]);
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
    let strategy = super::CharLengthChunkStrategy {
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
