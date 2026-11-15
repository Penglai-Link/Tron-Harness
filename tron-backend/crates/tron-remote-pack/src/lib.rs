//! Tron product pack for the shared remote RPC layer.
//!
//! The pack intentionally talks to the existing loopback HTTP API. This keeps
//! business handlers, profile middleware and streaming semantics exactly as
//! they already are; the remote protocol only changes how the phone reaches
//! the listener.

use async_trait::async_trait;
use eliteforge_agent_remote_rpc::{
    EventEmitter, Message, RemotePack, RemotePrincipal, RpcError, RpcMethod,
    RpcRegistry, SendMessageResult, SessionCreateParams, SessionHistoryParams, SessionHistoryResult,
    SessionListParams, SessionListResult, SessionRpc, SessionSendMessageParams,
    SessionStopRunParams, SessionSummary, SessionUnwatchEventsParams, SessionWatchEventsParams,
    StopRunResult, Subscription,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::broadcast;

const TRON_CODE: &str = "tron";
const TRON_NAME: &str = "Tron";

#[derive(Clone)]
pub struct TronRemotePack {
    http_base: String,
    client: reqwest::Client,
    agui: Option<Arc<dyn AguiEventSource>>,
}

/// Product-side bridge from the remote pack to Tron's AG-UI presentation
/// service. Tron implements this; the pack does not depend on Tron crates.
#[async_trait]
pub trait AguiEventSource: Send + Sync {
    async fn replay(&self, session_id: &str, cursor: Option<u64>) -> Result<Vec<Value>, String>;
    fn subscribe(&self, session_id: &str) -> Result<AguiLive, String>;
}

pub struct AguiLive {
    pub receiver: broadcast::Receiver<Value>,
}

impl TronRemotePack {
    pub fn new(http_base: impl Into<String>, agui: Option<Arc<dyn AguiEventSource>>) -> Self {
        Self {
            http_base: http_base.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            agui,
        }
    }

    async fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, RpcError> {
        let response = self
            .client
            .get(format!("{}{}", self.http_base, path))
            .query(query)
            .header("Accept-Language", "zh-CN")
            .send()
            .await
            .map_err(|e| RpcError::internal(format!("tron request failed: {e}")))?;
        let envelope: Value = response
            .json()
            .await
            .map_err(|e| RpcError::internal(format!("tron response invalid: {e}")))?;
        unwrap_envelope(envelope)
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, RpcError> {
        let response = self
            .client
            .post(format!("{}{}", self.http_base, path))
            .header("Accept-Language", "zh-CN")
            .json(&body)
            .send()
            .await
            .map_err(|e| RpcError::internal(format!("tron request failed: {e}")))?;
        let envelope: Value = response
            .json()
            .await
            .map_err(|e| RpcError::internal(format!("tron response invalid: {e}")))?;
        unwrap_envelope(envelope)
    }

    async fn post_query(&self, path: &str, query: &[(&str, &str)], body: Value) -> Result<Value, RpcError> {
        let response = self
            .client
            .post(format!("{}{}", self.http_base, path))
            .query(query)
            .header("Accept-Language", "zh-CN")
            .json(&body)
            .send()
            .await
            .map_err(|e| RpcError::internal(format!("tron request failed: {e}")))?;
        let envelope: Value = response
            .json()
            .await
            .map_err(|e| RpcError::internal(format!("tron response invalid: {e}")))?;
        unwrap_envelope(envelope)
    }

    async fn default_workspace(&self) -> Result<String, RpcError> {
        let data = self.get("/api/v1/admin/workspaces/options", &[]).await?;
        let list = data
            .as_array()
            .cloned()
            .or_else(|| data.get("options").and_then(Value::as_array).cloned())
            .unwrap_or_default();
        for item in &list {
            if let Some(code) = item
                .get("workspaceCode")
                .or_else(|| item.get("code"))
                .and_then(Value::as_str)
            {
                return Ok(code.to_owned());
            }
        }
        Err(RpcError::internal("no workspace is available"))
    }

    async fn workspace_for(&self, params_workspace: Option<&str>) -> Result<String, RpcError> {
        match params_workspace.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => Ok(value.to_owned()),
            None => self.default_workspace().await,
        }
    }

    fn session_from_value(item: &Value, workspace: &str) -> Option<SessionSummary> {
        let id = item
            .get("sessionCode")
            .or_else(|| item.get("id"))
            .and_then(Value::as_str)?
            .to_owned();
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Chat")
            .to_owned();
        let status = item
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("ACTIVE")
            .to_owned();
        let updated_at = item
            .get("updatedAt")
            .cloned()
            .unwrap_or_else(|| json!(0));
        Some(SessionSummary {
            id,
            product_code: TRON_CODE.into(),
            title,
            updated_at,
            status,
            workspace: Some(workspace.into()),
            metadata: Some(json!({
                "sourceType": item.get("sourceType").cloned().unwrap_or(Value::Null),
                "dudeCode": item.get("agentCode").cloned().unwrap_or(Value::Null),
                "projectCode": item.get("projectCode").cloned().unwrap_or(Value::Null),
                "directoryCode": item.get("directoryCode").cloned().unwrap_or(Value::Null),
            })),
        })
    }

    fn message_from_value(item: &Value) -> Option<Message> {
        let id = item.get("messageCode").cloned()?;
        let role = item
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("ASSISTANT")
            .to_lowercase();
        Some(Message {
            id,
            role,
            content: item
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            created_at: item.get("createdAt").cloned().unwrap_or(Value::Null),
            run_id: item
                .get("runCode")
                .and_then(Value::as_str)
                .map(str::to_owned),
            metadata: Some(item.clone()),
        })
    }
}

#[async_trait]
impl SessionRpc for TronRemotePack {
    async fn list(&self, _ctx: &RemotePrincipal, params: SessionListParams) -> Result<SessionListResult, RpcError> {
        let workspace = self
            .workspace_for(params.filter.as_ref().and_then(|v| v.get("workspace")).and_then(Value::as_str))
            .await?;
        let data = self
            .get("/api/v1/chats/session-groups", &[("workspaceCode", &workspace)])
            .await?;
        let groups = data.as_array().cloned().unwrap_or_default();
        let mut sessions = Vec::new();
        for group in groups {
            if let Some(items) = group.get("sessions").and_then(Value::as_array) {
                for item in items {
                    if let Some(session) = Self::session_from_value(item, &workspace) {
                        sessions.push(session);
                    }
                }
            }
        }
        let limit = params.limit.unwrap_or(50).clamp(1, 100) as usize;
        sessions.truncate(limit);
        Ok(SessionListResult {
            sessions,
            next_cursor: None,
        })
    }

    async fn history(&self, _ctx: &RemotePrincipal, params: SessionHistoryParams) -> Result<SessionHistoryResult, RpcError> {
        let workspace = self.default_workspace().await?;
        let data = self
            .get(
                &format!("/api/v1/chats/sessions/{}/messages", params.session_id),
                &[("workspaceCode", &workspace)],
            )
            .await?;
        let items = data
            .as_array()
            .cloned()
            .or_else(|| data.get("messages").and_then(Value::as_array).cloned())
            .unwrap_or_default();
        let messages = items.iter().filter_map(Self::message_from_value).collect();
        Ok(SessionHistoryResult {
            messages,
            next_cursor: None,
        })
    }

    async fn create(&self, _ctx: &RemotePrincipal, params: SessionCreateParams) -> Result<SessionSummary, RpcError> {
        let workspace = self.workspace_for(params.workspace.as_deref()).await?;
        let mut body = json!({ "workspaceCode": workspace });
        if let Some(title) = params.title {
            body["title"] = json!(title);
        }
        let data = self.post("/api/v1/chats/sessions", body).await?;
        Self::session_from_value(&data, &workspace)
            .ok_or_else(|| RpcError::internal("tron did not return a session"))
    }

    async fn send_message(
        &self,
        _ctx: &RemotePrincipal,
        params: SessionSendMessageParams,
        events: EventEmitter,
    ) -> Result<SendMessageResult, RpcError> {
        let workspace = self.default_workspace().await?;
        let mut body = json!({
            "content": params.content,
            "workspaceCode": workspace,
            "clientCapabilities": params.client_capabilities.unwrap_or_else(|| json!({"userInputRequests": false})),
        });
        if let Some(attachments) = params.attachments {
            body["attachments"] = attachments;
        }
        if let Some(metadata) = params.metadata {
            body["a2uiMetadata"] = metadata;
        }
        let response = self
            .client
            .post(format!(
                "{}/api/v1/chats/sessions/{}/messages/stream",
                self.http_base, params.session_id
            ))
            .query(&[("workspaceCode", workspace.as_str())])
            .header("Accept-Language", "zh-CN")
            .json(&body)
            .send()
            .await
            .map_err(|e| RpcError::internal(format!("tron stream failed: {e}")))?;
        if !response.status().is_success() {
            return Err(RpcError::internal(format!(
                "tron stream returned {}",
                response.status()
            )));
        }
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut run_id = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| RpcError::internal(format!("stream read failed: {e}")))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find("\n\n") {
                let frame = buffer[..index].to_owned();
                buffer = buffer[index + 2..].to_owned();
                let data = frame
                    .lines()
                    .filter_map(|line| line.strip_prefix("data:"))
                    .map(str::trim_start)
                    .collect::<Vec<_>>()
                    .join("\n");
                if data.is_empty() {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<Value>(&data) {
                    if run_id.is_empty() {
                        run_id = value
                            .get("runId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                    }
                    events.emit(value)?;
                }
            }
            if events.is_cancelled() {
                break;
            }
        }
        if run_id.is_empty() {
            run_id = format!("run-{}", eliteforge_agent_remote_protocol::now_ms());
        }
        Ok(SendMessageResult { run_id })
    }

    async fn stop_run(&self, _ctx: &RemotePrincipal, params: SessionStopRunParams) -> Result<StopRunResult, RpcError> {
        let body = json!({
            "sessionCode": params.session_id,
            "expectedRunCode": params.run_id.unwrap_or_default(),
        });
        let _ = self.post("/api/v1/chats/sessions/stop-run", body).await?;
        Ok(StopRunResult { cancelled: true })
    }

    async fn watch_events(
        &self,
        _ctx: &RemotePrincipal,
        params: SessionWatchEventsParams,
        events: EventEmitter,
    ) -> Result<Subscription, RpcError> {
        let source = self
            .agui
            .as_ref()
            .ok_or_else(|| RpcError::unsupported("AG-UI event source is not connected"))?
            .clone();
        for event in source
            .replay(&params.session_id, params.cursor)
            .await
            .map_err(RpcError::internal)?
        {
            events.emit(event)?;
        }
        let mut live = source
            .subscribe(&params.session_id)
            .map_err(RpcError::internal)?;
        let cancelled = events.cancelled();
        loop {
            tokio::select! {
                _ = cancelled.cancelled() => break,
                received = live.receiver.recv() => match received {
                    Ok(event) => { events.emit(event)?; }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        for event in source.replay(&params.session_id, params.cursor).await.map_err(RpcError::internal)? {
                            events.emit(event)?;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
        Ok(Subscription {
            subscription_id: format!("sub-{}", eliteforge_agent_remote_protocol::now_ms()),
        })
    }

    async fn unwatch_events(&self, _ctx: &RemotePrincipal, _params: SessionUnwatchEventsParams) -> Result<(), RpcError> {
        Ok(())
    }
}

struct TronMethod {
    pack: TronRemotePack,
    method: String,
}

#[async_trait]
impl RpcMethod for TronMethod {
    async fn call(&self, _ctx: &RemotePrincipal, params: Value) -> Result<Value, RpcError> {
        let workspace = params
            .get("workspaceCode")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let dude_code = params
            .get("dudeCode")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        match self.method.as_str() {
            "tron.workspaces.list" => self.pack.get("/api/v1/admin/workspaces/options", &[]).await,
            "tron.dudes.list" => {
                self.pack
                    .get("/api/v1/dudes", &[("workspaceCode", workspace.as_str())])
                    .await
            }
            "tron.dudes.mainSession" => {
                self.pack
                    .post_query(
                        "/api/v1/dudes/main/session",
                        &[("workspaceCode", workspace.as_str())],
                        json!({}),
                    )
                    .await
            }
            "tron.dudes.createSession" => {
                self.pack
                    .post_query(
                        &format!("/api/v1/dudes/{dude_code}/sessions"),
                        &[("workspaceCode", workspace.as_str())],
                        json!({}),
                    )
                    .await
            }
            "tron.projects.list" => self.pack.post("/api/v1/projects/list", params).await,
            "tron.directories.list" => {
                self.pack
                    .get("/api/v1/directories", &[("workspaceCode", workspace.as_str())])
                    .await
            }
            "tron.session.groups" => {
                self.pack
                    .get(
                        "/api/v1/chats/session-groups",
                        &[("workspaceCode", workspace.as_str())],
                    )
                    .await
            }
            "tron.skins.list" => self.pack.get("/api/v1/system/skins", &[]).await,
            "tron.skins.active" => self.pack.get("/api/v1/system/skins/active", &[]).await,
            other => Err(RpcError::unsupported(format!("{other} is not implemented"))),
        }
    }
}

impl RemotePack for TronRemotePack {
    fn session_rpc(&self) -> Option<Arc<dyn SessionRpc>> {
        Some(Arc::new(self.clone()))
    }

    fn register(&self, registry: &mut RpcRegistry) {
        for method in [
            "tron.workspaces.list",
            "tron.dudes.list",
            "tron.dudes.mainSession",
            "tron.dudes.createSession",
            "tron.projects.list",
            "tron.directories.list",
            "tron.session.groups",
            "tron.skins.list",
            "tron.skins.active",
        ] {
            registry.register(
                method,
                Arc::new(TronMethod {
                    pack: self.clone(),
                    method: method.to_owned(),
                }),
            );
        }
    }
}

fn unwrap_envelope(value: Value) -> Result<Value, RpcError> {
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("SUCCESS");
    if code != "SUCCESS" && code != "0" {
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("tron request failed");
        return Err(RpcError::internal(message.to_owned()));
    }
    Ok(value.get("data").cloned().unwrap_or(Value::Null))
}

pub fn product_identity() -> eliteforge_agent_remote_protocol::ProductIdentity {
    eliteforge_agent_remote_protocol::ProductIdentity {
        product_code: TRON_CODE.into(),
        product_name: TRON_NAME.into(),
        node_id: String::new(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        machine_name: String::new(),
    }
}

