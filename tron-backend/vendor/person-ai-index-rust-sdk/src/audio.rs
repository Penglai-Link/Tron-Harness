//! 音频转写客户端
//!
//! 对应 Java: `client/audio/` 包
//!
//! 提供两种模式：
//! - HTTP 文件转写：一次性上传音频文件，同步返回完整文本（见 `AudioFileTranscriptionClient`）
//! - WebSocket 实时转录：流式推送音频分片，实时返回增量文本（所有业务消息加密后发送，见 `AudioTranscriptionsClient`）

use std::collections::HashMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use reqwest::Method;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::crypto::{ClientEncryptedEnvelope, ClientEnvelopeEncryptor};
use crate::error::{Result, ServerApiError};

// ─── 事件监听器 ────────────────────────────────────────────

/// 音频转写事件监听器
///
/// 对应 Java: `AudioTranscriptionListener`
///
/// 所有方法都有默认空实现，按需覆盖
#[async_trait::async_trait]
pub trait AudioTranscriptionListener: Send + Sync {
    /// WebSocket 连接已打开
    async fn on_open(&self) {}

    /// 收到 `type == "connected"` 消息
    async fn on_connected(&self) {}

    /// 收到 `type == "started"` 消息
    async fn on_started(&self, _message: serde_json::Value) {}

    /// 收到 `type == "realtime_event"` 消息的 event 字段
    async fn on_realtime_event(&self, _event: serde_json::Value) {}

    /// 收到其他消息
    async fn on_message(&self, _message: serde_json::Value) {}

    /// 收到 `type == "error"` 消息
    async fn on_error(&self, _message: &str, _payload: serde_json::Value) {}

    /// WebSocket 正常关闭
    async fn on_closed(&self, _code: u16, _reason: &str) {}

    /// WebSocket 异常
    async fn on_failure(&self, _error: &str) {}
}

// ─── WebSocket 会话 ────────────────────────────────────────

/// WebSocket 会话
///
/// 对应 Java: `AudioTranscriptionsWebSocketSession`
pub struct AudioTranscriptionsSession {
    ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    public_key: String,
}

impl AudioTranscriptionsSession {
    /// 发送开始指令
    ///
    /// 发送加密后的: `{"type":"start","language":"...","target_language":"...","prompt":"...","vad":true/false,"delay":"..."}`
    pub async fn start(
        &mut self,
        language: &str,
        target_language: Option<&str>,
        prompt: Option<&str>,
        vad: bool,
        delay: Option<&str>,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "type": "start",
            "language": language,
            "target_language": target_language,
            "prompt": prompt,
            "vad": vad,
            "delay": delay,
        });
        self.send_encrypted(&payload).await
    }

    /// 发送音频数据 (Base64 编码)
    ///
    /// 发送加密后的: `{"type":"audio","audio":"base64..."}`
    pub async fn send_audio(&mut self, base64_audio: &str) -> Result<()> {
        let payload = serde_json::json!({
            "type": "audio",
            "audio": base64_audio,
        });
        self.send_encrypted(&payload).await
    }

    /// 发送提交指令
    ///
    /// 发送加密后的: `{"type":"commit"}`
    pub async fn commit(&mut self) -> Result<()> {
        let payload = serde_json::json!({"type": "commit"});
        self.send_encrypted(&payload).await
    }

    /// 发送停止指令
    ///
    /// 发送加密后的: `{"type":"stop"}`
    pub async fn stop(&mut self) -> Result<()> {
        let payload = serde_json::json!({"type": "stop"});
        self.send_encrypted(&payload).await
    }

    /// 发送原始 JSON 消息 (会自动加密)
    pub async fn send_encrypted(&mut self, payload: &serde_json::Value) -> Result<()> {
        let raw_json = serde_json::to_string(payload)?;
        let envelope = ClientEnvelopeEncryptor::encrypt(&self.public_key, &raw_json)?;
        let envelope_json = serde_json::to_string(&envelope)?;
        self.ws_stream
            .send(Message::Text(envelope_json.into()))
            .await
            .map_err(|e| ServerApiError::WebSocket(format!("发送消息失败: {e}")))?;
        Ok(())
    }

    /// 直接发送已加密的信封
    pub async fn send_envelope(&mut self, envelope: &ClientEncryptedEnvelope) -> Result<()> {
        let json = serde_json::to_string(envelope)?;
        self.ws_stream
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| ServerApiError::WebSocket(format!("发送消息失败: {e}")))?;
        Ok(())
    }

    /// 关闭 WebSocket 连接
    pub async fn close(&mut self) -> Result<()> {
        self.ws_stream
            .send(Message::Close(Some(
                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code: 1000.into(),
                    reason: "normal closure".into(),
                },
            )))
            .await
            .map_err(|e| ServerApiError::WebSocket(format!("关闭连接失败: {e}")))?;
        Ok(())
    }

    /// 接收并分发消息 (阻塞直到连接关闭)
    ///
    /// 根据消息 `type` 字段分发到 listener 的对应方法
    pub async fn listen(&mut self, listener: &dyn AudioTranscriptionListener) -> Result<()> {
        listener.on_open().await;

        while let Some(msg_result) = self.ws_stream.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    let message: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    match msg_type {
                        "connected" => listener.on_connected().await,
                        "started" => listener.on_started(message).await,
                        "realtime_event" => {
                            let event = message.get("event").cloned().unwrap_or(serde_json::Value::Null);
                            listener.on_realtime_event(event).await;
                        }
                        "error" => {
                            let msg = message
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("未知错误")
                                .to_string();
                            listener.on_error(&msg, message).await;
                        }
                        _ => listener.on_message(message).await,
                    }
                }
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        listener
                            .on_closed(frame.code.into(), &frame.reason)
                            .await;
                    } else {
                        listener.on_closed(1000, "").await;
                    }
                    break;
                }
                Ok(Message::Binary(_)) => {
                    // 忽略二进制消息
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                    // 心跳消息自动处理
                }
                Err(e) => {
                    listener.on_failure(&e.to_string()).await;
                    return Err(ServerApiError::WebSocket(format!("WebSocket 错误: {e}")));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

// ─── WebSocket 客户端 ──────────────────────────────────────

/// 音频转写 WebSocket 客户端
///
/// 对应 Java: `AudioTranscriptionsWebSocketClient`
pub struct AudioTranscriptionsClient {
    api: AuthorizedClientApi,
}

impl AudioTranscriptionsClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 构建 WebSocket URL
    ///
    /// `ws(s)://{baseUrl}/api/v1/ws/audio/transcriptions?token={token}&client_id={clientId}`
    fn build_websocket_url(&self, context: &AuthorizedClientContext) -> Result<String> {
        let token = context
            .token
            .as_ref()
            .ok_or_else(|| ServerApiError::Validation("缺少 token".into()))?;
        let client_id = context
            .client_id
            .as_ref()
            .ok_or_else(|| ServerApiError::Validation("缺少 clientId".into()))?;

        let mut params = HashMap::new();
        params.insert("token".to_string(), token.clone());
        params.insert("client_id".to_string(), client_id.clone());

        self.api
            .build_websocket_url("/ws/audio/transcriptions", &params)
    }

    /// 建立 WebSocket 连接
    ///
    /// 返回 `(会话, 接收任务句柄)`
    ///
    /// 调用方应在异步任务中调用 `session.listen(listener)` 接收事件
    pub async fn connect(
        &self,
        context: &AuthorizedClientContext,
    ) -> Result<AudioTranscriptionsSession> {
        context.validate_gateway_access()?;

        let ws_url = self.build_websocket_url(context)?;
        tracing::info!("连接音频转写 WebSocket: {ws_url}");

        let (ws_stream, response) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| ServerApiError::WebSocket(format!("连接失败: {e}")))?;

        tracing::debug!("WebSocket 连接响应: {response:?}");

        let public_key = context
            .public_key
            .as_ref()
            .ok_or_else(|| ServerApiError::Validation("缺少 publicKey".into()))?
            .clone();

        Ok(AudioTranscriptionsSession {
            ws_stream,
            public_key,
        })
    }
}

