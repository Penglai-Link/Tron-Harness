//! 多媒体模型客户端 - 模型列表查询、图片生成、视频生成、任务进度查询、AIGC 内容检测
//!
//! 对应 API: `/api/v1/multimedia/models`, `/api/v1/images/generations`, `/api/v1/videos/generations`,
//! `/api/v1/multimedia/tasks/{id}`, `/api/v1/aigc/detections/**`
//!
//! 不使用请求参数加密信封，请求体为普通 JSON；返回值为 `ApiResponse`（`{code,message,data}`）。

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext, PageResult};
use crate::error::{Result, ServerApiError};

// ─── 数据模型 ──────────────────────────────────────────────

/// 多媒体任务响应
///
/// 对应 API: `MultimediaTaskResponse`
#[derive(Debug, Clone, Deserialize)]
pub struct MultimediaTaskResponse {
    pub id: String,
    pub media_type: i32,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    pub status: i32,
    pub progress: i32,
    #[serde(default)]
    pub result_url: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub remote_task_id: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// 多媒体任务进度响应（精简）
///
/// 对应 API: `MultimediaTaskProgressResponse`
#[derive(Debug, Clone, Deserialize)]
pub struct MultimediaTaskProgressResponse {
    pub id: String,
    pub status: i32,
    pub progress: i32,
    #[serde(default)]
    pub result_url: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

/// 多媒体模型公开信息（模型列表查询返回值）
///
/// 对应 Java: `MultimediaModelPublicResponse`
#[derive(Debug, Clone, Deserialize)]
pub struct MultimediaModelInfo {
    /// 模型 ID（创建生成任务时 `model_id` 字段填此值，非模型编码）
    pub model_id: String,
    /// 模型编码（展示/匹配用，如 `MiniMax-H3`）
    pub model_code: String,
    /// 模型名称（展示用）
    #[serde(default)]
    pub model_name: Option<String>,
    /// 多媒体类型（见 [`MediaType`]）
    pub media_type: i32,
}

/// 多媒体类型常量
pub struct MediaType;

impl MediaType {
    /// 图片
    pub const IMAGE: i32 = 0;
    /// 视频
    pub const VIDEO: i32 = 1;
    /// 语音
    pub const SPEECH: i32 = 2;
    /// 文本 AIGC 检测
    pub const AIGC_TEXT_DETECTION: i32 = 3;
    /// 图片 AIGC 检测
    pub const AIGC_IMAGE_DETECTION: i32 = 4;
}

/// 图片生成请求
#[derive(Debug, Clone, Serialize)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
}

/// 视频生成请求
///
/// 扩展参数按 MiniMax H3（v2 异步视频生成）约定：
/// `duration`/`resolution`/`ratio` 顶层透传；`first_frame`/`last_frame` 为首尾帧图片 URL；
/// `reference_images`/`reference_videos`/`reference_audios` 为参考素材 URL 数组；
/// 其余上游参数通过 `extra_params` 原样透传。
#[derive(Debug, Clone, Default, Serialize)]
pub struct VideoGenerationRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_frame: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_frame: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_videos: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_audios: Option<Vec<String>>,
    /// 未覆盖的上游参数（如 `content` 原生多模态数组等），扁平并入请求体顶层
    #[serde(flatten)]
    pub extra_params: Option<serde_json::Map<String, serde_json::Value>>,
}

/// 任务状态常量
pub struct TaskStatus;

impl TaskStatus {
    pub const PENDING: i32 = 0;
    pub const PROCESSING: i32 = 1;
    pub const SUCCEEDED: i32 = 2;
    pub const FAILED: i32 = 3;
}

// ─── AIGC 内容检测 ─────────────────────────────────────────

/// 文本检测最大长度（字符），超过时服务端返回 400
pub const MAX_TEXT_DETECTION_LENGTH: usize = 20000;

/// 图片 Base64 内容最大长度（约对应 10MB 解码后字节，与服务端限制一致）
pub const MAX_IMAGE_BASE64_LENGTH: usize = 14_000_000;

/// AIGC 检测类型常量（对应 `detection_type`）
pub struct DetectionType;

