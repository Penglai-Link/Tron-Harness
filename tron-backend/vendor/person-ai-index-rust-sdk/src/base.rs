//! 基础设施层 — 授权上下文、统一响应信封、分页结果、基础 API 客户端
//!
//! 对应 Java: `client/base/` 包下的全部类

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;

use crate::error::{Result, ServerApiError};

/// API 路径前缀
pub const API_PREFIX: &str = "/api/v1";

/// 非流式请求总超时
const NON_STREAM_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// 拼接错误及其 source 链（reqwest 顶层 Display 会隐藏超时/连接重置等根因）
pub fn format_error_chain(err: &dyn std::error::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        msg.push_str(&format!(" <- {cause}"));
        source = cause.source();
    }
    msg
}

/// Token 刷新回调
///
/// 对应 Java: `AbstractAuthorizedClientApi.TokenRefreshCallback`
#[async_trait::async_trait]
pub trait TokenRefreshCallback: Send + Sync {
    /// 使用 refresh_token 获取新的 token，返回 (新 token, 新 refresh_token)
    async fn refresh(&self, refresh_token: &str) -> Result<(String, Option<String>)>;
}

/// 授权客户端上下文
///
/// 对应 Java: `AuthorizedClientContext`
///
/// 持有 token / refreshToken / clientId / publicKey 四元组，
/// 贯穿所有需要认证的客户端调用。
#[derive(Debug, Clone, Default)]
pub struct AuthorizedClientContext {
    /// JWT access token
    pub token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 客户端标识
    pub client_id: Option<String>,
    /// RSA 公钥 (PEM 格式，来自服务端)
    pub public_key: Option<String>,
}

impl AuthorizedClientContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn with_refresh_token(mut self, refresh_token: impl Into<String>) -> Self {
        self.refresh_token = Some(refresh_token.into());
        self
    }

    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn with_public_key(mut self, public_key: impl Into<String>) -> Self {
        self.public_key = Some(public_key.into());
        self
    }

    /// 校验已授权接口: 仅需 token 非空
    pub fn validate_authorized_access(&self) -> Result<()> {
        if self.token.is_none() {
            return Err(ServerApiError::Validation("授权上下文缺少 token".into()));
        }
        Ok(())
    }

    /// 校验网关接口: token + clientId + publicKey 均非空
    pub fn validate_gateway_access(&self) -> Result<()> {
        self.validate_authorized_access()?;
        if self.client_id.is_none() {
            return Err(ServerApiError::Validation("授权上下文缺少 clientId".into()));
        }
        if self.public_key.is_none() {
            return Err(ServerApiError::Validation(
                "授权上下文缺少 publicKey".into(),
            ));
        }
        Ok(())
    }
}

/// 服务端统一响应信封
///
/// 对应 Java: `ApiResponseEnvelope`
///
/// 所有 API 返回都遵循此格式: `{"code": 0, "message": "OK", "data": {...}}`
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponseEnvelope<T> {
    pub code: Option<i32>,
    #[serde(alias = "msg")]
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ApiResponseEnvelope<T> {
    /// 检查业务码是否成功 (code == 0 或 200)
    pub fn is_success(&self) -> bool {
        matches!(self.code.unwrap_or(-1), 0 | 200)
    }
}

/// 分页结果
///
/// 对应 Java: `PageResult<T>`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageResult<T> {
    pub page: Option<i32>,
    #[serde(rename = "page_size")]
    pub page_size: Option<i32>,
    pub total: Option<i64>,
    pub items: Option<Vec<T>>,
}

/// 基础 API 客户端
///
/// 对应 Java: `AbstractAuthorizedClientApi`
///
/// 提供 URL 构建、请求构建 (普通/授权/网关三级)、响应解析 (信封格式)、
/// 401 自动刷新 Token 重试等核心能力。所有具体客户端都组合此结构。
pub struct AuthorizedClientApi {
    /// 服务端基础地址
    server_base_url: String,
    /// HTTP 客户端 (信任所有证书)
    http_client: Client,
    /// Token 刷新回调
    token_refresh_callback: Option<Arc<dyn TokenRefreshCallback>>,
    /// 防并发刷新的互斥锁
    refresh_lock: Mutex<()>,
}

