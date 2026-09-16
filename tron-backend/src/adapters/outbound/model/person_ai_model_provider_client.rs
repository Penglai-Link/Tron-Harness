use std::{future::Future, pin::Pin, sync::Arc};

use serde_json::{Value, json};

use crate::{
    application::{
        llm::{LlmRuntimeClient, LlmRuntimeRerankHit},
        model::RuntimeModelConfig,
        model_provider::ModelProviderService,
    },
    domain::{
        agent::{AgentMessage, AgentModelConfig, AgentModelProtocol, ModelFinish, ModelRequest},
        model::ModelType,
    },
};

use super::{
    ChatCompletionsAdapter, HttpLlmRuntimeClient, HttpModelProviderClient, ModelProtocolAdapter,
    ModelProviderClient, chat_completions_request,
};

const PERSON_AI_CONVERSATION_PATH_PREFIX: &str = "person-ai/conversations/";

#[derive(Clone)]
pub struct PersonAiModelProviderClient {
    fallback: HttpModelProviderClient,
    providers: ModelProviderService,
}

impl PersonAiModelProviderClient {
    pub fn new(fallback: HttpModelProviderClient, providers: ModelProviderService) -> Self {
        Self {
            fallback,
            providers,
        }
    }
}

impl ModelProviderClient for PersonAiModelProviderClient {
    fn post_json<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        url: &'a str,
        body: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>> {
        if self.providers.is_person_ai_config(&config.model_code) {
            let (conversation_id, run_code) = person_ai_runtime_metadata(config);
            return Box::pin(async move {
                self.providers
                    .chat_completion(config, body, conversation_id, run_code)
                    .await
            });
        }
        self.fallback.post_json(config, url, body)
    }

    fn post_event_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        url: &'a str,
        body: &'a Value,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        if self.providers.is_person_ai_config(&config.model_code) {
            let (conversation_id, run_code) = person_ai_runtime_metadata(config);
            return Box::pin(async move {
                self.providers
                    .chat_completion_stream(config, body, conversation_id, run_code, sink)
                    .await
            });
        }
        self.fallback.post_event_stream(config, url, body, sink)
    }
}

#[derive(Clone)]
pub struct PersonAiAwareLlmRuntimeClient {
    fallback: HttpLlmRuntimeClient,
    providers: ModelProviderService,
}

impl PersonAiAwareLlmRuntimeClient {
    pub fn new(fallback: HttpLlmRuntimeClient, providers: ModelProviderService) -> Self {
        Self {
            fallback,
            providers,
        }
    }
}

impl LlmRuntimeClient for PersonAiAwareLlmRuntimeClient {
    fn generate<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        if !self.providers.is_person_ai_config(&config.model_code) {
            return self.fallback.generate(config, prompt);
        }
        Box::pin(async move {
            if config.model_type != ModelType::Text {
                return Err("Person-AI text generation requires a TEXT model".into());
            }
            let model_config = runtime_to_agent_config(config);
            let request = ModelRequest {
                messages: vec![AgentMessage::user(prompt)],
                tools: Vec::new(),
                prepared_tool_schema: None,
                system_agent_session_id: None,
                response_input_items: None,
                previous_response_id: None,
            };
            let response = self
                .providers
                .chat_completion(
                    &model_config,
                    &chat_completions_request(&model_config, &request),
                    None,
                    None,
                )
                .await?;
            let response = ChatCompletionsAdapter
                .parse_response(&response)
                .map_err(|_| {
                    "Person-AI text generation returned an unsupported response".to_string()
                })?;
            if response.finish != ModelFinish::Final {
                return Err("Person-AI text generation unexpectedly requested tool calls".into());
            }
            Ok(response.output_text())
        })
    }

    fn embed<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<f32>, String>> + Send + 'a>> {
        if !self.providers.is_person_ai_config(&config.model_code) {
            return self.fallback.embed(config, input);
        }
        Box::pin(async move {
            if config.model_type != ModelType::Embedding {
                return Err("Person-AI embeddings require an EMBEDDING model".into());
            }
            let response = self.providers.embedding(&json!({ "input": input })).await?;
            parse_embedding(&response)
        })
    }

    fn rerank<'a>(
        &'a self,
        config: &'a RuntimeModelConfig,
        query: &'a str,
        documents: &'a [String],
        top_n: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<LlmRuntimeRerankHit>, String>> + Send + 'a>> {
        if !self.providers.is_person_ai_config(&config.model_code)
            || config.model_type != ModelType::Rerank
        {
            return self.fallback.rerank(config, query, documents, top_n);
        }
        Box::pin(async move {
            let model_config = runtime_to_agent_config(config);
            self.providers
                .llm_rerank_with_config(&model_config, query, documents, top_n)
                .await
                .map(|items| {
                    items
                        .into_iter()
                        .map(|(document_index, score)| LlmRuntimeRerankHit {
                            document_index,
                            score,
                        })
                        .collect()
                })
        })
    }
}

pub fn with_person_ai_conversation(
    mut config: AgentModelConfig,
    session_code: &str,
    run_code: Option<&str>,
) -> AgentModelConfig {
    if config.model_code.starts_with("PERSON_AI:") {
        config.api_path = Some(
            match run_code.map(str::trim).filter(|value| !value.is_empty()) {
                Some(run_code) => {
                    format!("{PERSON_AI_CONVERSATION_PATH_PREFIX}{session_code}?runCode={run_code}")
                }
                None => format!("{PERSON_AI_CONVERSATION_PATH_PREFIX}{session_code}"),
            },
        );
    }
    config
}

fn person_ai_runtime_metadata(config: &AgentModelConfig) -> (Option<&str>, Option<&str>) {
    let path = config
        .api_path
        .as_deref()
        .and_then(|path| path.strip_prefix(PERSON_AI_CONVERSATION_PATH_PREFIX));
    let Some(path) = path else {
        return (None, None);
    };
    let (conversation_id, query) = path.split_once('?').unwrap_or((path, ""));
    let conversation_id = (!conversation_id.trim().is_empty()).then_some(conversation_id);
    let run_code = query
        .split('&')
        .find_map(|entry| entry.strip_prefix("runCode="))
        .filter(|value| !value.trim().is_empty());
    (conversation_id, run_code)
}

fn runtime_to_agent_config(config: &RuntimeModelConfig) -> AgentModelConfig {
    AgentModelConfig {
        model_code: config.model_code.clone(),
        protocol: AgentModelProtocol::ChatCompletions,
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

fn parse_embedding(response: &Value) -> Result<Vec<f32>, String> {
    response
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("embedding"))
        .and_then(Value::as_array)
        .ok_or_else(|| "Person-AI embedding response missing vector".to_string())?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_f64()
                .map(|value| value as f32)
                .ok_or_else(|| format!("Person-AI embedding value {index} was not numeric"))
        })
        .collect()
}
