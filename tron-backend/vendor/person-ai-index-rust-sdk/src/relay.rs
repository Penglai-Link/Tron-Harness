//! 消息中继 WebSocket 客户端
//!
//! 桌面端通过 WebSocket 连接服务端 `/api/v1/relay/ws`，接收手机端下发的任务指令
//! （quest.new / tool_confirm / quest.cancel / workspaces.list / quests.list），
//! 并上行执行事件（chat.event / quest.done）与查询响应。
//! 服务端会在手机端上线/离线时推送 `client.online` / `client.offline`，
//! 桌面端应在收到 `client.online` 后再发送 register 声明上线。
//!
//! 鉴权采用握手头：`Authorization: Bearer <token>` + `X-Client-Id: <client_id>`。

use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::{Result, ServerApiError};

// ─── WebSocket 流类型别名 ──────────────────────────────────

/// Relay WebSocket 流类型
pub type RelayStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// 写入半流
pub type RelaySink =
    futures_util::stream::SplitSink<RelayStream, Message>;

/// 读取半流
pub type RelayReader = futures_util::stream::SplitStream<RelayStream>;

/// 拆分 WebSocket 流为读写两半
pub fn split_stream(stream: RelayStream) -> (RelaySink, RelayReader) {
    use futures_util::StreamExt;
    stream.split()
}

