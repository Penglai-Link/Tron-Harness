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
}
impl HttpLlmRuntimeClient {
    pub fn new(client: sdk::ModelHttpClient) -> Self {
        Self {
            models: Arc::new(HttpModelClient::new(Arc::new(
                HttpModelProviderClient::new(client),
            ))),
        }
    }
    pub fn with_model_client(models: Arc<HttpModelClient>) -> Self {
        Self { models }
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
#[path = "http_llm_observed.rs"]
mod observed;
impl LlmRuntimeClient for HttpLlmRuntimeClient {
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
        Box::pin(self.observed_embed(config, input))
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
            if config.model_type != ModelType::Text {
                return Err("text generation requires TEXT model".into());
            }
            let request = ModelRequest {
                messages: vec![AgentMessage::user(prompt)],
                tools: vec![],
                prepared_tool_schema: None,
                system_agent_session_id: None,
                response_input_items: None,
                previous_response_id: None,
            };
            let response = self
                .models
                .generate(&runtime_to_agent_model_config(config), &request)
                .await?;
            if response.finish != ModelFinish::Final {
                return Err("text generation unexpectedly requested tool calls".into());
            }
            Ok(response.output_text())
        })
    }
    fn embed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        Box::pin(async move {
            if config.model_type != ModelType::Embedding {
                return Err("embedding requires EMBEDDING model".into());
            }
            let model = runtime_to_agent_model_config(config);
            let reference = fw::ModelRef::new("http-embedding", &model.model_code);
            let service = fw::EmbeddingModelService::new(
                reference.clone(),
                Arc::new(sdk::HttpEmbeddingModel {
                    config: sdk_config(&model)?,
                    factory: self.models.factory.clone(),
                }),
            );
            let mut request = fw::EmbeddingRequest::texts([input]);
            request.choice = fw::ModelChoice::model(reference);
            let response = self
                .models
                .jobs()
                .await?
                .embed(
                    service,
                    request,
                    limits(&model),
                    config.max_retries.unwrap_or(0) as usize + 2,
                )
                .await
                .map_err(error)?;
            Ok(response
                .vectors
                .into_iter()
                .next()
                .ok_or("embedding response empty")?
                .values)
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
            if config.model_type != ModelType::Rerank {
                return Err("rerank requires RERANK model".into());
            }
            if top_n == Some(0) {
                return Err("rerank top_n must be positive".into());
            }
            if documents.is_empty() {
                return Ok(Vec::new());
            }
            // Product search limits mean at most N hits; SDK top_n describes
            // the actual candidate set and must stay within its cardinality.
            let top_n = top_n.map(|limit| limit.min(documents.len()));
            let model = runtime_to_agent_model_config(config);
            let reference = fw::ModelRef::new("http-rerank", &model.model_code);
            let service = fw::RerankModelService::new(
                reference.clone(),
                Arc::new(sdk::HttpRerankModel {
                    config: sdk_config(&model)?,
                    factory: self.models.factory.clone(),
                    chat_fallback: config.model_code.starts_with("PERSON_AI:"),
                }),
            );
            let request = fw::RerankRequest {
                choice: fw::ModelChoice::model(reference),
                query: query.into(),
                documents: documents.iter().cloned().map(Into::into).collect(),
                top_n,
            };
            let response = self
                .models
                .jobs()
                .await?
                .rerank(
                    service,
                    request,
                    limits(&model),
                    config.max_retries.unwrap_or(0) as usize + 2,
                )
                .await
                .map_err(error)?;
            Ok(response
                .results
                .into_iter()
                .map(|item| LlmRuntimeRerankHit {
                    document_index: item.index,
                    score: item.score,
                })
                .collect())
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
}
