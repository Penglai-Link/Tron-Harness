use std::{future::Future, pin::Pin};

use serde::Serialize;
use serde_json::Value;

use crate::domain::agent::{AgentRunContext, AgentRunResult, ModelStreamEvent};

use super::{
    AgentInvocationError, AgentRuntime, ModelStreamSink, RuntimeAdapterKey, RuntimeCapabilities,
};

#[derive(Clone)]
pub(super) struct ServiceModeRuntime {
    key: RuntimeAdapterKey,
    capabilities: RuntimeCapabilities,
    endpoint: String,
    working_directory: Option<String>,
    client: reqwest::Client,
}

impl ServiceModeRuntime {
    pub(super) fn new(
        key: RuntimeAdapterKey,
        capabilities: RuntimeCapabilities,
        endpoint: String,
        working_directory: Option<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            key,
            capabilities,
            endpoint,
            working_directory,
            client,
        }
    }

    fn run<'a>(
        &'a self,
        action: &'static str,
        requested_session_id: Option<String>,
        mut context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Some(session_id) = requested_session_id.as_ref() {
                context.system_agent_session_id = Some(session_id.clone());
            }
            if context.variables.iter().any(|variable| variable.sensitive) {
                return Err(AgentInvocationError::RuntimeSecretDeliveryUntrusted);
            }
            let endpoint = normalized_endpoint(&self.endpoint).ok_or_else(|| {
                AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` endpoint is required",
                    self.key.code()
                ))
            })?;

            let request_path = match action {
                "start" => "runs/start",
                "resume" => "runs/resume",
                _ => {
                    return Err(AgentInvocationError::RuntimeFailed(format!(
                        "runtime adapter `{}` does not support action `{action}`",
                        self.key.code()
                    )));
                }
            };
            let url = join_endpoint(&endpoint, request_path);
            let body = ServiceModeRunRequest {
                action: action.into(),
                requested_session_id,
                working_directory: self.working_directory.clone(),
                context,
            };

            let response = self
                .client
                .post(url)
                .json(&body)
                .send()
                .await
                .map_err(|error| {
                    AgentInvocationError::RuntimeFailed(format!(
                        "runtime adapter `{}` request failed: {error}",
                        self.key.code()
                    ))
                })?;
            let status = response.status();
            let response_json = response.json::<Value>().await.map_err(|error| {
                AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` response parse failed: {error}",
                    self.key.code()
                ))
            })?;
            if !status.is_success() {
                return Err(AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` returned status {} with payload {}",
                    self.key.code(),
                    status.as_u16(),
                    compact_payload(&response_json)
                )));
            }
            let payload = response_json.get("data").cloned().unwrap_or(response_json);
            let run = serde_json::from_value::<AgentRunResult>(payload).map_err(|error| {
                AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` run decode failed: {error}",
                    self.key.code()
                ))
            })?;
            if let Some(sink) = sink
                && !run.final_output.is_empty()
            {
                for chunk in split_stream_chunks(&run.final_output, 72) {
                    sink(ModelStreamEvent::text_delta(chunk));
                }
            }
            Ok(run)
        })
    }

    fn cancel_request<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        Box::pin(async move {
            let endpoint = normalized_endpoint(&self.endpoint).ok_or_else(|| {
                AgentInvocationError::RuntimeFailed(format!(
                    "runtime adapter `{}` endpoint is required",
                    self.key.code()
                ))
            })?;
            let url = join_endpoint(&endpoint, &format!("runs/{run_code}/cancel"));
            let response = self
                .client
                .post(url)
                .json(&serde_json::json!({ "runCode": run_code }))
                .send()
                .await
                .map_err(|error| {
                    AgentInvocationError::RuntimeFailed(format!(
                        "runtime adapter `{}` cancel request failed: {error}",
                        self.key.code()
                    ))
                })?;
            if response.status().is_success() {
                return Ok(());
            }
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(AgentInvocationError::RuntimeFailed(format!(
                "runtime adapter `{}` cancel returned status {} with body {}",
                self.key.code(),
                status.as_u16(),
                body.trim()
            )))
        })
    }
}

impl AgentRuntime for ServiceModeRuntime {
    fn key(&self) -> RuntimeAdapterKey {
        self.key
    }

    fn capabilities(&self) -> RuntimeCapabilities {
        self.capabilities
    }

    fn start<'a>(
        &'a self,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.run("start", None, context, sink)
    }

    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        context: AgentRunContext,
        sink: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<AgentRunResult, AgentInvocationError>> + Send + 'a>>
    {
        self.run("resume", Some(session_id.to_string()), context, sink)
    }

    fn cancel<'a>(
        &'a self,
        run_code: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentInvocationError>> + Send + 'a>> {
        self.cancel_request(run_code)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceModeRunRequest {
    action: String,
    requested_session_id: Option<String>,
    working_directory: Option<String>,
    context: AgentRunContext,
}

fn normalized_endpoint(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed)
}

fn join_endpoint(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn compact_payload(value: &Value) -> String {
    const LIMIT: usize = 192;
    let serialized = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    if serialized.len() <= LIMIT {
        return serialized;
    }
    format!("{}...", &serialized[..LIMIT.saturating_sub(3)])
}

fn split_stream_chunks(output: &str, chunk_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in output.chars() {
        current.push(ch);
        if current.chars().count() >= chunk_chars {
            chunks.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}