/// 写入器循环：从 mpsc 接收 JSON 字符串并发送到 WebSocket
pub async fn run_writer(
    mut sink: RelaySink,
    mut rx: tokio::sync::mpsc::Receiver<String>,
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::sync::atomic::Ordering;
    while let Some(json) = rx.recv().await {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        if sink.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}

// ─── 出站消息载荷 ──────────────────────────────────────────

/// register 消息中的智能体信息
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgent {
    pub agent_id: String,
    pub name: String,
}

/// 构建 register 消息 JSON
pub fn build_register(device_name: &str, agents: &[(String, String)]) -> String {
    let agents_v: Vec<serde_json::Value> = agents
        .iter()
        .map(|(id, name)| serde_json::json!({ "agentId": id, "name": name }))
        .collect();
    let msg = serde_json::json!({
        "type": "register",
        "deviceName": device_name,
        "agents": agents_v,
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

/// 构建 chat.event 消息 JSON（直接使用手机端期望的类型名，服务端纯透传）
pub fn build_chat_event(quest_id: &str, seq: u64, event: &serde_json::Value) -> String {
    let msg = serde_json::json!({
        "type": "chat.event",
        "questId": quest_id,
        "seq": seq,
        "event": event,
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

/// 构建 chat.send.result 响应 JSON（回复移动端 chat.send，携带 questId）
pub fn build_chat_send_result(client_msg_id: &str, quest_id: &str) -> String {
    let msg = serde_json::json!({
        "type": "chat.send.result",
        "clientMsgId": client_msg_id,
        "questId": quest_id,
        "status": "running",
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

/// 构建 quest.done 消息 JSON
pub fn build_quest_done(quest_id: &str, status: &str, error: Option<&str>) -> String {
    let mut msg = serde_json::json!({
        "type": "quest.done",
        "questId": quest_id,
        "status": status,
    });
    if let Some(e) = error {
        msg["error"] = serde_json::Value::String(e.to_string());
    }
    serde_json::to_string(&msg).unwrap_or_default()
}

/// 构建 ping 心跳消息 JSON（客户端定期发送，服务端原样回 pong，不转发）
pub fn build_ping() -> String {
    serde_json::json!({ "type": "ping" }).to_string()
}

/// workspaces.list.result 中的单项
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub workspace_id: String,
    pub name: String,
    pub agent_id: String,
    pub agent_name: String,
}

/// 构建 workspaces.list.result 消息 JSON
pub fn build_workspaces_list_result(req_id: &str, workspaces: &[WorkspaceSummary]) -> String {
    let msg = serde_json::json!({
        "type": "workspaces.list.result",
        "reqId": req_id,
        "workspaces": workspaces,
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

/// quests.list.result 中的单项
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestSummary {
    pub quest_id: String,
    pub title: String,
    pub status: String,
    pub workspace_id: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 构建 quests.list.result 消息 JSON
pub fn build_quests_list_result(req_id: &str, quests: &[QuestSummary]) -> String {
    let msg = serde_json::json!({
        "type": "quests.list.result",
        "reqId": req_id,
        "quests": quests,
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

/// 构建 quest.messages.result 消息 JSON
pub fn build_quest_messages_result(
    req_id: &str,
    quest_id: &str,
    messages: &[QuestMessageItem],
    has_more: bool,
) -> String {
    let msg = serde_json::json!({
        "type": "quest.messages.result",
        "reqId": req_id,
        "questId": quest_id,
        "messages": messages,
        "hasMore": has_more,
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

// ─── 入站消息载荷 ──────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestNewMessage {
    pub quest_id: String,
    pub agent_id: String,
    pub content: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfirmMessage {
    pub client_id: Option<String>,
    pub quest_id: String,
    pub tool_call_id: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacesListMessage {
    pub req_id: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestsListMessage {
    pub req_id: String,
    pub agent_id: Option<String>,
    pub workspace_id: Option<String>,
    pub limit: Option<i32>,
    pub before: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestMessagesMessage {
    pub req_id: String,
    pub agent_id: Option<String>,
    pub quest_id: String,
    pub limit: Option<i32>,
    pub page: Option<i32>,
    /// 游标：只返回 sequence_no 小于该值的事件（更旧的历史），避免 offset 分页漂移
    pub before: Option<i64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendMessage {
    pub client_msg_id: String,
    pub client_id: String,
    pub agent_id: String,
    pub content: String,
    pub quest_id: Option<String>,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserAnswerItem {
    pub tool_call_id: String,
    pub answer: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserAnswerMessage {
    pub client_id: String,
    pub quest_id: String,
    pub agent_id: String,
    pub workspace_id: Option<String>,
    pub answers: Vec<AskUserAnswerItem>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestMessageItem {
    pub event_type: String,
    pub payload: String,
    /// 事件序号（event_record.sequence_no），移动端按 seq 去重与游标分页
    pub seq: i64,
}

// ─── 事件监听器 ────────────────────────────────────────────

/// Relay 事件监听器
///
/// 所有方法都有默认空实现，按需覆盖。
#[async_trait::async_trait]
pub trait RelayListener: Send + Sync {
    /// 收到 `quest.new` —— 新任务或续接任务
    async fn on_quest_new(&self, _msg: QuestNewMessage) {}

    /// 收到 `tool_confirm` —— 工具权限确认结果
    async fn on_tool_confirm(&self, _msg: ToolConfirmMessage) {}

    /// 收到 `quest.cancel` —— 取消任务
    async fn on_quest_cancel(&self, _quest_id: String) {}

    /// 收到 `workspaces.list` —— 工作空间查询请求
    async fn on_workspaces_list(&self, _msg: WorkspacesListMessage) {}

    /// 收到 `quests.list` —— 任务历史查询请求
    async fn on_quests_list(&self, _msg: QuestsListMessage) {}

    /// 收到 `quest.messages` —— 任务聊天历史查询请求
    async fn on_quest_messages(&self, _msg: QuestMessagesMessage) {}

    /// 收到 `chat.send` —— 移动端发送聊天消息（转发给桌面端处理）
    async fn on_chat_send(&self, _msg: ChatSendMessage) {}

    /// 收到 `chat.askUser.answer` —— 移动端回答 askUser 工具
    async fn on_ask_user_answer(&self, _msg: AskUserAnswerMessage) {}

    /// 收到 `pong` —— 心跳响应（服务端原样回，不转发）
    async fn on_pong(&self) {}

    /// 收到 `client.online` —— 移动端上线（服务端推送）
    async fn on_client_online(&self, _client_id: String) {}

    /// 收到 `client.offline` —— 移动端离线（服务端推送）
    async fn on_client_offline(&self, _client_id: String) {}

    /// WebSocket 正常关闭
    async fn on_closed(&self, _code: u16, _reason: String) {}

    /// WebSocket 异常
    async fn on_failure(&self, _error: String) {}
}

/// 接收并分发消息（阻塞直到连接关闭）
///
/// 读取流的每一帧，按 `type` 字段分发到 listener 的对应方法。
pub async fn relay_listen(
    mut stream: futures_util::stream::SplitStream<RelayStream>,
    listener: &dyn RelayListener,
) -> Result<()> {
    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                let message: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("[relay] 无法解析消息: {e}, raw={text}");
                        continue;
                    }
                };

                let msg_type = message
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                match msg_type {
                    "quest.new" => {
                        if let Ok(m) = serde_json::from_value::<QuestNewMessage>(message.clone()) {
                            listener.on_quest_new(m).await;
                        }
                    }
                    "tool_confirm" => {
                        if let Ok(m) = serde_json::from_value::<ToolConfirmMessage>(message.clone()) {
                            listener.on_tool_confirm(m).await;
                        }
                    }
                    "quest.cancel" => {
                        let quest_id = message
                            .get("questId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !quest_id.is_empty() {
                            listener.on_quest_cancel(quest_id).await;
                        }
                    }
                    "workspaces.list" => {
                        if let Ok(m) = serde_json::from_value::<WorkspacesListMessage>(message.clone())
                        {
                            listener.on_workspaces_list(m).await;
                        }
                    }
                    "quests.list" => {
                        if let Ok(m) = serde_json::from_value::<QuestsListMessage>(message.clone()) {
                            listener.on_quests_list(m).await;
                        }
                    }
                    "quest.messages" => {
                        if let Ok(m) = serde_json::from_value::<QuestMessagesMessage>(message.clone()) {
                            listener.on_quest_messages(m).await;
                        }
                    }
                    "chat.send" => {
                        if let Ok(m) = serde_json::from_value::<ChatSendMessage>(message.clone()) {
                            listener.on_chat_send(m).await;
                        }
                    }
                    "chat.askUser.answer" => {
                        if let Ok(m) = serde_json::from_value::<AskUserAnswerMessage>(message.clone()) {
                            listener.on_ask_user_answer(m).await;
                        }
                    }
                    "pong" => {
                        listener.on_pong().await;
                    }
                    "client.online" => {
                        let client_id = message
                            .get("clientId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        listener.on_client_online(client_id).await;
                    }
                    "client.offline" => {
                        let client_id = message
                            .get("clientId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        listener.on_client_offline(client_id).await;
                    }
                    _ => {
                        tracing::debug!("[relay] 忽略未知消息类型: {msg_type}");
                    }
                }
            }
            Ok(Message::Close(frame)) => {
                let (code, reason) = match frame {
                    Some(f) => (f.code.into(), f.reason.to_string()),
                    None => (1000, String::new()),
                };
                listener.on_closed(code, reason).await;
                break;
            }
            Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Err(e) => {
                listener.on_failure(e.to_string()).await;
                return Err(ServerApiError::WebSocket(format!("WebSocket 错误: {e}")));
            }
            _ => {}
        }
    }

    Ok(())
}

// ─── WebSocket 客户端 ──────────────────────────────────────

/// Relay WebSocket 客户端
pub struct RelayClient {
    api: AuthorizedClientApi,
}

/// 从 JWT token 解析 subject（userId），用于 WS 连接的 Sticky Session 路由
fn decode_jwt_subject(token: &str) -> Option<String> {
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    payload.get("sub")?.as_str().map(|s| s.to_string())
}

impl RelayClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 建立 WebSocket 连接（握手头携带 Authorization + X-Client-Id）
    ///
    /// 返回 WebSocket 流，调用方自行 split 为读写两半。
    pub async fn connect(&self, context: &AuthorizedClientContext) -> Result<RelayStream> {
        let token = context
            .token
            .as_ref()
            .ok_or_else(|| ServerApiError::Validation("缺少 token".into()))?;
        let client_id = context
            .client_id
            .as_ref()
            .ok_or_else(|| ServerApiError::Validation("缺少 clientId".into()))?;

        let mut params = HashMap::<String, String>::new();
        if let Some(user_id) = decode_jwt_subject(token) {
            params.insert("userId".to_string(), user_id);
        }
        let ws_url = self
            .api
            .build_websocket_url("/relay/ws", &params)?;

        tracing::info!("[relay] 连接 WebSocket: {ws_url}");

        let mut request = ws_url
            .into_client_request()
            .map_err(|e| ServerApiError::WebSocket(format!("构造请求失败: {e}")))?;
        let headers = request.headers_mut();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().map_err(|e| {
                ServerApiError::Other(format!("构造 Authorization 头失败: {e}"))
            })?,
        );
        headers.insert(
            "X-Client-Id",
            client_id.parse().map_err(|e| {
                ServerApiError::Other(format!("构造 X-Client-Id 头失败: {e}"))
            })?,
        );

        let (ws_stream, response) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| ServerApiError::WebSocket(format!("连接失败: {e}")))?;

        tracing::debug!("[relay] WebSocket 连接响应: {response:?}");

        Ok(ws_stream)
    }
}

/// 等待写入器任务结束的辅助：向 SplitSink 发送关闭帧
pub async fn close_sink(
    sink: &mut futures_util::stream::SplitSink<RelayStream, Message>,
) -> Result<()> {
    sink.send(Message::Close(Some(
        tokio_tungstenite::tungstenite::protocol::CloseFrame {
            code: 1000.into(),
            reason: "normal closure".into(),
        },
    )))
    .await
    .map_err(|e| ServerApiError::WebSocket(format!("关闭连接失败: {e}")))?;
    Ok(())
}
