use serde_json::{Value, json};

use crate::domain::agent::{
    AgentModelConfig, AgentModelProtocol, ModelRequest, ModelResponse, ModelStreamEvent,
    ModelTokenUsage,
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

    fn parse_usage(&self, body: &Value) -> Option<ModelTokenUsage> {
        parse_responses_usage(body)
    }
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
