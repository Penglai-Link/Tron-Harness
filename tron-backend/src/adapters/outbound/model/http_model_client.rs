use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use serde_json::Value;

use crate::{
    application::agent::{ModelClient, ModelStreamSink},
    domain::agent::{AgentModelConfig, ModelRequest, ModelResponse, ModelStreamEventKind},
};

use super::protocol_adapters::{ModelProtocolAdapter, adapter_for_protocol};

pub trait ProviderClient: Send + Sync {
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
    provider: Arc<dyn ProviderClient>,
}

impl Default for HttpModelClient {
    fn default() -> Self {
        Self {
            provider: Arc::new(ReqwestProviderClient::default()),
        }
    }
}

impl HttpModelClient {
    pub fn new(provider: Arc<dyn ProviderClient>) -> Self {
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
        let output = Arc::new(std::sync::Mutex::new(String::new()));
        let output_for_sink = output.clone();
        let token_usage = Arc::new(std::sync::Mutex::new(None));
        let token_usage_for_sink = token_usage.clone();
        let event_sink = Arc::new(move |event: Value| {
            if let Some(usage) = adapter.parse_usage(&event) {
                *token_usage_for_sink
                    .lock()
                    .expect("model stream usage mutex poisoned") = Some(usage);
            }
            if let Some(delta) = adapter.parse_stream_event(&event)? {
                if delta.kind == ModelStreamEventKind::Text {
                    output_for_sink
                        .lock()
                        .expect("model stream output mutex poisoned")
                        .push_str(&delta.delta);
                }
                sink(delta);
            }
            Ok::<(), String>(())
        });
        self.provider
            .post_event_stream(config, &url, &body, event_sink)
            .await?;
        let output = output
            .lock()
            .expect("model stream output mutex poisoned")
            .clone();
        let token_usage = *token_usage
            .lock()
            .expect("model stream usage mutex poisoned");
        if output.is_empty() {
            return self.generate_with_adapter(config, request).await;
        }
        Ok(ModelResponse::final_output_with_usage(output, token_usage))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReqwestProviderClient {
    client: reqwest::Client,
}

impl ProviderClient for ReqwestProviderClient {
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

impl ReqwestProviderClient {
    fn request(
        &self,
        config: &AgentModelConfig,
        url: &str,
        body: &Value,
    ) -> reqwest::RequestBuilder {
        let mut request = self
            .client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(body);
        if let Some(timeout_ms) = config.timeout_ms
            && timeout_ms > 0
        {
            request = request.timeout(Duration::from_millis(timeout_ms));
        }
        request
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
        "model provider request timed out".into()
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
