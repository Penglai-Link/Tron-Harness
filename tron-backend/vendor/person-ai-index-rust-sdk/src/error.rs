//! 统一错误类型

use thiserror::Error;

/// 服务端 API 错误类型
#[derive(Debug, Error)]
pub enum ServerApiError {
    /// 业务异常 (服务端返回 code != 0)
    #[error("业务异常: {message}")]
    Business { code: i32, message: String },

    /// 参数校验失败
    #[error("参数校验失败: {0}")]
    Validation(String),

    /// HTTP 请求失败
    #[error("HTTP请求失败: {0}")]
    Http(#[from] reqwest::Error),

    /// HTTP 401: 授权访问令牌已被拒绝
    #[error("HTTP 401: access token was rejected")]
    Unauthorized,

    /// JSON 序列化/反序列化失败
    #[error("JSON错误: {0}")]
    Json(#[from] serde_json::Error),

    /// 加密/解密错误
    #[error("加密错误: {0}")]
    Crypto(String),

    /// 服务端解密请求信封失败 (本地授权密钥与服务端不匹配，需重新登录)
    #[error("服务端解密失败: 授权密钥已失效，请重新登录后再试")]
    DecryptFailed,

    /// WebSocket 错误
    #[error("WebSocket错误: {0}")]
    WebSocket(String),

    /// IO 错误
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    /// URL 解析错误
    #[error("URL错误: {0}")]
    Url(#[from] url::ParseError),

    /// 其他内部错误
    #[error("{0}")]
    Other(String),
}

impl ServerApiError {
    /// 创建业务异常
    pub fn business(code: i32, message: impl Into<String>) -> Self {
        Self::Business {
            code,
            message: message.into(),
        }
    }

    /// 创建加密错误
    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }

    /// 创建其他错误
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl From<base64::DecodeError> for ServerApiError {
    fn from(e: base64::DecodeError) -> Self {
        Self::Crypto(format!("Base64解码失败: {e}"))
    }
}

impl From<rsa::Error> for ServerApiError {
    fn from(e: rsa::Error) -> Self {
        Self::Crypto(format!("RSA错误: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, ServerApiError>;
