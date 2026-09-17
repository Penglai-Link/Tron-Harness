//! 认证客户端 - 登录会话、Token 刷新、心跳、账户信息
//!
//! 对应 Java: `client/auth/` 包

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::Result;

// ─── 数据模型 ──────────────────────────────────────────────

/// 创建登录会话请求
///
/// 对应 Java: `AuthorizedClientSessionRequest`
#[derive(Debug, Clone, Serialize)]
pub struct SessionRequest {
    #[serde(rename = "client_id", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(rename = "client_name")]
    pub client_name: String,
    #[serde(rename = "callback_url", skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

/// 登录会话信息
///
/// 对应 Java: `AuthorizedClientSession`
#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub session_id: String,
    #[serde(default, alias = "clientId")]
    pub client_id: Option<String>,
    #[serde(default, alias = "clientName")]
    pub client_name: Option<String>,
    pub status: String, // pending / completed / cancelled / expired
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default, alias = "refreshToken")]
    pub refresh_token: Option<String>,
    #[serde(default, alias = "expiresIn")]
    pub expires_in: Option<i64>,
    #[serde(default, alias = "publicKey")]
    pub public_key: Option<String>,
    #[serde(default, alias = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(default, alias = "loginUrl")]
    pub login_url: Option<String>,
}

impl Session {
    pub fn is_completed(&self) -> bool {
        self.status.eq_ignore_ascii_case("completed")
    }

    pub fn is_finished(&self) -> bool {
        ["completed", "cancelled", "expired"]
            .iter()
            .any(|s| self.status.eq_ignore_ascii_case(s))
    }
}

/// 密码登录请求
///
/// 对应 Java: `AuthorizedClientPasswordLoginRequest`
#[derive(Debug, Clone, Serialize)]
pub struct PasswordLoginRequest {
    pub account: String,
    pub password: String,
    #[serde(rename = "client_name")]
    pub client_name: String,
    #[serde(rename = "os_type", skip_serializing_if = "Option::is_none")]
    pub os_type: Option<String>,
    #[serde(rename = "computer_name", skip_serializing_if = "Option::is_none")]
    pub computer_name: Option<String>,
}

/// 密码登录响应
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordLoginResult {
    #[serde(rename = "client_id")]
    pub client_id: String,
    pub token: String,
    #[serde(rename = "refresh_token")]
    pub refresh_token: String,
    #[serde(rename = "public_key")]
    pub public_key: String,
}

/// 发送短信验证码请求
///
/// `POST /api/v1/auth/otp/sms/send`
#[derive(Debug, Clone, Serialize)]
pub struct SmsSendRequest {
    pub phone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(rename = "captcha_id")]
    pub captcha_id: String,
    #[serde(rename = "captcha_answer")]
    pub captcha_answer: String,
}

/// 短信验证码登录请求
///
/// `POST /api/v1/auth/client-sessions/sms-login`
#[derive(Debug, Clone, Serialize)]
pub struct SmsLoginRequest {
    pub phone: String,
    pub code: String,
    #[serde(rename = "client_name")]
    pub client_name: String,
    #[serde(rename = "os_type", skip_serializing_if = "Option::is_none")]
    pub os_type: Option<String>,
    #[serde(rename = "computer_name", skip_serializing_if = "Option::is_none")]
    pub computer_name: Option<String>,
}

/// Token 状态查询响应
///
/// 对应 Java: `AuthorizedClientTokenStatus`
#[derive(Debug, Clone, Deserialize)]
pub struct TokenStatus {
    pub valid: bool,
    pub expired: bool,
    #[serde(rename = "expires_at", default)]
    pub expires_at: Option<String>,
    #[serde(rename = "expires_in", default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// Token 刷新响应
///
/// 对应 Java: `AuthorizedClientTokenRefresh`
#[derive(Debug, Clone, Deserialize)]
pub struct TokenRefresh {
    pub token: String,
    #[serde(rename = "expires_in", default)]
    pub expires_in: Option<i64>,
    #[serde(rename = "refresh_token", default)]
    pub refresh_token: Option<String>,
}

/// 当前用户信息
///
/// 对应 Java: `AuthorizedClientAccountInfo`
#[derive(Debug, Clone, Deserialize)]
pub struct AccountInfo {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(rename = "vip_level", default)]
    pub vip_level: Option<i32>, // 0=无, 1=VIP, 2=Pro, 3=Max
    #[serde(rename = "remaining_tokens", default)]
    pub remaining_tokens: Option<i64>,
}

#[derive(Debug, Serialize)]
struct RefreshTokenRequest {
    #[serde(rename = "refresh_token")]
    refresh_token: String,
}

// ─── 登录客户端 ────────────────────────────────────────────

/// 登录客户端
///
/// 对应 Java: `AuthorizedClientLoginClient`
pub struct LoginClient {
    api: AuthorizedClientApi,
}

