use std::{future::Future, pin::Pin};

use serde_json::{Value, json};

use crate::{
    application::agent::ModelClient,
    domain::agent::{
        AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse,
    },
};

#[derive(Debug, Clone, Default)]
pub struct OpenAiCompatibleModelClient {
    client: reqwest::Client,
}

impl ModelClient for OpenAiCompatibleModelClient {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            match config.protocol {
                AgentModelProtocol::ChatCompletions => self.chat_completions(config, request).await,
                AgentModelProtocol::Responses => self.responses(config, request).await,
                AgentModelProtocol::Embeddings => {
                    Err("embedding models cannot generate text".into())
                }
            }
        })
    }
}

impl OpenAiCompatibleModelClient {
    async fn chat_completions(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let url = endpoint_url(config, "chat/completions")?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&chat_completions_request(config, request))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let body: Value = response.json().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("model provider returned {status}"));
        }
        parse_chat_response(&body).ok_or_else(|| "model response was unsupported".into())
    }

    async fn responses(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let url = endpoint_url(config, "responses")?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&responses_request(config, request))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let body: Value = response.json().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("model provider returned {status}"));
        }
        parse_responses_response(&body).ok_or_else(|| "model response was unsupported".into())
    }
}

fn endpoint_url(config: &AgentModelConfig, default_path: &str) -> Result<String, String> {
    let path = config.api_path.as_deref().unwrap_or(default_path);
    let base = config.base_url.trim().trim_end_matches('/');
    let path = path.trim().trim_start_matches('/');
    if base.is_empty() || path.is_empty() {
        return Err("model baseUrl and apiPath are required".into());
    }
    Ok(format!("{base}/{path}"))
}

pub fn chat_completions_request(config: &AgentModelConfig, request: &ModelRequest) -> Value {
    let mut payload = json!({
        "model": config.model_name,
        "messages": request.messages.iter().map(chat_message_json).collect::<Vec<_>>(),
    });
    if let Some(temperature) = config.temperature {
        payload["temperature"] = json!(temperature);
    }
    if let Some(max_tokens) = config.max_tokens {
        payload["max_tokens"] = json!(max_tokens);
    }
    if let Some(top_p) = config.top_p {
        payload["top_p"] = json!(top_p);
    }
    if !request.tools.is_empty() {
        payload["tools"] = json!(
            request
                .tools
                .iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.tool_name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }
                }))
                .collect::<Vec<_>>()
        );
    }
    payload
}

pub fn responses_request(config: &AgentModelConfig, request: &ModelRequest) -> Value {
    let mut payload = json!({
        "model": config.model_name,
        "input": request.messages.iter().map(response_message_json).collect::<Vec<_>>(),
    });
    if let Some(temperature) = config.temperature {
        payload["temperature"] = json!(temperature);
    }
    if let Some(max_tokens) = config.max_tokens {
        payload["max_output_tokens"] = json!(max_tokens);
    }
    if !request.tools.is_empty() {
        payload["tools"] = json!(
            request
                .tools
                .iter()
                .map(|tool| json!({
                    "type": "function",
                    "name": tool.tool_name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                }))
                .collect::<Vec<_>>()
        );
    }
    payload
}

pub fn embeddings_request(config: &AgentModelConfig, input: &str) -> Value {
    json!({
        "model": config.model_name,
        "input": input,
    })
}

fn chat_message_json(message: &AgentMessage) -> Value {
    json!({
        "role": message.role,
        "content": message.content,
    })
}

fn response_message_json(message: &AgentMessage) -> Value {
    json!({
        "role": message.role,
        "content": message.content,
    })
}

fn parse_chat_response(body: &Value) -> Option<ModelResponse> {
    let message = body.get("choices")?.as_array()?.first()?.get("message")?;
    if let Some(tool_call) = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .and_then(|calls| calls.first())
    {
        let function = tool_call.get("function")?;
        let name = function.get("name")?.as_str()?;
        let arguments = function
            .get("arguments")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| json!({}));
        return Some(ModelResponse::tool_call(name, arguments));
    }
    message
        .get("content")
        .and_then(Value::as_str)
        .map(ModelResponse::final_output)
}

fn parse_responses_response(body: &Value) -> Option<ModelResponse> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return Some(ModelResponse::final_output(text));
    }
    let output = body.get("output")?.as_array()?.first()?;
    if output.get("type").and_then(Value::as_str) == Some("function_call") {
        let name = output.get("name")?.as_str()?;
        let arguments = output
            .get("arguments")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| json!({}));
        return Some(ModelResponse::tool_call(name, arguments));
    }
    output
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .map(ModelResponse::final_output)
}