impl DetectionType {
    /// 文本检测
    pub const TEXT: i32 = 0;
    /// 图片检测
    pub const IMAGE: i32 = 1;
}

/// AIGC 检测状态常量（对应 `status`）
pub struct DetectionStatus;

impl DetectionStatus {
    /// 检测成功
    pub const SUCCEEDED: i32 = 0;
    /// 检测失败（错误信息见 `last_error`）
    pub const FAILED: i32 = 1;
}

/// 文本 AIGC 检测请求
#[derive(Debug, Clone, Serialize)]
pub struct AigcTextDetectionRequest {
    /// 待检测文本（最长 [`MAX_TEXT_DETECTION_LENGTH`] 字符）
    pub text: String,
    /// 是否合并分段，默认 `true`；`false` 时每个段落独立输出置信度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_merge: Option<bool>,
    /// 文本检测模型 ID（`media_type=3`），不传时使用启用中的默认模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

/// 图片 AIGC 检测请求
///
/// `image_url` 与 `image_base64` 二选一，都传时服务端优先使用 `image_url`；
/// 上游要求图片大于 300×300 像素，支持 jpg/png/webp，解码后不超过 10MB。
#[derive(Debug, Clone, Default, Serialize)]
pub struct AigcImageDetectionRequest {
    /// 图片公网 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// 图片 Base64 内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
    /// 图片检测模型 ID（`media_type=4`），不传时使用启用中的默认模型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

/// AIGC 检测分段标签（仅文本检测时有值）
#[derive(Debug, Clone, Deserialize)]
pub struct AigcDetectionSegment {
    #[serde(default)]
    pub text: Option<String>,
    /// 分段类型（0=人工，1=AI，2=疑似 AI）
    #[serde(default)]
    pub label: Option<i32>,
    /// 分段置信度
    #[serde(default)]
    pub conf: Option<f64>,
    #[serde(default)]
    pub order: Option<i32>,
    /// 分段在原文中的位置 `[start, end)`
    #[serde(default)]
    pub position: Option<Vec<i32>>,
}

/// AIGC 内容检测响应
///
/// 文本检测填充占比与分段字段；图片检测仅填充 `ai_confidence`。
#[derive(Debug, Clone, Deserialize)]
pub struct AigcDetectionResponse {
    pub id: String,
    /// 检测类型（0=文本，1=图片）
    pub detection_type: i32,
    /// 模型编码（如 `zhuque-text`/`zhuque-image`）
    pub model_code: String,
    /// 检测状态（0=成功，1=失败）
    pub status: i32,
    /// AI 生成置信度（文本=整体置信度；图片=AI 生成置信度，取值 0-1）
    #[serde(default)]
    pub ai_confidence: Option<f64>,
    /// 人写内容占比（仅文本检测，取值 0-1）
    #[serde(default)]
    pub human_ratio: Option<f64>,
    /// AI 内容占比（仅文本检测，取值 0-1）
    #[serde(default)]
    pub ai_ratio: Option<f64>,
    /// 疑似 AI 内容占比（仅文本检测，取值 0-1）
    #[serde(default)]
    pub suspected_ai_ratio: Option<f64>,
    /// 疑似 AI 内容整体占比（仅文本检测）
    #[serde(default)]
    pub ratio_confidence: Option<f64>,
    #[serde(default)]
    pub segment_labels: Option<Vec<AigcDetectionSegment>>,
    /// 待检测文本原文（仅文本检测）
    #[serde(default)]
    pub input_text: Option<String>,
    /// 待检测图片 URL（仅图片检测且通过 URL 提交）
    #[serde(default)]
    pub image_url: Option<String>,
    /// 图片内容 SHA-256 摘要（仅图片检测且通过 Base64 提交）
    #[serde(default)]
    pub image_digest: Option<String>,
    /// 文本检测是否合并分段
    #[serde(default)]
    pub is_merge: Option<bool>,
    /// 上游 token 用量（仅记录不参与扣费，上游未返回时为空）
    #[serde(default)]
    pub tokens_used: Option<i32>,
    /// 本次扣减积分数（按次 1 × 系数）
    #[serde(default)]
    pub points_cost: Option<f64>,
    /// 检测失败错误信息（成功时为空）
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// AIGC 检测记录分页查询参数
#[derive(Debug, Clone, Default)]
pub struct AigcDetectionQuery {
    /// 检测类型过滤（0=文本，1=图片），不传返回全部类型
    pub detection_type: Option<i32>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

impl AigcDetectionQuery {
    pub fn of(page: i32, page_size: i32) -> Self {
        Self {
            detection_type: None,
            page: Some(page),
            page_size: Some(page_size),
        }
    }

