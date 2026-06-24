use serde_json::{Value, json};

use crate::domain::agent::{
    AgentMessage, AgentMessagePart, AgentModelConfig, AgentModelProtocol, ModelFinish,
    ModelRequest, ModelResponse, ModelStreamEvent, ModelTokenUsage, ToolCall,
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
        payload["thinking"] = json!({ "type": "enabled" });
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
        payload["parallel_tool_calls"] = json!(true);
    }
    payload
}

fn parse_chat_response(body: &Value) -> Option<ModelResponse> {
    let token_usage = parse_openai_compatible_usage(body);
    let response_id = body.get("id").and_then(Value::as_str).map(str::to_string);
    let message = body.get("choices")?.as_array()?.first()?.get("message")?;
    let mut parts = Vec::new();
    if let Some(content) = message.get("content").and_then(text_from_content) {
        parts.push(AgentMessagePart::Text { text: content });
    }
    if let Some(summary) = chat_reasoning_summary_delta(message) {
        parts.push(AgentMessagePart::ReasoningSummary { text: summary });
    }
    if let Some(tool_calls) = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| parse_chat_tool_calls(calls.as_slice()))
        .filter(|calls| !calls.is_empty())
    {
        parts.extend(tool_calls.into_iter().map(tool_call_part));
        return Some(ModelResponse::from_assistant(
            AgentMessage::new("assistant", parts),
            ModelFinish::ToolCalls,
            token_usage,
            None,
            response_id,
        ));
    }
    if let Some(function_call) = message.get("function_call") {
        let tool_call = parse_chat_legacy_function_call(function_call)?;
        parts.push(tool_call_part(tool_call));
        return Some(ModelResponse::from_assistant(
            AgentMessage::new("assistant", parts),
            ModelFinish::ToolCalls,
            token_usage,
            None,
            response_id,
        ));
    }
    if parts.is_empty()
        && let Some(output) = message
            .get("reasoning_content")
            .and_then(Value::as_str)
            .map(str::to_owned)
    {
        parts.push(AgentMessagePart::Text { text: output });
    }
    (!parts.is_empty()).then(|| {
        ModelResponse::from_assistant(
            AgentMessage::new("assistant", parts),
            ModelFinish::Final,
            token_usage,
            None,
            response_id,
        )
    })
}

fn parse_chat_tool_calls(calls: &[Value]) -> Vec<ToolCall> {
    calls
        .iter()
        .filter_map(parse_chat_tool_call)
        .collect::<Vec<_>>()
}

fn parse_chat_tool_call(call: &Value) -> Option<ToolCall> {
    let function = call.get("function")?;
    let name = function.get("name")?.as_str()?.to_owned();
    let call_id = call
        .get("call_id")
        .or_else(|| call.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let arguments = function
        .get("arguments")
        .and_then(Value::as_str)
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| json!({}));
    Some(ToolCall {
        call_id,
        tool_name: name,
        input: arguments,
    })
}

fn parse_chat_legacy_function_call(function_call: &Value) -> Option<ToolCall> {
    let name = function_call.get("name")?.as_str()?;
    let arguments = function_call
        .get("arguments")
        .and_then(Value::as_str)
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| json!({}));
    Some(ToolCall {
        call_id: None,
        tool_name: name.to_owned(),
        input: arguments,
    })
}

fn tool_call_part(call: ToolCall) -> AgentMessagePart {
    AgentMessagePart::ToolCall {
        call_id: call.call_id,
        tool_name: call.tool_name,
        input: call.input,
    }
}

fn parse_chat_stream_event(body: &Value) -> Option<ModelStreamEvent> {
    let delta = body.get("choices")?.as_array()?.first()?.get("delta")?;
    let event = if let Some(text) = delta.get("content").and_then(text_from_content) {
        ModelStreamEvent::text_delta(text)
    } else if let Some(text) = chat_reasoning_summary_delta(delta) {
        ModelStreamEvent::reasoning_summary_delta(text)
    } else if let Some(text) = chat_reasoning_delta(delta) {
        ModelStreamEvent::reasoning_delta(text)
    } else {
        return None;
    };
    if event.delta.is_empty() {
        None
    } else {
        Some(event)
    }
}

fn chat_reasoning_summary_delta(delta: &Value) -> Option<String> {
    [
        "reasoning_summary_content",
        "reasoning_summary",
        "thinking_summary_content",
        "thinking_summary",
    ]
    .iter()
    .find_map(|key| delta.get(key).and_then(text_from_content))
}

fn chat_reasoning_delta(delta: &Value) -> Option<String> {
    [
        "reasoning_content",
        "reasoning",
        "thinking_content",
        "thinking",
    ]
    .iter()
    .find_map(|key| delta.get(key).and_then(text_from_content))
}
