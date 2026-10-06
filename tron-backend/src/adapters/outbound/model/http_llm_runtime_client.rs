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
#[path = "http_llm_in_run.rs"]
mod in_run;
#[path = "http_llm_observed.rs"]
mod observed;
impl LlmRuntimeClient for HttpLlmRuntimeClient {
    fn model_authorization_scope(&self, config: &RuntimeModelConfig) -> Result<Option<String>, String> {
        self.models.authorization_scope(&runtime_to_agent_model_config(config))
    }
    fn generate_vision<'a>(&'a self, config: &'a RuntimeModelConfig, prompt: &'a str, mime: &'a str, bytes: &'a [u8]) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            use base64::Engine;
            use crate::domain::agent::AgentMessagePart;
            if config.model_type != ModelType::Text { return Err("vision requires a text model".into()); }
            let request = ModelRequest {
                messages: vec![AgentMessage::new("user", vec![AgentMessagePart::Text { text: prompt.into() }, AgentMessagePart::Image { mime_type: mime.into(), data: base64::engine::general_purpose::STANDARD.encode(bytes) }])],
                tools: vec![], prepared_tool_schema: None, system_agent_session_id: None, response_input_items: None, previous_response_id: None,
            };
            let response = self.models.generate_observed(&runtime_to_agent_model_config(config), &request).await.into_result().map_err(error)?;
            if response.finish != ModelFinish::Final { return Err("image description requested tools".into()); }
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
        let url = format!("http://{}",listener.local_addr().unwrap());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move { axum::serve(listener,app).with_graceful_shutdown(async { let _=stopped.await; }).await.unwrap(); });
        let config = RuntimeModelConfig { model_code:"synthetic-local".into(), protocol:ModelProtocol::OpenAiCompatible, model_type:ModelType::Text, base_url:url, api_path:Some("chat/completions".into()), api_key:"synthetic".into(), model_name:"synthetic-vision".into(), thinking_enabled:false, reasoning_effort:None, responses_stateful_enabled:false, temperature:Some(0.0), max_tokens:Some(2048), context_window_tokens:None, top_p:None, max_retries:Some(0) };
        let client=HttpLlmRuntimeClient::new(sdk::ModelHttpClient::new());
        assert_eq!(client.generate_vision(&config,"synthetic prompt","image/jpeg",&[0xff,0xd8,0xff]).await.unwrap(),"synthetic description");
        assert_eq!(calls.load(Ordering::SeqCst),1);
        client.models.close().await;drop(client);stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2),server).await.unwrap().unwrap();
    }

}
