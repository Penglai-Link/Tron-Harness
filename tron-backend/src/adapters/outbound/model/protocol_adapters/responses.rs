use serde_json::{Value, json};

use crate::domain::agent::{
    AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse, ModelStreamEvent,
    ModelTokenUsage, ToolCall,
};

use super::{content::*, registry::ModelProtocolAdapter, usage::parse_responses_usage};

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

    fn parse_stream_terminal_response(&self, body: &Value) -> Result<Option<ModelResponse>, String> {
        Ok(parse_responses_stream_terminal_response(body))
    }

    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
        parse_responses_usage(body)
    }
}

pub fn responses_request(config: &AgentModelConfig, request: &ModelRequest) -> Value {
    let instructions = responses_instructions_text(&request.messages);
    let input = responses_input_value(&request.messages);
    let mut payload = json!({
        "model": config.model_name,
        "input": input,
    });
    if let Some(instructions) = instructions {
        payload["instructions"] = json!(instructions);
    }
    if let Some(temperature) = config.temperature {
        payload["temperature"] = json!(temperature);
    }
    if let Some(max_tokens) = config.max_tokens {
        payload["max_output_tokens"] = json!(max_tokens);
    }
    if config.thinking_enabled {
        payload["reasoning"] = json!({ "effort": "medium" });
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
        payload["parallel_tool_calls"] = json!(true);
    }
    payload
}

fn responses_input_value(messages: &[crate::domain::agent::AgentMessage]) -> Value {
    let input_messages = messages
        .iter()
        .filter(|message| !matches!(message.role.as_str(), "system" | "developer"))
        .collect::<Vec<_>>();
    if input_messages.len() == 1 && input_messages[0].role == "user" {
        return json!(input_messages[0].content.as_str());
    }
    Value::Array(
        input_messages
            .into_iter()
            .map(responses_input_item_json)
            .collect::<Vec<_>>(),
    )
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
        "response.reasoning_summary_text.delta" => body
            .get("delta")
            .and_then(Value::as_str)
            .map(|text| ModelStreamEvent::reasoning_summary_delta(text.to_owned())),
        _ => None,
    }?;
    if event.delta.is_empty() {
        None
    } else {
        Some(event)
    }
}

fn parse_responses_stream_terminal_response(body: &Value) -> Option<ModelResponse> {
    match body.get("type").and_then(Value::as_str) {
        Some("response.completed") => body.get("response").and_then(parse_responses_response),
        _ => None,
    }
}

fn parse_responses_response(body: &Value) -> Option<ModelResponse> {
    let token_usage = parse_responses_usage(body);
    if let Some(output) = body.get("output").and_then(Value::as_array) {
        let tool_calls = output
            .iter()
            .filter_map(parse_responses_tool_call)
            .collect::<Vec<_>>();
        if !tool_calls.is_empty() {
            return Some(ModelResponse::tool_calls_with_usage(tool_calls, token_usage));
        }
        let mut final_output = String::new();
        for item in output {
            if item.get("type").and_then(Value::as_str) == Some("message")
                && let Some(text) = item.get("content").and_then(text_from_content)
            {
                final_output.push_str(&text);
            }
        }
        if !final_output.is_empty() {
            return Some(ModelResponse::final_output_with_usage(
                final_output,
                token_usage,
            ));
        }
        if let Some(text) = output
            .iter()
            .find_map(|item| item.get("content").and_then(text_from_content))
        {
            return Some(ModelResponse::final_output_with_usage(text, token_usage));
        }
    }
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return Some(ModelResponse::final_output_with_usage(text, token_usage));
    }
    None
}

fn parse_responses_tool_call(item: &Value) -> Option<ToolCall> {
    match item.get("type").and_then(Value::as_str) {
        Some("function_call") | Some("custom_tool_call") => {
            let name = item.get("name")?.as_str()?.to_owned();
            let call_id = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .map(|value| value.to_owned());
            let raw_arguments = item
                .get("arguments")
                .or_else(|| item.get("input"))
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let input = serde_json::from_str(raw_arguments).unwrap_or_else(|_| json!({}));
            Some(ToolCall {
                call_id,
                tool_name: name,
                input,
            })
        }
        _ => None,
    }
}