impl LoginClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 创建登录会话
    ///
    /// `POST /api/v1/auth/client-sessions`
    pub async fn create_session(&self, req: &SessionRequest) -> Result<Session> {
        let request = self
            .api
            .new_api_request(Method::POST, "/auth/client-sessions")
            .json(req);
        self.api
            .execute_for_object(request, None, "创建登录会话")
            .await
    }

    /// 获取登录会话状态
    ///
    /// `GET /api/v1/auth/client-sessions/{sessionId}`
    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        let path = format!("/auth/client-sessions/{session_id}");
        let request = self.api.new_api_request(Method::GET, &path);
        self.api
            .execute_for_object(request, None, "查询登录会话")
            .await
    }

    /// 取消登录会话
    ///
    /// `POST /api/v1/auth/client-sessions/{sessionId}/cancel`
    pub async fn cancel_session(&self, session_id: &str) -> Result<()> {
        let path = format!("/auth/client-sessions/{session_id}/cancel");
        let request = self.api.new_api_request(Method::POST, &path);
        self.api
            .execute_for_success(request, None, "取消登录会话")
            .await
    }

    /// 密码直接登录
    ///
    /// `POST /api/v1/auth/client-sessions/password-login`
    pub async fn password_login(&self, req: &PasswordLoginRequest) -> Result<PasswordLoginResult> {
        let request = self
            .api
            .new_api_request(Method::POST, "/auth/client-sessions/password-login")
            .json(req);
        self.api
            .execute_for_object(request, None, "密码登录")
            .await
    }

    /// 发送短信验证码
    ///
    /// `POST /api/v1/auth/otp/sms/send`
    pub async fn send_sms_code(
        &self,
        phone: &str,
        scene: Option<&str>,
        captcha_id: &str,
        captcha_answer: &str,
    ) -> Result<()> {
        let req = SmsSendRequest {
            phone: phone.to_string(),
            scene: scene.map(|s| s.to_string()),
            captcha_id: captcha_id.to_string(),
            captcha_answer: captcha_answer.to_string(),
        };
        let request = self
            .api
            .new_api_request(Method::POST, "/auth/otp/sms/send")
            .json(&req);
        self.api
            .execute_for_success(request, None, "发送短信验证码")
            .await
    }

    /// 短信验证码登录
    ///
    /// `POST /api/v1/auth/client-sessions/sms-login`
    pub async fn sms_login(&self, req: &SmsLoginRequest) -> Result<PasswordLoginResult> {
        let request = self
            .api
            .new_api_request(Method::POST, "/auth/client-sessions/sms-login")
            .json(req);
        self.api
            .execute_for_object(request, None, "短信验证码登录")
            .await
    }

    /// 查询 Token 状态
    ///
    /// `GET /api/v1/auth/token/status`
    pub async fn get_token_status(&self, token: &str) -> Result<TokenStatus> {
        let request = self
            .api
            .new_api_request(Method::GET, "/auth/token/status")
            .bearer_auth(token);
        self.api
            .execute_for_object(request, None, "查询Token状态")
            .await
    }

    /// 刷新 Token
    ///
    /// `POST /api/v1/auth/token/refresh`
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenRefresh> {
        let req = RefreshTokenRequest {
            refresh_token: refresh_token.to_string(),
        };
        let request = self
            .api
            .new_api_request(Method::POST, "/auth/token/refresh")
            .json(&req);
        self.api
            .execute_for_object(request, None, "刷新Token")
            .await
    }

    /// 构建浏览器登录地址
    ///
    /// `{webBaseUrl}/client/login?session_id={id}&os_type={os}&computer_name={name}`
    pub fn build_login_url(
        &self,
        session_id: &str,
        os_type: Option<&str>,
        computer_name: Option<&str>,
    ) -> String {
        let mut url = format!(
            "{}/client/login?session_id={}",
            self.api.server_base_url().trim_end_matches('/'),
            urlencoding::encode(session_id)
        );
        if let Some(os) = os_type {
            url.push_str(&format!("&os_type={}", urlencoding::encode(os)));
        }
        if let Some(name) = computer_name {
            url.push_str(&format!("&computer_name={}", urlencoding::encode(name)));
        }
        url
    }

    /// 探测操作系统类型
    pub fn detect_os_type() -> &'static str {
        match std::env::consts::OS {
            "windows" => "windows",
            "macos" => "macos",
            "linux" => "linux",
            _ => "unknown",
        }
    }

    /// 探测计算机名称
    pub fn detect_computer_name() -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// 获取底层 API 客户端
    pub fn api(&self) -> &AuthorizedClientApi {
        &self.api
    }
}

// ─── 账户客户端 ────────────────────────────────────────────

/// 账户信息客户端
///
/// 对应 Java: `AuthorizedClientAccountClient`
pub struct AccountClient {
    api: AuthorizedClientApi,
}

impl AccountClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 获取当前用户信息
    ///
    /// `GET /api/v1/account/info`
    /// 需要 `Authorization: Bearer {token}` + `X-Client-Id: {clientId}`
    pub async fn get_info(&self, context: &AuthorizedClientContext) -> Result<AccountInfo> {
        let request = self
            .api
            .new_gateway_request(Method::GET, "/account/info", context)?;
        self.api
            .execute_for_object(request, Some(context), "获取账户信息")
            .await
    }
}

/// URL 编码 (内部使用)
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes())
            .collect()
    }
}