    /// 按检测类型过滤
    pub fn with_detection_type(mut self, detection_type: i32) -> Self {
        self.detection_type = Some(detection_type);
        self
    }
}

// ─── 多媒体客户端 ──────────────────────────────────────────

/// 多媒体模型客户端
///
/// 提供模型列表查询、图片/视频生成和任务进度查询能力。
/// 使用 `Authorization: Bearer <token>` 鉴权，不使用加密信封。
pub struct MultimediaClient {
    api: AuthorizedClientApi,
}

impl MultimediaClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 查询启用中的多媒体模型列表
    ///
    /// `GET /api/v1/multimedia/models` (仅需用户 Bearer Token，无需加密信封与 `X-Client-Id`)
    ///
    /// 返回启用状态的多媒体模型公开列表（按类型升序、创建时间倒序）。
    /// `media_type` 为可选类型过滤（见 [`MediaType`]），不传返回全部类型。
    /// 创建生成任务前可先通过此接口查询模型，将返回的 `model_id`
    /// 填入 `generate_image`/`generate_video` 请求的 `model_id` 字段显式指定模型。
    pub async fn list_models(
        &self,
        context: &AuthorizedClientContext,
        media_type: Option<i32>,
    ) -> Result<Vec<MultimediaModelInfo>> {
        let mut request =
            self.api
                .new_authorized_request(Method::GET, "/multimedia/models", context)?;
        if let Some(t) = media_type {
            request = request.query(&[("media_type", t.to_string())]);
        }
        self.api
            .execute_for_list(request, Some(context), "多媒体模型列表查询")
            .await
    }

    /// 提交图片生成任务
    ///
    /// `POST /api/v1/images/generations`
    ///
    /// 提交后返回任务信息（status=0），客户端需轮询任务进度获取最终结果。
    /// `model_id` 填模型 ID（见 [`Self::list_models`] 返回的 `model_id`，非模型编码），
    /// 不传时服务端按媒体类型选取当前启用模型。
    pub async fn generate_image(
        &self,
        context: &AuthorizedClientContext,
        req: &ImageGenerationRequest,
    ) -> Result<MultimediaTaskResponse> {
        let request = self
            .api
            .new_authorized_request(Method::POST, "/images/generations", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "图片生成")
            .await
    }

    /// 提交视频生成任务
    ///
    /// `POST /api/v1/videos/generations`
    ///
    /// 提交后返回任务信息（status=0），客户端需轮询任务进度获取最终结果。
    /// 支持 MiniMax H3 等视频模型：`model_id` 填模型 ID
    /// （见 [`Self::list_models`] 返回的 `model_id`，非模型编码），不传时服务端选取当前启用模型；
    /// 扩展参数（`duration`/`resolution`/`ratio`/首尾帧/参考素材等）透传上游。
    pub async fn generate_video(
        &self,
        context: &AuthorizedClientContext,
        req: &VideoGenerationRequest,
    ) -> Result<MultimediaTaskResponse> {
        let request = self
            .api
            .new_authorized_request(Method::POST, "/videos/generations", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "视频生成")
            .await
    }

    /// 查询任务进度（精简）
    ///
    /// `GET /api/v1/multimedia/tasks/{id}/progress`
    ///
    /// 仅包含进度轮询所需核心字段，适合高频查询。
    pub async fn get_task_progress(
        &self,
        context: &AuthorizedClientContext,
        task_id: &str,
    ) -> Result<MultimediaTaskProgressResponse> {
        let path = format!("/multimedia/tasks/{}/progress", task_id);
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "查询任务进度")
            .await
    }

