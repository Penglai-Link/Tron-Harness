use std::{future::Future, pin::Pin, sync::Arc};

use serde_json::Value;

use crate::{
    application::agent::{ModelClient, ModelStreamSink},
    domain::agent::{AgentModelConfig, ModelRequest, ModelResponse},
};

use super::protocol_adapters::{ModelProtocolAdapter, adapter_for_protocol};

#[path = "http_model_stream_accumulator.rs"]
mod http_model_stream_accumulator;

use http_model_stream_accumulator::StreamAccumulator;

pub trait ModelProviderClient: Send + Sync {
    fn post_json<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        url: &'a str,
        body: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>>;

    fn post_event_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        url: &'a str,
        body: &'a Value,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

#[derive(Clone)]
pub struct HttpModelClient {
    provider: Arc<dyn ModelProviderClient>,
}

impl HttpModelClient {
    pub fn new(provider: Arc<dyn ModelProviderClient>) -> Self {
        Self { provider }
    }
}

impl ModelClient for HttpModelClient {
    fn generate<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move { self.generate_with_adapter(config, request).await })
    }

    fn generate_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a ModelRequest,
        sink: ModelStreamSink,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.generate_stream_with_adapter(config, request, sink)
                .await
        })
    }
}

impl HttpModelClient {
    async fn generate_with_adapter(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
    ) -> Result<ModelResponse, String> {
        let adapter = adapter_for_protocol(config.protocol)
            .ok_or_else(|| "model protocol was unsupported".to_string())?;
        let url = endpoint_url(config, adapter.default_path())?;
        let body = adapter.request_body(config, request)?;
        let response_body = self.provider.post_json(config, &url, &body).await?;
        adapter.parse_response(&response_body)
    }

    async fn generate_stream_with_adapter(
        &self,
        config: &AgentModelConfig,
        request: &ModelRequest,
        sink: ModelStreamSink,
    ) -> Result<ModelResponse, String> {
        let adapter = adapter_for_protocol(config.protocol)
            .ok_or_else(|| "model protocol was unsupported".to_string())?;
        let url = endpoint_url(config, adapter.default_path())?;
        let body = adapter.stream_request_body(config, request)?;
        let accumulator = Arc::new(std::sync::Mutex::new(StreamAccumulator::default()));
        let accumulator_for_sink = accumulator.clone();
        let protocol = config.protocol;
        let event_sink = Arc::new(move |event: Value| {
            let mut accumulator = accumulator_for_sink
                .lock()
                .expect("model stream accumulator mutex poisoned");
            if let Some(usage) = adapter.parse_usage(&event) {
                accumulator.token_usage = Some(usage);
            }
            if let Some(response) = adapter.parse_stream_terminal_response(&event)? {
                accumulator.terminal_response = Some(response);
            }
            accumulator.observe_raw_event(protocol, &event);
            if let Some(delta) = adapter.parse_stream_event(&event)? {
                accumulator.observe_delta(&delta);
                sink(delta);
            }
            Ok::<(), String>(())
        });
        self.provider
            .post_event_stream(config, &url, &body, event_sink)
            .await?;
        let accumulator = accumulator
            .lock()
            .expect("model stream accumulator mutex poisoned")
            .clone();
        if let Some(response) = accumulator.terminal_response {
            return Ok(response);
        }
        if let Some(response) = accumulator.into_response() {
            return Ok(response);
        }
        self.generate_with_adapter(config, request).await
    }
}

#[derive(Debug, Clone)]
pub struct HttpModelProviderClient {
    client: reqwest::Client,
}

impl HttpModelProviderClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl ModelProviderClient for HttpModelProviderClient {
    fn post_json<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        url: &'a str,
        body: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>> {
        Box::pin(async move {
            let response = self
                .request(config, url, body)
                .send()
                .await
                .map_err(provider_error)?;
            let status = response.status();
            let body: Value = response.json().await.map_err(provider_error)?;
            if !status.is_success() {
                return Err(provider_status_error(status, &body));
            }
            Ok(body)
        })
    }

    fn post_event_stream<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        url: &'a str,
        body: &'a Value,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let mut response = self
                .request(config, url, body)
                .header("Accept", "text/event-stream")
                .send()
                .await
                .map_err(provider_error)?;
            let status = response.status();
            if !status.is_success() {
                let body: Value = response.json().await.unwrap_or(Value::Null);
                return Err(provider_status_error(status, &body));
            }