// ─── 文件转写 HTTP 客户端 ─────────────────────────────────

/// 单文件大小上限（字节），与服务端 `spring.servlet.multipart.max-file-size` 保持一致。
///
/// 超过此大小的音频需由调用方切片分多次上传，再自行拼接结果。
pub const MAX_AUDIO_FILE_BYTES: usize = 50 * 1024 * 1024;

/// 音频文件转写 HTTP 客户端
///
/// 对应 `POST /api/v1/audio/transcriptions`（multipart/form-data，不加密，仅需用户 Token）。
///
/// 服务端从已启用的语音模型中随机选择一个执行转写，同步返回完整文本。
pub struct AudioFileTranscriptionClient {
    api: AuthorizedClientApi,
}

impl AudioFileTranscriptionClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 上传音频文件并转写为文本。
    ///
    /// - `file_name`：文件名（用于推断音频格式，如 `audio.mp3`）
    /// - `audio`：音频文件字节内容
    /// - `language`：可选语言代码（如 `zh`、`en`），辅助识别
    ///
    /// 单次上传大小上限为 [`MAX_AUDIO_FILE_BYTES`]（50MB），超过需自行切片分多次调用，
    /// 再将各次返回的文本按顺序拼接为完整结果。返回本次转写出的文本。
    pub async fn transcribe(
        &self,
        context: &AuthorizedClientContext,
        file_name: &str,
        audio: Vec<u8>,
        language: Option<&str>,
    ) -> Result<String> {
        let part = reqwest::multipart::Part::bytes(audio)
            .file_name(file_name.to_string())
            .mime_str(guess_mime(file_name))
            .map_err(|e| ServerApiError::Other(format!("音频转写: 设置 MIME 失败: {e}")))?;
        let mut form = reqwest::multipart::Form::new().part("file", part);
        if let Some(lang) = language {
            if !lang.is_empty() {
                form = form.text("language", lang.to_string());
            }
        }

        let request = self
            .api
            .new_authorized_request(Method::POST, "/audio/transcriptions", context)?
            .multipart(form);

        // 文件转写可能耗时较长，使用比默认 120s 更宽的超时
        let response = request
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| {
                ServerApiError::Other(format!("音频转写: 请求失败: {}", crate::base::format_error_chain(&e)))
            })?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ServerApiError::Other(format!(
                "音频转写: HTTP {status}, body: {body}"
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ServerApiError::Other(format!("音频转写: 反序列化失败: {e}")))?;
        Ok(json
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }
}

/// 根据文件名后缀推断 MIME 类型，便于上游识别音频格式。
fn guess_mime(file_name: &str) -> &'static str {
    let ext = file_name
        .rsplit('.')
        .next()
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "aac" => "audio/aac",
        _ => "application/octet-stream",
    }
}
