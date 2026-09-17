//! 用户反馈客户端
//!
//! 客户端应用通过本模块收集用户反馈的问题：
//! 反馈内容为文本，可附带多张 base64 图片（字符串数组），服务端原样存储。
//!
//! 单张图片 base64 内容最大 50KB（[`MAX_IMAGE_LENGTH`]），超过时需调用方先压缩再上传，
//! 否则服务端返回 422。

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::Result;

/// 单张反馈图片 base64 内容的最大长度（50KB），超过需调用方压缩后上传
pub const MAX_IMAGE_LENGTH: usize = 50 * 1024;

/// 单次反馈最多上传的图片数量
pub const MAX_IMAGES: usize = 9;

/// 反馈内容的最大长度（字符数）
pub const MAX_CONTENT_LENGTH: usize = 5000;

// ─── 数据模型 ──────────────────────────────────────────────

/// 提交用户反馈请求
#[derive(Debug, Clone, Serialize)]
pub struct FeedbackCreateRequest {
    /// 反馈内容（必填，最长 [`MAX_CONTENT_LENGTH`] 字符）
    pub content: String,
    /// 反馈图片（可选，最多 [`MAX_IMAGES`] 张；元素为 base64 图片内容，
    /// 可带 `data:image/...;base64,` 前缀，单张最大 [`MAX_IMAGE_LENGTH`] 字符）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

impl FeedbackCreateRequest {
    /// 构建仅包含文本内容的反馈请求
    pub fn of(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            images: None,
        }
    }

    /// 附带反馈图片
    pub fn with_images(mut self, images: Vec<String>) -> Self {
        self.images = Some(images);
        self
    }
}

/// 提交用户反馈响应
#[derive(Debug, Clone, Deserialize)]
pub struct FeedbackCreateResult {
    /// 反馈记录 ID
    #[serde(default)]
    pub id: Option<String>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 用户反馈客户端
///
/// 需要 Bearer token 认证
pub struct FeedbackClient {
    api: AuthorizedClientApi,
}

impl FeedbackClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 提交用户反馈
    ///
    /// `POST /api/v1/feedback`
    ///
    /// 图片单张超过 50KB 时服务端返回 422，需压缩后重新上传
    pub async fn create(
        &self,
        context: &AuthorizedClientContext,
        req: &FeedbackCreateRequest,
    ) -> Result<FeedbackCreateResult> {
        let request = self
            .api
            .new_authorized_request(Method::POST, "/feedback", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "提交用户反馈")
            .await
    }
}
