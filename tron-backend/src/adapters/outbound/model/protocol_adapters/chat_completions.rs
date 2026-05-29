use serde_json::{Value, json};

use crate::domain::agent::{
    AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse, ModelStreamEvent,
    ModelTokenUsage,
};

use super::{content::*, registry::ModelProtocolAdapter, usage::parse_openai_compatible_usage};

#[derive(Debug, Clone, Copy)]
pub struct ChatCompletionsAdapter;

impl ModelProtocolAdapter for ChatCompletionsAdapter {
    fn protocol(&self) -> AgentModelProtocol {
        AgentModelProtocol::ChatCompletions
    }

    fn default_path(&self) -> &'static str {
        "chat/completions"
    }

    fn request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String> {
        Ok(chat_completions_request(config, request))
    }

    fn stream_request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String> {
        let mut body = chat_completions_request(config, request);
        body["stream"] = json!(true);
        body["stream_options"] = json!({ "include_usage": true });
        Ok(body)
    }

    fn parse_response(&self, body: &Value) -> Result<ModelResponse, String> {
        parse_chat_response(body).ok_or_else(|| "model response was unsupported".into())
    }

    fn parse_stream_event(&self, body: &Value) -> Result<Option<ModelStreamEvent>, String> {
        Ok(parse_chat_stream_event(body))
    }

    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
        parse_openai_compatible_usage(body)
    }
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
    if config.thinking_enabled {
        payload["enable_thinking"] = json!(true);
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

fn parse_chat_response(body: &Value) -> Option<ModelResponse> {
    let token_usage = parse_openai_compatible_usage(body);
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
        return Some(ModelResponse::tool_call_with_usage(
            name,
            arguments,
            token_usage,
        ));
    }
    if let Some(function_call) = message.get("function_call") {
        let name = function_call.get("name")?.as_str()?;
        let arguments = function_call
            .get("arguments")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| json!({}));
        return Some(ModelResponse::tool_call_with_usage(
            name,
            arguments,
            token_usage,
        ));
    }
    message
        .get("content")
        .and_then(text_from_content)
        .or_else(|| {
            message
                .get("reasoning_content")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .map(|output| ModelResponse::final_output_with_usage(output, token_usage))
}

fn parse_chat_stream_event(body: &Value) -> Option<ModelStreamEvent> {
    let delta = body.get("choices")?.as_array()?.first()?.get("delta")?;
    let event = if let Some(text) = delta.get("content").and_then(text_from_content) {
        ModelStreamEvent::text_delta(text)
    } else if let Some(text) = delta.get("reasoning_content").and_then(Value::as_str) {
        ModelStreamEvent::reasoning_summary_delta(text)
    } else {
        return None;
    };
    if event.delta.is_empty() {
        None
    } else {
        Some(event)
    }
}
