use super::http_model_client::{error, limits, sdk_config};
use super::{HttpModelClient, HttpModelProviderClient};
use crate::{
    application::{
        agent::ModelClient,
        llm::{LlmRuntimeClient, LlmRuntimeRerankHit},
        model::RuntimeModelConfig,
    },
    domain::{
        agent::{AgentMessage, AgentModelConfig, AgentModelProtocol, ModelFinish, ModelRequest},
        model::{ModelProtocol, ModelType},
    },
};
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_model_http as sdk;
use std::{future::Future, pin::Pin, sync::Arc};
#[derive(Clone)]
pub struct HttpLlmRuntimeClient {
    models: Arc<HttpModelClient>,
    auto_routers: fw::AutoRouters,
}
impl HttpLlmRuntimeClient {
    pub fn new(client: sdk::ModelHttpClient) -> Self {
        Self {
            models: Arc::new(HttpModelClient::new(Arc::new(
                HttpModelProviderClient::new(client),
            ))),
            auto_routers: default_auto_routers(),
        }
    }
    pub fn with_model_client(models: Arc<HttpModelClient>) -> Self {
        Self {
            models,
            auto_routers: default_auto_routers(),
        }
    }
    pub fn with_auto_routers(mut self, routers: fw::AutoRouters) -> Self {
        self.auto_routers = routers;
        self
    }
    fn routing_for(&self, model: &AgentModelConfig, reference: &fw::ModelRef) -> fw::AutoRouters {
        let mut routers = self.auto_routers.clone();
        if routers.source.is_none() {
            let mut candidate = fw::AutoRouteCandidate::new(reference.clone());
            candidate.metadata.insert(
                eliteforge_agent_capability_personai::PERSONAI_PROVIDER.into(),
                model.model_code.starts_with("PERSON_AI:").into(),
            );
            candidate.context_window_tokens = model.context_window_tokens.map(|v| v as usize);
            routers.source = Some(Arc::new(
                crate::application::model_router::TronAutoRouteDataSource(vec![candidate]),
            ));
        }
        routers
    }
}
fn default_auto_routers() -> fw::AutoRouters {
    let policy = Arc::new(eliteforge_agent_capability_personai::PersonAiAutoRouter::default());
    fw::AutoRouters {
        embedding: Some(policy.clone()),
        rerank: Some(policy),
        ..Default::default()
    }
}
pub(crate) fn runtime_to_agent_model_config(config: &RuntimeModelConfig) -> AgentModelConfig {
    AgentModelConfig {
        model_code: config.model_code.clone(),
        protocol: match config.protocol {
            ModelProtocol::Responses => AgentModelProtocol::Responses,
            ModelProtocol::OpenAiCompatibleEmbedding => AgentModelProtocol::Embeddings,
            _ => AgentModelProtocol::ChatCompletions,
        },
        base_url: config.base_url.clone(),
        api_path: config.api_path.clone(),
        api_key: config.api_key.clone(),
        model_name: config.model_name.clone(),
        thinking_enabled: config.thinking_enabled,
        reasoning_effort: config.reasoning_effort,
        responses_stateful_enabled: config.responses_stateful_enabled,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        context_window_tokens: config.context_window_tokens,
        top_p: config.top_p,
        max_retries: config.max_retries,
    }
}
#[path = "http_llm_in_run.rs"]
mod in_run;
#[path = "http_llm_observed.rs"]
mod observed;
impl LlmRuntimeClient for HttpLlmRuntimeClient {
    fn embed_content_batch_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        inputs: &'a [fw::Content],
        dimensions: usize,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(Vec<fw::EmbeddingVector>, Option<String>), String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.observed_embed_content_batch(config, inputs, Some(dimensions))
                .await
                .into_result()
                .map(|response| (response.vectors, response.provider_model))
                .map_err(error)
        })
    }
    fn model_authorization_scope(
        &self,
        config: &RuntimeModelConfig,
    ) -> Result<Option<String>, String> {
        self.models
            .authorization_scope(&runtime_to_agent_model_config(config))
    }
    fn generate_vision<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
        mime: &'a str,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            use crate::domain::agent::AgentMessagePart;
            use base64::Engine;
            if config.model_type != ModelType::Text {
                return Err("vision requires a text model".into());
            }
            let request = ModelRequest {
                messages: vec![AgentMessage::new(
                    "user",
                    vec![
                        AgentMessagePart::Text {
                            text: prompt.into(),
                        },
                        AgentMessagePart::Image {
                            mime_type: mime.into(),
                            data: base64::engine::general_purpose::STANDARD.encode(bytes),
                        },
                    ],
                )],
                tools: vec![],
                prepared_tool_schema: None,
                system_agent_session_id: None,
                response_input_items: None,
                previous_response_id: None,
            };
            let response = self
                .models
                .generate_observed(&runtime_to_agent_model_config(config), &request)
                .await
                .into_result()
                .map_err(error)?;
            if response.finish != ModelFinish::Final {
                return Err("image description requested tools".into());
            }
            Ok(response.output_text())
        })
    }

    fn embed_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(async move {
            self.observed_embed(config, input)
                .await
                .into_result()
                .map_err(error)
        })
    }
    fn embed_batch_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        inputs: &'a [String],
        dimensions: usize,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(Vec<fw::EmbeddingVector>, Option<String>), String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.observed_embed_batch(config, inputs, Some(dimensions))
                .await
                .into_result()
                .map(|response| (response.vectors, response.provider_model))
                .map_err(error)
        })
    }
    fn embed_query_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
        dimensions: usize,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(self.query_embedding(config, input, dimensions, None))
    }
    fn embed_query_with_model_in_run<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
        dimensions: usize,
        run: &'a fw::RunContext,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<f32>, Option<String>), String>> + Send + 'a>> {
        Box::pin(self.query_embedding_in_run(config, input, dimensions, run))
    }
    fn embed_query_in_run<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
        dimensions: usize,
        run: &'a fw::RunContext,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.query_embedding_in_run(config, input, dimensions, run)
                .await
                .map(|(vector, _)| vector)
        })
    }

    fn generate_observed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<
        Box<
            dyn Future<Output = eliteforge_agent_framework::ObservedModelResult<String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(self.observed_generate(config, prompt))
    }
    fn embed_observed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<
        Box<
            dyn Future<Output = eliteforge_agent_framework::ObservedModelResult<Vec<f32>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.observed_embed(config, input)
                .await
                .map(|(vector, _)| vector)
        })
    }
    fn rerank_observed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        query: &'a str,
        documents: &'a [String],
        top_n: Option<usize>,
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
        Box::pin(self.observed_rerank(config, query, documents, top_n))
    }

    fn generate<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            self.observed_generate(config, prompt)
                .await
                .into_result()
                .map_err(error)
        })
    }
    fn embed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.observed_embed(config, input)
                .await
                .into_result()
                .map(|(vector, _)| vector)
                .map_err(error)
        })
    }
    fn rerank<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        query: &'a str,
        documents: &'a [String],
        top_n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.observed_rerank(config, query, documents, top_n)
                .await
                .into_result()
                .map_err(error)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::post};
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn embedding_media_uses_declared_adapter_capabilities_before_dispatch() {
        struct MediaFactory {
            inner: HttpModelProviderClient,
            enabled: bool,
        }
        #[async_trait::async_trait]
        impl sdk::ModelTransportFactory for MediaFactory {
            fn embedding_input_modalities(
                &self,
                _: &sdk::AgentModelConfig,
            ) -> Result<Vec<fw::Modality>, fw::ModelError> {
                Ok(if self.enabled {
                    vec![fw::Modality::Text, fw::Modality::Image]
                } else {
                    vec![fw::Modality::Text]
                })
            }
            fn embedding_body(
                &self,
                config: &sdk::AgentModelConfig,
                request: &fw::EmbeddingRequest,
            ) -> Result<Value, fw::ModelError> {
                eliteforge_agent_capability_personai::personai_embedding_request_body(
                    &config.model_name,
                    request,
                )
            }
            async fn create(
                &self,
                config: &sdk::AgentModelConfig,
            ) -> Result<Arc<dyn sdk::ModelTransport>, fw::ModelError> {
                sdk::ModelTransportFactory::create(&self.inner, config).await
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let app = Router::new().route(
            "/embeddings",
            post(move |Json(body): Json<Value>| {
                let seen = seen.clone();
                async move {
                    seen.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(body["input"][0]["image_base64"], "AQID");
                    Json(json!({"model":"fixture", "data":[{"index":0,"embedding":[0.25,0.75]}]}))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let config = RuntimeModelConfig {
            model_code: "fixture".into(),
            protocol: ModelProtocol::OpenAiCompatibleEmbedding,
            model_type: ModelType::Embedding,
            base_url: url,
            api_path: None,
            api_key: "fixture".into(),
            model_name: "fixture".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
        };
        let content = fw::Content(vec![fw::ContentBlock::Media {
            media: fw::MediaContent {
                mime_type: "image/png".into(),
                source: fw::MediaSource::Base64("AQID".into()),
                name: None,
                estimated_tokens: Some(8),
            },
        }]);
        for enabled in [false, true] {
            let models = Arc::new(HttpModelClient::new(Arc::new(MediaFactory {
                inner: HttpModelProviderClient::new(sdk::ModelHttpClient::new()),
                enabled,
            })));
            let client = HttpLlmRuntimeClient::with_model_client(models.clone());
            let result = client
                .embed_content_batch_with_model(&config, std::slice::from_ref(&content), 2)
                .await;
            assert_eq!(result.is_ok(), enabled, "{result:?}");
            assert_eq!(calls.load(Ordering::SeqCst), usize::from(enabled));
            models.close().await;
        }
        server.abort();
    }

    #[tokio::test]
    async fn embedding_batch_dispatches_one_http_array_and_rejects_invalid_indices() {
        use crate::application::llm::{
            LlmEmbeddingBatchCallCommand, LlmRuntimeModelResolver, LlmRuntimeService,
        };
        use crate::domain::model::ModelError;
        struct Resolver(RuntimeModelConfig);
        impl LlmRuntimeModelResolver for Resolver {
            fn embedding_runtime_model_config(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<Option<RuntimeModelConfig>, ModelError> {
                Ok(Some(self.0.clone()))
            }
            fn rerank_runtime_model_config(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<Option<RuntimeModelConfig>, ModelError> {
                Ok(None)
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let app = Router::new().route("/embeddings", post(move |Json(body): Json<Value>| {
            let observed = observed.clone();
            async move {
                let attempt = observed.fetch_add(1, Ordering::SeqCst);
                assert_eq!(body["input"], json!(["first", "second", "third"]));
                assert_eq!(body["dimensions"], 2);
                let last_index = if attempt == 0 { 0 } else { 1 };
                Json(json!({"model":"provider-embedding", "data":[{"index":2,"embedding":[3.0,3.0]}, {"index":1,"embedding":[2.0,2.0]}, {"index":last_index,"embedding":[1.0,1.0]}], "usage":{"prompt_tokens":3,"total_tokens":3}}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (shutdown, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let config = RuntimeModelConfig {
            model_code: "fixture".into(),
            protocol: ModelProtocol::OpenAiCompatibleEmbedding,
            model_type: ModelType::Embedding,
            base_url: url,
            api_path: None,
            api_key: "fixture".into(),
            model_name: "fixture".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
        };
        let client = HttpLlmRuntimeClient::new(sdk::ModelHttpClient::new());
        let llm = LlmRuntimeService::new(Arc::new(Resolver(config)), Arc::new(client.clone()));
        let command = LlmEmbeddingBatchCallCommand {
            workspace_code: "PLATFORM".into(),
            model_code: Some("fixture".into()),
            inputs: vec!["first".into(), "second".into(), "third".into()],
            dimensions: 2,
        };
        let result = llm.embedding_batch_call(command.clone()).await.unwrap();
        assert_eq!(result.model_code, "fixture");
        assert_eq!(result.provider_model.as_deref(), Some("provider-embedding"));
        assert_eq!(
            result
                .vectors
                .iter()
                .map(|vector| vector.index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            result
                .vectors
                .iter()
                .map(|vector| vector.values[0])
                .collect::<Vec<_>>(),
            [1.0, 2.0, 3.0]
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(llm.embedding_batch_call(command.clone()).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let mut invalid = command.clone();
        invalid.inputs.clear();
        assert!(llm.embedding_batch_call(invalid).await.is_err());
        let mut invalid = command;
        invalid.inputs[1] = " ".into();
        assert!(llm.embedding_batch_call(invalid).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        client.models.close().await;
        drop(llm);
        drop(client);
        shutdown.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn native_rerank_batches_capacity_and_merges_global_indices_and_unknown_usage() {
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let sizes = Arc::new(std::sync::Mutex::new(Vec::new()));
        let body_sizes = sizes.clone();
        let app = Router::new().route(
            "/rerank",
            post(move |Json(body): Json<Value>| {
                let seen = seen.clone();
                let body_sizes = body_sizes.clone();
                async move {
                    let attempt = seen.fetch_add(1, Ordering::SeqCst);
                    assert!(body.to_string().len() <= 300);
                    body_sizes.lock().unwrap().push(body.to_string().len());
                    let docs = body["documents"].as_array().unwrap();
                    let results = docs
                        .iter()
                        .enumerate()
                        .map(|(index, value)| {
                            let score = match value.as_str().unwrap().chars().next().unwrap() {
                                'A' => 0.1,
                                'B' => 0.8,
                                'C' => 0.9,
                                _ => 0.6,
                            };
                            json!({"index":index,"relevance_score":score})
                        })
                        .collect::<Vec<_>>();
                    let mut response = json!({"results":results});
                    if attempt == 0 {
                        response["usage"] = json!({"total_tokens":10});
                    }
                    Json(response)
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let config = RuntimeModelConfig {
            model_code: "fixture".into(),
            protocol: ModelProtocol::Rerank,
            model_type: ModelType::Rerank,
            base_url: url,
            api_path: None,
            api_key: "fixture".into(),
            model_name: "fixture".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: Some(300),
            top_p: None,
            max_retries: Some(0),
        };
        let client = HttpLlmRuntimeClient::new(sdk::ModelHttpClient::new());
        let docs = ['A', 'B', 'C', 'D'].map(|c| c.to_string().repeat(90));
        let observed = client.observed_rerank(&config, "q", &docs, Some(2)).await;
        let hits = observed.result.unwrap();
        assert_eq!(
            hits.iter().map(|h| h.document_index).collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(observed.report.calls, 2);
        assert_eq!(
            observed.report.ledger_usage.input_tokens,
            10 + sizes.lock().unwrap()[1] as u64,
            "cumulative budget snapshots must not be double-counted"
        );
        assert!(
            observed.report.provider_usage.is_none(),
            "one batch did not report usage"
        );
        assert_eq!(
            observed
                .report
                .provider_usage_subtotal
                .unwrap()
                .input_tokens,
            10
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let oversized = ["A".repeat(90), "X".repeat(1000)];
        assert!(
            client
                .observed_rerank(&config, "q", &oversized, Some(2))
                .await
                .result
                .is_err()
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "all inputs are checked before paid dispatch"
        );
        client.models.close().await;
        server.abort();
    }

    #[tokio::test]
    async fn native_rerank_limits_search_size_to_candidates_and_skips_empty_dispatch() {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let app = Router::new().route("/rerank", post(move |Json(body): Json<Value>| {
            let observed = observed.clone();
            async move {
                observed.fetch_add(1, Ordering::SeqCst);
                assert_eq!(body["top_n"], 2);
                assert_eq!(body["documents"].as_array().unwrap().len(), 2);
                Json(json!({"results":[{"index":0,"relevance_score":1.0},{"index":1,"relevance_score":0.5}]}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (shutdown, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let config = RuntimeModelConfig {
            model_code: "fixture".into(),
            protocol: ModelProtocol::Rerank,
            model_type: ModelType::Rerank,
            base_url: url,
            api_path: None,
            api_key: "fixture".into(),
            model_name: "fixture".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
        };
        let client = HttpLlmRuntimeClient::new(sdk::ModelHttpClient::new());
        let results = client
            .rerank(
                &config,
                "query",
                &["first".into(), "second".into()],
                Some(20),
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            client
                .rerank(&config, "query", &[], Some(20))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(client.rerank(&config, "query", &[], Some(0)).await.is_err());
        client.models.close().await;
        drop(client);
        shutdown.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }
    #[tokio::test]
    async fn vision_uses_existing_framework_multimodal_client_without_tools() {
        let calls = Arc::new(AtomicUsize::new(0));
        let received = calls.clone();
        let app = Router::new().route("/chat/completions", post(move |Json(body): Json<Value>| {
            let received = received.clone();
            async move {
                received.fetch_add(1, Ordering::SeqCst);
                assert_eq!(body["model"],"synthetic-vision");
                assert!(body["tools"].is_null() || body["tools"].as_array().is_some_and(|a|a.is_empty()));
                let content = body["messages"][0]["content"].as_array().unwrap();
                assert!(content.iter().any(|c|c["type"]=="text" && c["text"]=="synthetic prompt"));
                assert!(content.iter().any(|c|c["type"]=="image_url" && c["image_url"]["url"]=="data:image/jpeg;base64,/9j/"));
                Json(json!({"id":"synthetic","object":"chat.completion","model":"synthetic-vision","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"synthetic description"}}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let config = RuntimeModelConfig {
            model_code: "synthetic-local".into(),
            protocol: ModelProtocol::OpenAiCompatible,
            model_type: ModelType::Text,
            base_url: url,
            api_path: Some("chat/completions".into()),
            api_key: "synthetic".into(),
            model_name: "synthetic-vision".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.0),
            max_tokens: Some(2048),
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
        };
        let client = HttpLlmRuntimeClient::new(sdk::ModelHttpClient::new());
        assert_eq!(
            client
                .generate_vision(
                    &config,
                    "synthetic prompt",
                    "image/jpeg",
                    &[0xff, 0xd8, 0xff]
                )
                .await
                .unwrap(),
            "synthetic description"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        client.models.close().await;
        drop(client);
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
    }
}