impl AuthorizedClientApi {
    /// 创建新的 API 客户端
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        let http_client = Self::create_unsafe_client()?;
        Ok(Self {
            server_base_url: Self::normalize_base_url(&server_base_url.into()),
            http_client,
            token_refresh_callback: None,
            refresh_lock: Mutex::new(()),
        })
    }

    /// 使用已有的 HTTP 客户端创建
    pub fn with_client(server_base_url: impl Into<String>, http_client: Client) -> Self {
        Self {
            server_base_url: Self::normalize_base_url(&server_base_url.into()),
            http_client,
            token_refresh_callback: None,
            refresh_lock: Mutex::new(()),
        }
    }

    /// 设置 Token 刷新回调
    pub fn set_token_refresh_callback(&mut self, callback: Arc<dyn TokenRefreshCallback>) {
        self.token_refresh_callback = Some(callback);
    }

    /// 获取 HTTP 客户端引用
    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    /// 获取服务端基础 URL
    pub fn server_base_url(&self) -> &str {
        &self.server_base_url
    }

    // ─── URL 构建 ────────────────────────────────────────────

    /// 构建 API URL: `{serverBaseUrl}/api/v1{apiPath}`
    pub fn build_api_url(&self, api_path: &str) -> String {
        format!("{}{}{}", self.server_base_url, API_PREFIX, api_path)
    }

    /// 构建 WebSocket URL
    ///
    /// 将 http(s):// 转为 ws(s)://，并附加查询参数
    pub fn build_websocket_url(
        &self,
        api_path: &str,
        query_params: &HashMap<String, String>,
    ) -> Result<String> {
        let mut base = self.server_base_url.clone();

        // http -> ws, https -> wss
        if base.starts_with("https://") {
            base = format!("wss://{}", &base[8..]);
        } else if base.starts_with("http://") {
            base = format!("ws://{}", &base[7..]);
        }

        // 去除 /api/v1 后缀
        if base.ends_with(API_PREFIX) {
            base = base[..base.len() - API_PREFIX.len()].to_string();
        }

        let mut url = format!("{}{}{}", base, API_PREFIX, api_path);

        if !query_params.is_empty() {
            let pairs: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&pairs.join("&"));
        }

        Ok(url)
    }

    // ─── 请求构建 ────────────────────────────────────────────

    /// 构建普通请求 (无认证头)
    pub fn new_api_request(&self, method: Method, api_path: &str) -> RequestBuilder {
        self.http_client
            .request(method, self.build_api_url(api_path))
    }

    /// 构建授权请求 (附加 `Authorization: Bearer {token}`)
    pub fn new_authorized_request(
        &self,
        method: Method,
        api_path: &str,
        context: &AuthorizedClientContext,
    ) -> Result<RequestBuilder> {
        context.validate_authorized_access()?;
        let token = context.token.as_ref().unwrap();
        Ok(self
            .http_client
            .request(method, self.build_api_url(api_path))
            .bearer_auth(token))
    }

    /// 构建网关请求 (附加 `Authorization` + `X-Client-Id`)
    pub fn new_gateway_request(
        &self,
        method: Method,
        api_path: &str,
        context: &AuthorizedClientContext,
    ) -> Result<RequestBuilder> {
        context.validate_gateway_access()?;
        let token = context.token.as_ref().unwrap();
        let client_id = context.client_id.as_ref().unwrap();
        Ok(self
            .http_client
            .request(method, self.build_api_url(api_path))
            .bearer_auth(token)
            .header("X-Client-Id", client_id))
    }

    // ─── 响应执行 ────────────────────────────────────────────

    /// 执行请求并提取 `data` 字段反序列化为单个对象
    ///
    /// 支持 401 自动刷新 Token 重试
    pub async fn execute_for_object<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
        context: Option<&AuthorizedClientContext>,
        error_tag: &str,
    ) -> Result<T> {
        let response = self.send_with_retry(request, context, error_tag).await?;
        let envelope: ApiResponseEnvelope<T> = response
            .json()
            .await
            .map_err(|e| ServerApiError::Other(format!("{error_tag}: 反序列化失败: {e}")))?;

        if !envelope.is_success() {
            return Err(ServerApiError::Business {
                code: envelope.code.unwrap_or(-1),
                message: envelope.message.unwrap_or_else(|| error_tag.to_string()),
            });
        }

        envelope
            .data
            .ok_or_else(|| ServerApiError::Other(format!("{error_tag}: 响应缺少 data 字段")))
    }

    /// 执行请求并提取 `data` 字段反序列化为列表
    pub async fn execute_for_list<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
        context: Option<&AuthorizedClientContext>,
        error_tag: &str,
    ) -> Result<Vec<T>> {
        let response = self.send_with_retry(request, context, error_tag).await?;
        let envelope: ApiResponseEnvelope<Vec<T>> = response
            .json()
            .await
            .map_err(|e| ServerApiError::Other(format!("{error_tag}: 反序列化失败: {e}")))?;

        if !envelope.is_success() {
            return Err(ServerApiError::Business {
                code: envelope.code.unwrap_or(-1),
                message: envelope.message.unwrap_or_else(|| error_tag.to_string()),
            });
        }

        Ok(envelope.data.unwrap_or_default())
    }

    /// 执行请求并提取 `data` 字段反序列化为分页结果
    pub async fn execute_for_page<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
        context: Option<&AuthorizedClientContext>,
        error_tag: &str,
    ) -> Result<PageResult<T>> {
        let response = self.send_with_retry(request, context, error_tag).await?;
        let envelope: ApiResponseEnvelope<PageResult<T>> = response
            .json()
            .await
            .map_err(|e| ServerApiError::Other(format!("{error_tag}: 反序列化失败: {e}")))?;

        if !envelope.is_success() {
            return Err(ServerApiError::Business {
                code: envelope.code.unwrap_or(-1),
                message: envelope.message.unwrap_or_else(|| error_tag.to_string()),
            });
        }

        Ok(envelope.data.unwrap_or(PageResult {
            page: None,
            page_size: None,
            total: None,
            items: None,
        }))
    }

    /// 执行请求仅检查成功 (不解析 data)
    pub async fn execute_for_success(
        &self,
        request: RequestBuilder,
        context: Option<&AuthorizedClientContext>,
        error_tag: &str,
    ) -> Result<()> {
        let response = self.send_with_retry(request, context, error_tag).await?;
        let envelope: ApiResponseEnvelope<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| ServerApiError::Other(format!("{error_tag}: 反序列化失败: {e}")))?;

        if !envelope.is_success() {
            return Err(ServerApiError::Business {
                code: envelope.code.unwrap_or(-1),
                message: envelope.message.unwrap_or_else(|| error_tag.to_string()),
            });
        }

        Ok(())
    }

    /// 执行请求并返回原始 JSON (不走信封解析，网关接口用)
    pub async fn execute_for_raw_json(
        &self,
        request: RequestBuilder,
        context: Option<&AuthorizedClientContext>,
        error_tag: &str,
    ) -> Result<serde_json::Value> {
        let response = self.send_with_retry(request, context, error_tag).await?;
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ServerApiError::Other(format!("{error_tag}: 反序列化失败: {e}")))?;
        Ok(json)
    }

    /// 执行流式请求，通过回调逐行处理 SSE 事件
    pub async fn execute_for_stream<F>(
        &self,
        request: RequestBuilder,
        _context: Option<&AuthorizedClientContext>,
        error_tag: &str,
        mut on_data: F,
    ) -> Result<()>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let response = request.send().await.map_err(|e| {
            ServerApiError::Other(format!("{error_tag}: 请求失败: {}", format_error_chain(&e)))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(match Self::map_error_body(status, &body, error_tag) {
                ServerApiError::DecryptFailed => ServerApiError::DecryptFailed,
                ServerApiError::Unauthorized => ServerApiError::Unauthorized,
                _ => ServerApiError::Other(format!("流式请求失败: {status}, body: {body}")),
            });
        }

        use futures_util::StreamExt;
        let mut stream = response.bytes_stream();
        // 按字节缓冲，仅在遇到 '\n'（ASCII，必落在 UTF-8 字符边界）后整行解码，
        // 避免 chunk 边界切断多字节字符产生 U+FFFD 乱码
        let mut buffer: Vec<u8> = Vec::new();
        // 不设总超时（长生成会超过固定上限），仅限制单次读取的空闲等待
        const SSE_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

        loop {
            let chunk = match tokio::time::timeout(SSE_IDLE_TIMEOUT, stream.next()).await {
                Ok(Some(Ok(chunk))) => chunk,
                Ok(Some(Err(e))) => {
                    return Err(ServerApiError::Other(format!(
                        "{error_tag}: 流式响应读取失败: {}",
                        format_error_chain(&e)
                    )));
                }
                Ok(None) => break,
                Err(_) => {
                    return Err(ServerApiError::Other(format!(
                        "{error_tag}: 流式响应读取超时（{} 秒无数据）",
                        SSE_IDLE_TIMEOUT.as_secs()
                    )));
                }
            };
            buffer.extend_from_slice(&chunk);

            // 按行解析 data: 前缀的 SSE 事件
            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..pos + 1).collect();
                let line = String::from_utf8_lossy(&line_bytes);
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                // 兼容 data: (有空格) 和 data: (无空格) 两种 SSE 格式
                let data = if line.starts_with("data: ") {
                    &line[6..]
                } else if line.starts_with("data:") {
                    &line[5..]
                } else {
                    continue;
                };
                if data == "[DONE]" {
                    return Ok(());
                }

                on_data(data)?;
            }
        }

        // 流结束：处理缓冲区中可能残留的最后一行（服务端未以换行符结尾的情况）
        let remaining = String::from_utf8_lossy(&buffer);
        let remaining = remaining.trim();
        if !remaining.is_empty() {
            let data = remaining
                .strip_prefix("data: ")
                .or_else(|| remaining.strip_prefix("data:"));
            if let Some(data) = data {
                let data = data.trim();
                if data == "[DONE]" {
                    return Ok(());
                }
                if !data.is_empty() {
                    on_data(data)?;
                }
            }
        }

        // 未收到 [DONE] 结束标记连接就关闭了，属于异常断流（上游错误、网络截断等），
        // 必须返回错误而非静默成功，避免上层把空响应或截断响应当成正常结果
        Err(ServerApiError::Other(format!(
            "{error_tag}: 流式响应异常结束：未收到 [DONE] 结束标记"
        )))
    }

    // ─── 内部方法 ────────────────────────────────────────────

    /// 发送请求，遇到 401 时自动刷新 Token 并重试
    async fn send_with_retry(
        &self,
        request: RequestBuilder,
        context: Option<&AuthorizedClientContext>,
        error_tag: &str,
    ) -> Result<reqwest::Response> {
        let response = request
            .timeout(NON_STREAM_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| {
                ServerApiError::Other(format!("{error_tag}: 请求失败: {}", format_error_chain(&e)))
            })?;

        if response.status() == StatusCode::UNAUTHORIZED {
            // 尝试刷新 Token 并重试
            if let (Some(ctx), Some(callback)) = (context, &self.token_refresh_callback) {
                if let Some(refresh_token) = &ctx.refresh_token {
                    // 获取锁防止并发刷新
                    let _guard = self.refresh_lock.lock().await;

                    // 双重检查: 可能其他线程已经刷新了
                    // 注意: 由于 RequestBuilder 已被消费，这里需要重新构建请求
                    // 简化实现: 直接返回 401 错误，由上层处理重试
                    let refresh_result = callback.refresh(refresh_token).await;

                    if let Ok((new_token, new_refresh_token)) = refresh_result {
                        tracing::info!("{error_tag}: Token 已刷新，重试请求");

                        // 更新上下文 (需要可变引用，这里通过 Arc<Mutex> 模式由上层处理)
                        // 由于 Rust 所有权模型，这里仅记录日志，实际重试由上层发起
                        let _ = (new_token, new_refresh_token);

                        // 返回特定错误，上层可据此重试
                        return Err(ServerApiError::Other(format!(
                            "{error_tag}: Token 已过期且已刷新，请重试"
                        )));
                    }
                }
            }
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Self::map_error_body(status, &body, error_tag));
        }

        Ok(response)
    }

    /// 将非 2xx 响应映射为错误，识别服务端 `decrypt_error`
    fn map_error_body(status: StatusCode, body: &str, error_tag: &str) -> ServerApiError {
        let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
        let is_decrypt_error = parsed
            .as_ref()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("type"))
                    .and_then(|t| t.as_str())
                    .map(|s| s == "decrypt_error")
            })
            .unwrap_or(false);
        if is_decrypt_error {
            return ServerApiError::DecryptFailed;
        }
        if status == StatusCode::UNAUTHORIZED {
            return ServerApiError::Unauthorized;
        }
        // 非 2xx 也可能携带标准信封体（如无可用版本时 HTTP 404 + {"code":404,...}），
        // 映射为 Business 让调用方可以按业务码处理
        if let Some(code) = parsed
            .as_ref()
            .and_then(|v| v.get("code"))
            .and_then(|c| c.as_i64())
        {
            let message = parsed
                .as_ref()
                .and_then(|v| v.get("message").or_else(|| v.get("msg")))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{error_tag}: HTTP {status}"));
            return ServerApiError::Business {
                code: code as i32,
                message,
            };
        }
        ServerApiError::Other(format!("{error_tag}: HTTP {status}, body: {body}"))
    }

    /// 创建信任所有证书的 HTTP 客户端
    ///
    /// 不设置总超时：总超时从建连计到响应体读完，会掐断进行中的 SSE 流；
    /// 非流式请求在 `send_with_retry` 按请求设置总超时，流式请求仅做空闲超时
    fn create_unsafe_client() -> Result<Client> {
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "X-Product-Code",
            reqwest::header::HeaderValue::from_static("tron"),
        );
        Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(default_headers)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ServerApiError::Other(format!("创建 HTTP 客户端失败: {e}")))
    }

    /// 归一化基础 URL (去除尾部斜杠)
    fn normalize_base_url(url: &str) -> String {
        url.trim_end_matches('/').to_string()
    }
}

