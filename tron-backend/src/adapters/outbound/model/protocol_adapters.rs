use serde_json::{Value, json};

use crate::domain::agent::{
    AgentMessage, AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse,
    ModelStreamEvent, ModelTokenUsage,
};

pub trait ModelProtocolAdapter: Send + Sync {
    fn protocol(&self) -> AgentModelProtocol;
    fn default_path(&self) -> &'static str;
    fn request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String>;
    fn stream_request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String> {
        let mut body = self.request_body(config, request)?;
        body["stream"] = json!(true);
        Ok(body)
    }
    fn parse_response(&self, body: &Value) -> Result<ModelResponse, String>;
    fn parse_stream_event(&self, body: &Value) -> Result<Option<ModelStreamEvent>, String>;
    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage>;
}

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

#[derive(Debug, Clone, Copy)]
pub struct ResponsesAdapter;

impl ModelProtocolAdapter for ResponsesAdapter {
    fn protocol(&self) -> AgentModelProtocol {
        AgentModelProtocol::Responses
    }

    fn default_path(&self) -> &'static str {
        "responses"
    }

    fn request_body(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<Value, String> {
        Ok(responses_request(config, request))
    }

    fn parse_response(&self, body: &Value) -> Result<ModelResponse, String> {
        parse_responses_response(body).ok_or_else(|| "model response was unsupported".into())
    }

    fn parse_stream_event(&self, body: &Value) -> Result<Option<ModelStreamEvent>, String> {
        Ok(parse_responses_stream_event(body))
    }

    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
        parse_responses_usage(body)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingsAdapter;

impl ModelProtocolAdapter for EmbeddingsAdapter {
    fn protocol(&self) -> AgentModelProtocol {
        AgentModelProtocol::Embeddings
    }

    fn default_path(&self) -> &'static str {
        "embeddings"
    }

    fn request_body(
        &self,
        _config: &AgentModelConfig,
        _request: &ModelRequest,
    ) -> Result<Value, String> {
        Err("embedding models cannot generate text".into())
    }

    fn parse_response(&self, _body: &Value) -> Result<ModelResponse, String> {
        Err("embedding models cannot generate text".into())
    }

    fn parse_stream_event(&self, _body: &Value) -> Result<Option<ModelStreamEvent>, String> {
        Err("embedding models cannot generate text".into())
    }

    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
        parse_input_only_usage(body)
    }
}

pub fn adapter_for_protocol(
    protocol: AgentModelProtocol,
) -> Option<&'static dyn ModelProtocolAdapter> {
    match protocol {
        AgentModelProtocol::ChatCompletions => Some(&ChatCompletionsAdapter),
        AgentModelProtocol::Responses => Some(&ResponsesAdapter),
        AgentModelProtocol::Embeddings => Some(&EmbeddingsAdapter),
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

fn parse_responses_stream_event(body: &Value) -> Option<ModelStreamEvent> {
    let event_type = body.get("type").and_then(Value::as_str).unwrap_or_default();
    let event = match event_type {
        "response.output_text.delta" | "response.refusal.delta" => body
            .get("delta")
            .and_then(Value::as_str)
            .map(|text| ModelStreamEvent::text_delta(text.to_owned())),
        "response.reasoning_text.delta" | "response.reasoning.delta" => body
            .get("delta")
            .and_then(Value::as_str)
            .map(|text| ModelStreamEvent::reasoning_delta(text.to_owned())),
        _ => None,
    }?;
    if event.delta.is_empty() {
        None
    } else {
        Some(event)
    }
}

fn text_from_content(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
    }

    let text = content
        .as_array()?
        .iter()
        .filter_map(|item| {
            item.get("text")
                .or_else(|| item.get("content"))
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() { None } else { Some(text) }
}

fn parse_responses_response(body: &Value) -> Option<ModelResponse> {
    let token_usage = parse_responses_usage(body);
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return Some(ModelResponse::final_output_with_usage(text, token_usage));
    }
    let output = body.get("output")?.as_array()?;
    for item in output {
        if item.get("type").and_then(Value::as_str) == Some("function_call") {
            let name = item.get("name")?.as_str()?;
            let arguments = item
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
    }
    output
        .iter()
        .find_map(|item| item.get("content").and_then(text_from_content))
        .map(|text| ModelResponse::final_output_with_usage(text, token_usage))
}

fn parse_openai_compatible_usage(body: &Value) -> Option<ModelTokenUsage> {
    let usage = body.get("usage")?;
    let input_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(token_count)?;
    let output_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(token_count)?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(token_count)
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));
    Some(ModelTokenUsage {
        input_tokens,
        output_tokens,
        total_tokens,
    })
}

fn parse_responses_usage(body: &Value) -> Option<ModelTokenUsage> {
    let usage = body
        .get("usage")
        .or_else(|| body.pointer("/response/usage"))?;
    let input_tokens = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(token_count)?;
    let output_tokens = usage
        .get("output_tokens")
        .or_else(|| usage.get("completion_tokens"))
        .and_then(token_count)?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(token_count)
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));
    Some(ModelTokenUsage {
        input_tokens,
        output_tokens,
        total_tokens,
    })
}