            let mut buffer = String::new();
            while let Some(chunk) = response.chunk().await.map_err(provider_error)? {
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                drain_sse_events(&mut buffer, &sink)?;
            }
            drain_sse_events(&mut format!("{buffer}\n\n"), &sink)?;
            Ok(())
        })
    }
}

impl HttpModelProviderClient {
    fn request(
        &self,
        config: &AgentModelConfig,
        url: &str,
        body: &Value,
    ) -> reqwest::RequestBuilder {
        self.client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(body)
    }
}

fn drain_sse_events(
    buffer: &mut String,
    sink: &Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + '_>,
) -> Result<(), String> {
    while let Some(boundary) = buffer.find("\n\n") {
        let frame = buffer[..boundary].to_string();
        buffer.replace_range(..boundary + 2, "");
        for line in frame.lines() {
            let Some(raw_data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = raw_data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let event = serde_json::from_str(data)
                .map_err(|error| format!("model provider stream returned invalid JSON: {error}"))?;
            sink(event)?;
        }
    }
    Ok(())
}

fn provider_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "model provider connection timed out".into()
    } else {
        error.to_string()
    }
}

fn provider_status_error(status: reqwest::StatusCode, body: &Value) -> String {
    let detail = body
        .pointer("/error/message")
        .or_else(|| body.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("model provider request failed");
    format!("model provider returned {status}: {detail}")
}

fn _assert_adapter_object_safe(_: &dyn ModelProtocolAdapter) {}

fn endpoint_url(config: &AgentModelConfig, default_path: &str) -> Result<String, String> {
    let path = config.api_path.as_deref().unwrap_or(default_path);
    let base = config.base_url.trim().trim_end_matches('/');
    let path = path.trim().trim_start_matches('/');
    if base.is_empty() || path.is_empty() {
        return Err("model baseUrl and apiPath are required".into());
    }
    Ok(format!("{base}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::domain::agent::{
        AgentMessage, AgentModelProtocol, ModelRequest, ModelResponse, ModelStreamEvent, ToolCall,
    };

    #[derive(Clone)]
    struct StubProvider {
        stream_events: Vec<Value>,
    }

    impl ModelProviderClient for StubProvider {
        fn post_json<'a>(
            &'a self,
            _config: &'a AgentModelConfig,
            _url: &'a str,
            _body: &'a Value,
        ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>> {
            Box::pin(async { Err("post_json should not be used in this test".into()) })
        }

        fn post_event_stream<'a>(
            &'a self,
            _config: &'a AgentModelConfig,
            _url: &'a str,
            _body: &'a Value,
            sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            let events = self.stream_events.clone();
            Box::pin(async move {
                for event in events {
                    sink(event)?;
                }
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn responses_stream_terminal_tool_calls_override_text_accumulator() {
        let client = HttpModelClient::new(Arc::new(StubProvider {
            stream_events: vec![
                json!({
                    "type": "response.output_text.delta",
                    "delta": "好的，我来同时调用"
                }),
                json!({
                    "type": "response.completed",
                    "response": {
                        "output": [
                            {
                                "type": "function_call",
                                "call_id": "call_123",
                                "name": "lookup",
                                "arguments": "{\"query\":\"status\"}"
                            },
                            {
                                "type": "function_call",
                                "call_id": "call_456",
                                "name": "lookup",
                                "arguments": "{\"query\":\"health\"}"
                            }
                        ]
                    }
                }),
            ],
        }));
        let seen = Arc::new(std::sync::Mutex::new(Vec::<ModelStreamEvent>::new()));
        let seen_for_sink = seen.clone();
        let sink: ModelStreamSink = Arc::new(move |event| {
            seen_for_sink
                .lock()
                .expect("stream event collection mutex poisoned")
                .push(event);
        });

        let response = client
            .generate_stream(&responses_model_config(), &minimal_request(), sink)
            .await
            .expect("responses stream should parse terminal tool calls");

        assert_eq!(
            response,
            ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call_123".into()),
                    tool_name: "lookup".into(),
                    input: json!({ "query": "status" }),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call_456".into()),
                    tool_name: "lookup".into(),
                    input: json!({ "query": "health" }),
                    input_error: None,
                },
            ])
        );
        assert_eq!(
            seen.lock()
                .expect("stream event collection mutex poisoned")
                .as_slice(),
            &[ModelStreamEvent::text_delta("好的，我来同时调用")]
        );
    }

    #[tokio::test]
    async fn chat_completions_stream_accumulates_compatible_summary_and_tool_calls() {
        let client = HttpModelClient::new(Arc::new(StubProvider {
            stream_events: vec![
                json!({
                    "id": "chatcmpl-stream-1",
                    "choices": [{
                        "delta": {
                            "reasoning_summary": "Need a lookup. "
                        }
                    }]
                }),
                json!({
                    "id": "chatcmpl-stream-1",
                    "choices": [{
                        "delta": {
                            "tool_calls": [{
                                "index": 0,
                                "id": "call_chat_1",
                                "function": {
                                    "name": "lookup",
                                    "arguments": "{\"query\":"
                                }
                            }]
                        }
                    }]
                }),
                json!({
                    "id": "chatcmpl-stream-1",
                    "choices": [{
                        "delta": {
                            "tool_calls": [{
                                "index": 0,
                                "function": {
                                    "arguments": "\"status\"}"
                                }
                            }]
                        },
                        "finish_reason": "tool_calls"
                    }],
                    "usage": {
                        "prompt_tokens": 11,
                        "completion_tokens": 3,
                        "total_tokens": 14
                    }
                }),
            ],
        }));
        let seen = Arc::new(std::sync::Mutex::new(Vec::<ModelStreamEvent>::new()));
        let seen_for_sink = seen.clone();
        let sink: ModelStreamSink = Arc::new(move |event| {
            seen_for_sink
                .lock()
                .expect("stream event collection mutex poisoned")
                .push(event);
        });

        let response = client
            .generate_stream(&chat_model_config(), &minimal_request(), sink)
            .await
            .expect("chat completions stream should accumulate tool call response");

        assert_eq!(
            response.finish,
            crate::domain::agent::ModelFinish::ToolCalls
        );
        assert_eq!(response.response_id(), Some("chatcmpl-stream-1"));
        assert_eq!(
            response.token_usage(),
            Some(crate::domain::agent::ModelTokenUsage {
                input_tokens: 11,
                output_tokens: 3,
                total_tokens: 14,
                cached_input_tokens: None,
                reasoning_tokens: None,
            })
        );
        assert!(response.assistant.text_content().contains("Need a lookup."));
        assert_eq!(
            response.tool_calls(),
            vec![ToolCall {
                call_id: Some("call_chat_1".into()),
                tool_name: "lookup".into(),
                input: json!({ "query": "status" }),
                input_error: None,
            }]
        );
        assert_eq!(
            seen.lock()
                .expect("stream event collection mutex poisoned")
                .as_slice(),
            &[ModelStreamEvent::reasoning_summary_delta("Need a lookup. ")]
        );
    }

    fn responses_model_config() -> AgentModelConfig {
        AgentModelConfig {
            model_code: "MODEL-TEST".into(),
            protocol: AgentModelProtocol::Responses,
            base_url: "https://example.com/v1".into(),
            api_path: Some("/responses".into()),
            api_key: "test-key".into(),
            model_name: "qwen3.7-max".into(),
            thinking_enabled: true,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(4096),
            context_window_tokens: None,
            top_p: None,
            max_retries: None,
        }
    }

    fn chat_model_config() -> AgentModelConfig {
        AgentModelConfig {
            model_code: "MODEL-CHAT-TEST".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "https://example.com/v1".into(),
            api_path: Some("/chat/completions".into()),
            api_key: "test-key".into(),
            model_name: "qwen3.7-max".into(),
            thinking_enabled: true,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(4096),
            context_window_tokens: None,
            top_p: None,
            max_retries: None,
        }
    }

    fn minimal_request() -> ModelRequest {
        ModelRequest {
            messages: vec![AgentMessage::user("测试多个工具并行调用")],
            tools: Vec::new(),
            system_agent_session_id: None,
            prepared_tool_schema: None,
            response_input_items: None,
            previous_response_id: None,
        }
    }
}