    /// 查询任务详情
    ///
    /// `GET /api/v1/multimedia/tasks/{id}`
    pub async fn get_task(
        &self,
        context: &AuthorizedClientContext,
        task_id: &str,
    ) -> Result<MultimediaTaskResponse> {
        let path = format!("/multimedia/tasks/{}", task_id);
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "查询任务详情")
            .await
    }

    // ─── AIGC 内容检测 ──────────────────────────────────────

    /// 文本 AIGC 检测（同步返回结果）
    ///
    /// `POST /api/v1/aigc/detections/text`
    ///
    /// 对接腾讯朱雀检测模型（EdgeOne AI 网关）。每次检测按检测模型编码
    /// （如 `zhuque-text`）读取系数扣积分（按次 1 token × 系数），先扣后调，
    /// 上游失败不退积分（错误留痕在记录 `last_error` 字段，积分照扣）。
    /// `text` 为空或超过 [`MAX_TEXT_DETECTION_LENGTH`] 时客户端直接返回校验错误。
    pub async fn detect_text(
        &self,
        context: &AuthorizedClientContext,
        req: &AigcTextDetectionRequest,
    ) -> Result<AigcDetectionResponse> {
        if req.text.trim().is_empty() {
            return Err(ServerApiError::Validation("text 不能为空".to_string()));
        }
        let text_len = req.text.chars().count();
        if text_len > MAX_TEXT_DETECTION_LENGTH {
            return Err(ServerApiError::Validation(format!(
                "text 长度 {text_len} 超过上限 {MAX_TEXT_DETECTION_LENGTH}"
            )));
        }
        let request = self
            .api
            .new_authorized_request(Method::POST, "/aigc/detections/text", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "文本AIGC检测")
            .await
    }

    /// 图片 AIGC 检测（同步返回结果）
    ///
    /// `POST /api/v1/aigc/detections/image`
    ///
    /// `image_url` 与 `image_base64` 至少提供一个，否则客户端直接返回校验错误。
    /// 上游要求图片大于 300×300 像素，支持 jpg/png/webp，解码后不超过 10MB。
    pub async fn detect_image(
        &self,
        context: &AuthorizedClientContext,
        req: &AigcImageDetectionRequest,
    ) -> Result<AigcDetectionResponse> {
        if req.image_url.is_none() && req.image_base64.is_none() {
            return Err(ServerApiError::Validation(
                "image_url 与 image_base64 至少提供一个".to_string(),
            ));
        }
        if let Some(base64) = &req.image_base64 {
            if base64.len() > MAX_IMAGE_BASE64_LENGTH {
                return Err(ServerApiError::Validation(format!(
                    "image_base64 长度 {} 超过上限 {MAX_IMAGE_BASE64_LENGTH}",
                    base64.len()
                )));
            }
        }
        let request = self
            .api
            .new_authorized_request(Method::POST, "/aigc/detections/image", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "图片AIGC检测")
            .await
    }

    /// 分页查询本人检测记录
    ///
    /// `GET /api/v1/aigc/detections?detection_type=...&page=...&page_size=...`
    pub async fn list_detections(
        &self,
        context: &AuthorizedClientContext,
        query: &AigcDetectionQuery,
    ) -> Result<PageResult<AigcDetectionResponse>> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/aigc/detections", context)?;
        let mut request = request.query(&[("page", query.page.unwrap_or(1).to_string())]);
        request = request.query(&[(
            "page_size",
            query.page_size.unwrap_or(20).to_string(),
        )]);
        if let Some(detection_type) = query.detection_type {
            request = request.query(&[("detection_type", detection_type.to_string())]);
        }
        self.api
            .execute_for_page(request, Some(context), "查询检测记录列表")
            .await
    }

    /// 获取单条检测记录详情
    ///
    /// `GET /api/v1/aigc/detections/{id}`
    pub async fn get_detection(
        &self,
        context: &AuthorizedClientContext,
        detection_id: &str,
    ) -> Result<AigcDetectionResponse> {
        let path = format!("/aigc/detections/{detection_id}");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "查询检测记录详情")
            .await
    }
}