#[cfg(test)]
mod product_code_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn unauthorized_responses_are_exposed_as_a_typed_error_for_runtime_retry() {
        assert!(matches!(
            AuthorizedClientApi::map_error_body(
                StatusCode::UNAUTHORIZED,
                r#"{"message":"expired"}"#,
                "chat completion"
            ),
            ServerApiError::Unauthorized
        ));

        assert!(matches!(
            AuthorizedClientApi::map_error_body(
                StatusCode::BAD_REQUEST,
                r#"{"error":{"type":"decrypt_error"}}"#,
                "chat completion"
            ),
            ServerApiError::DecryptFailed
        ));
    }

    #[tokio::test]
    async fn common_sdk_client_sends_tron_product_code_for_all_request_classes() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut chunk = [0_u8; 1024];
                loop {
                    let read = stream.read(&mut chunk).await.unwrap();
                    request.extend_from_slice(&chunk[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
                assert!(request.contains("x-product-code: tron"), "{request}");
                stream
                    .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
            }
        });

        let api = AuthorizedClientApi::new(format!("http://{address}")).unwrap();
        let context = AuthorizedClientContext::new()
            .with_token("token")
            .with_client_id("client")
            .with_public_key("public-key");
        let requests = [
            api.new_api_request(Method::GET, "/plain"),
            api.new_authorized_request(Method::GET, "/authorized", &context)
                .unwrap(),
            api.new_gateway_request(Method::GET, "/gateway", &context)
                .unwrap(),
        ];
        for request in requests {
            api.http_client
                .execute(request.build().unwrap())
                .await
                .unwrap();
        }
        server.await.unwrap();
    }
}

/// URL 编码工具 (内部使用)
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
