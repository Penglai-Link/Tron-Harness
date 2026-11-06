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
    provider_rotation: Arc<eliteforge_agent_capability_auto_routing::RoundRobinAutoRouter>,
    prompts: Option<crate::application::prompt::PromptService>,
}
impl HttpLlmRuntimeClient {
    pub fn new(client: sdk::ModelHttpClient) -> Self {
        Self {
            models: Arc::new(HttpModelClient::new(Arc::new(
                HttpModelProviderClient::new(client),
            ))),
            auto_routers: default_auto_routers(),
            provider_rotation: Arc::default(),
            prompts: None,
        }
    }
    pub fn with_model_client(models: Arc<HttpModelClient>) -> Self {
        Self {
            models,
            auto_routers: default_auto_routers(),
            provider_rotation: Arc::default(),
            prompts: None,
        }
    }
    pub fn with_auto_routers(mut self, routers: fw::AutoRouters) -> Self {
        self.auto_routers = routers;
        self
    }
    pub fn with_prompt_service(
        mut self,
        prompts: crate::application::prompt::PromptService,
    ) -> Self {
        self.prompts = Some(prompts);
        self
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
            ModelProtocol::Anthropic => AgentModelProtocol::Anthropic,
            ModelProtocol::Gemini => AgentModelProtocol::Gemini,
            ModelProtocol::OpenAiCompatible | ModelProtocol::Rerank => {
                AgentModelProtocol::ChatCompletions
            }
        },
        base_url: config.base_url.clone(),
        api_path: config.api_path.clone(),
        api_key: config.api_key.clone(),
        request_headers: config.request_headers.clone(),
        model_name: config.model_name.clone(),
        thinking_enabled: config.thinking_enabled,
        supports_developer_role: config.supports_developer_role,
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
#[path = "http_llm_routing.rs"]
mod routing;
#[path = "speech_recognition.rs"]
mod speech_recognition;
impl LlmRuntimeClient for HttpLlmRuntimeClient {
    fn transcribe<'a>(
        &'a self, config: &'a RuntimeModelConfig, audio: Vec<u8>, mime: &'a str, file_name: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(speech_recognition::transcribe(config, audio, mime, file_name))
    }

    fn rerank_content_with_model<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        query: &'a fw::Content,
        documents: &'a [fw::Content],
        top_n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.observed_rerank_content(config, query, documents, top_n)
                .await
                .into_result()
                .map_err(error)
        })
    }

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