fn parse_input_only_usage(body: &Value) -> Option<ModelTokenUsage> {
    let usage = body.get("usage")?;
    let input_tokens = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .or_else(|| usage.get("total_tokens"))
        .and_then(token_count)?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(token_count)
        .unwrap_or(input_tokens);
    Some(ModelTokenUsage {
        input_tokens,
        output_tokens: 0,
        total_tokens,
    })
}

fn token_count(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_response_accepts_content_array() {
        let response = ChatCompletionsAdapter
            .parse_response(&json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": [
                            { "type": "text", "text": "hello" },
                            { "type": "text", "text": " world" }
                        ]
                    }
                }]
            }))
            .expect("content array should parse");

        assert_eq!(response, ModelResponse::final_output("hello world"));
    }

    #[test]
    fn chat_response_accepts_legacy_function_call() {
        let response = ChatCompletionsAdapter
            .parse_response(&json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "function_call": {
                            "name": "lookup",
                            "arguments": "{\"query\":\"status\"}"
                        }
                    }
                }]
            }))
            .expect("function_call should parse");

        assert_eq!(
            response,
            ModelResponse::tool_call("lookup", json!({ "query": "status" }))
        );
    }

    #[test]
    fn chat_response_maps_openai_compatible_token_usage() {
        let response = ChatCompletionsAdapter
            .parse_response(&json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "hello"
                    }
                }],
                "usage": {
                    "prompt_tokens": 11,
                    "completion_tokens": 7,
                    "total_tokens": 18
                }
            }))
            .expect("token usage response should parse");

        assert_eq!(
            response.token_usage(),
            Some(ModelTokenUsage {
                input_tokens: 11,
                output_tokens: 7,
                total_tokens: 18,
            })
        );
    }

    #[test]
    fn chat_stream_usage_chunk_maps_openai_compatible_token_usage() {
        let usage = ChatCompletionsAdapter.parse_usage(&json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 7,
                "total_tokens": 18
            }
        }));

        assert_eq!(
            usage,
            Some(ModelTokenUsage {
                input_tokens: 11,
                output_tokens: 7,
                total_tokens: 18,
            })
        );
    }

    #[test]
    fn chat_stream_request_asks_for_usage_chunk() {
        let config = AgentModelConfig {
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "https://provider.test/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "gpt-test".into(),
            temperature: None,
            max_tokens: None,
            context_window_tokens: Some(128_000),
            top_p: None,
            timeout_ms: None,
            max_retries: None,
        };
        let body = ChatCompletionsAdapter
            .stream_request_body(
                &config,
                &ModelRequest {
                    messages: vec![AgentMessage::user("hello")],
                    tools: vec![],
                },
            )
            .expect("stream request should build");

        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["stream_options"]["include_usage"], json!(true));
    }

    #[test]
    fn responses_response_skips_reasoning_and_reads_message_output_text() {
        let response = ResponsesAdapter
            .parse_response(&json!({
                "object": "response",
                "status": "completed",
                "output": [
                    {
                        "type": "reasoning",
                        "summary": [
                            { "type": "summary_text", "text": "thinking" }
                        ]
                    },
                    {
                        "type": "message",
                        "role": "assistant",
                        "content": [
                            { "type": "output_text", "text": "TRON" }
                        ]
                    }
                ]
            }))
            .expect("message output after reasoning should parse");

        assert_eq!(response, ModelResponse::final_output("TRON"));
    }

    #[test]
    fn responses_stream_completion_maps_nested_token_usage() {
        let usage = ResponsesAdapter.parse_usage(&json!({
            "type": "response.completed",
            "response": {
                "id": "resp_1",
                "usage": {
                    "input_tokens": 13,
                    "output_tokens": 8,
                    "total_tokens": 21
                }
            }
        }));

        assert_eq!(
            usage,
            Some(ModelTokenUsage {
                input_tokens: 13,
                output_tokens: 8,
                total_tokens: 21,
            })
        );
    }

    #[test]
    fn embeddings_usage_maps_openai_compatible_token_usage() {
        let usage = EmbeddingsAdapter.parse_usage(&json!({
            "usage": {
                "prompt_tokens": 5,
                "total_tokens": 5
            }
        }));

        assert_eq!(
            usage,
            Some(ModelTokenUsage {
                input_tokens: 5,
                output_tokens: 0,
                total_tokens: 5,
            })
        );
    }

    #[test]
    fn responses_stream_maps_reasoning_and_output_deltas() {
        let reasoning = ResponsesAdapter
            .parse_stream_event(&json!({
                "type": "response.reasoning_text.delta",
                "delta": "Thinking Process:"
            }))
            .expect("reasoning stream event should parse safely");
        let output = ResponsesAdapter
            .parse_stream_event(&json!({
                "type": "response.output_text.delta",
                "delta": "final answer"
            }))
            .expect("output stream event should parse safely");
        let done = ResponsesAdapter
            .parse_stream_event(&json!({
                "type": "response.content_part.done",
                "part": {
                    "content": [
                        { "type": "output_text", "text": "final answer" }
                    ]
                }
            }))
            .expect("done stream event should parse safely");

        assert_eq!(
            reasoning,
            Some(ModelStreamEvent::reasoning_delta("Thinking Process:"))
        );
        assert_eq!(output, Some(ModelStreamEvent::text_delta("final answer")));
        assert_eq!(done, None);
    }
}
