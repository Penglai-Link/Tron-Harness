//! 客户端文件上传与管理
//!
//! 对应服务端 API: `/api/v1/files`
//!
//! 上传采用「客户端直传七牛云」模式，文件内容不经过应用服务器，避免网络转发：
//!
//! 1. [`FileClient::upload_token`] 获取七牛云上传凭证与本次必须使用的临时 key
//! 2. [`FileClient::upload_to_storage`] 以表单方式把文件字节直接 POST 到七牛云上传地址
//! 3. [`FileClient::save`] 通知服务端保存文件信息，服务端校验归属后把临时对象转为正式文件并落库
//!
//! [`FileClient::upload`] 是上述三步的一站式封装。
//!
//! 临时对象若始终没有调用 [`FileClient::save`] 转正，服务端定时任务会在保留期
//! （默认 24 小时）后清理，因此拿到凭证后应尽快完成上传与保存。

use std::time::Duration;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::{Result, ServerApiError};

/// 直传七牛云的请求超时
///
/// 大文件上传耗时会超过服务端接口的默认 120 秒超时，这里单独放宽。
const STORAGE_UPLOAD_TIMEOUT: Duration = Duration::from_secs(600);

// ─── 数据模型 ──────────────────────────────────────────────

/// 七牛云上传凭证
///
/// 凭证的 scope 精确到 [`FileUploadToken::key`]，只能用该凭证写入该 key。
#[derive(Debug, Clone, Deserialize)]
pub struct FileUploadToken {
    /// 七牛云上传凭证
    #[serde(default)]
    pub upload_token: Option<String>,
    /// 本次上传必须使用的临时对象 key
    #[serde(default)]
    pub key: Option<String>,
    /// 七牛云存储桶名称
    #[serde(default)]
    pub bucket: Option<String>,
    /// 七牛云上传地址（表单上传的目标 URL）
    #[serde(default)]
    pub upload_host: Option<String>,
    /// 七牛云存储空间域名
    #[serde(default)]
    pub domain: Option<String>,
    /// 凭证过期时间（Unix 秒级时间戳），过期需重新获取
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// 单文件大小上限（字节），超过时七牛云直接拒绝
    #[serde(default)]
    pub max_size_bytes: Option<i64>,
}

/// 待上传的文件内容
///
/// 文件字节由调用方读入内存后传入，SDK 内部不读文件。
#[derive(Debug, Clone)]
pub struct FileUploadRequest {
    /// 原始文件名（含扩展名，用于服务端推断扩展名与下载时的文件名）
    pub file_name: String,
    /// 文件字节内容
    pub file_bytes: Vec<u8>,
    /// 文件 MIME 类型，为空时按扩展名推断
    pub content_type: Option<String>,
}

impl FileUploadRequest {
    /// 构建上传请求
    pub fn of(file_name: impl Into<String>, file_bytes: Vec<u8>) -> Self {
        Self {
            file_name: file_name.into(),
            file_bytes,
            content_type: None,
        }
    }

    /// 指定文件 MIME 类型
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

/// 七牛云表单上传响应
#[derive(Debug, Clone, Deserialize)]
pub struct QiniuUploadResult {
    /// 七牛云回传的对象 key
    #[serde(default)]
    pub key: Option<String>,
    /// 文件内容哈希
    #[serde(default)]
    pub hash: Option<String>,
}

/// 保存文件信息请求
#[derive(Debug, Clone, Serialize)]
pub struct FileSaveRequest {
    /// 上传凭证返回的临时对象 key（必填）
    pub temp_key: String,
    /// 原始文件名（必填）
    pub file_name: String,
    /// 文件大小（字节，可空；服务端以七牛云实际大小为准）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    /// 文件 MIME 类型（可空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

impl FileSaveRequest {
    /// 构建保存文件信息请求
    pub fn of(temp_key: impl Into<String>, file_name: impl Into<String>) -> Self {
        Self {
            temp_key: temp_key.into(),
            file_name: file_name.into(),
            file_size: None,
            content_type: None,
        }
    }

    /// 指定文件大小（字节）
    pub fn with_file_size(mut self, file_size: i64) -> Self {
        self.file_size = Some(file_size);
        self
    }

    /// 指定文件 MIME 类型
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

/// 文件详情
#[derive(Debug, Clone, Deserialize)]
pub struct FileDetail {
    /// 文件 ID（删除与后续查询都使用它）
    #[serde(default)]
    pub id: Option<String>,
    /// 原始文件名
    #[serde(default)]
    pub file_name: Option<String>,
    /// 七牛云正式对象 key
    #[serde(default)]
    pub file_key: Option<String>,
    /// 文件大小（字节）
    #[serde(default)]
    pub file_size: Option<i64>,
    /// 文件 MIME 类型
    #[serde(default)]
    pub content_type: Option<String>,
    /// 临时下载地址（有效期有限，不要缓存复用）
    #[serde(default)]
    pub download_url: Option<String>,
    /// 创建时间
    #[serde(default)]
    pub created_at: Option<String>,
    /// 更新时间
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 客户端文件客户端
///
/// 需要 Bearer token 认证。所有接口都按用户隔离，只能操作自己上传的文件。
pub struct FileClient {
    api: AuthorizedClientApi,
}

impl FileClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 一站式上传：获取凭证 → 直传七牛云 → 保存文件信息并转正
    ///
    /// 返回的 [`FileDetail`] 含文件 ID 与临时下载地址。
    pub async fn upload(
        &self,
        context: &AuthorizedClientContext,
        req: &FileUploadRequest,
    ) -> Result<FileDetail> {
        let token = self.upload_token(context, &req.file_name).await?;
        self.upload_to_storage(&token, req).await?;

        let temp_key = token.key.clone().ok_or_else(|| {
            ServerApiError::other("获取上传凭证失败: 响应缺少 key 字段")
        })?;
        let mut save_req = FileSaveRequest::of(temp_key, req.file_name.clone())
            .with_file_size(req.file_bytes.len() as i64);
        if let Some(content_type) = resolve_content_type(req) {
            save_req = save_req.with_content_type(content_type);
        }
        self.save(context, &save_req).await
    }

    /// 获取七牛云上传凭证
    ///
    /// `GET /api/v1/files/upload-token?file_name=...`
    pub async fn upload_token(
        &self,
        context: &AuthorizedClientContext,
        file_name: &str,
    ) -> Result<FileUploadToken> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/files/upload-token", context)?
            .query(&[("file_name", file_name)]);
        self.api
            .execute_for_object(request, Some(context), "获取文件上传凭证")
            .await
    }

    /// 把文件字节直传到七牛云
    ///
    /// 该请求发往七牛云上传地址而非本服务端，因此不走统一响应信封解析。
    /// 七牛云表单上传要求 `file` 字段排在最后，这里的字段顺序不可调整。
    pub async fn upload_to_storage(
        &self,
        token: &FileUploadToken,
        req: &FileUploadRequest,
    ) -> Result<QiniuUploadResult> {
        use reqwest::multipart;

        let upload_token = token.upload_token.as_ref().ok_or_else(|| {
            ServerApiError::other("上传凭证缺少 upload_token 字段")
        })?;
        let key = token
            .key
            .as_ref()
            .ok_or_else(|| ServerApiError::other("上传凭证缺少 key 字段"))?;
        let upload_host = token.upload_host.as_ref().ok_or_else(|| {
            ServerApiError::other("上传凭证缺少 upload_host 字段")
        })?;

        if let Some(max) = token.max_size_bytes {
            if max > 0 && req.file_bytes.len() as i64 > max {
                return Err(ServerApiError::Validation(format!(
                    "文件大小{}字节，超过上限{max}字节",
                    req.file_bytes.len()
                )));
            }
        }

        let mime = resolve_content_type(req).unwrap_or_else(|| "application/octet-stream".into());
        let form = multipart::Form::new()
            .text("token", upload_token.clone())
            .text("key", key.clone())
            .part(
                "file",
                multipart::Part::bytes(req.file_bytes.clone())
                    .file_name(req.file_name.clone())
                    .mime_str(&mime)
                    .map_err(|e| ServerApiError::other(format!("设置MIME失败: {e}")))?,
            );

        let response = self
            .api
            .http_client()
            .post(upload_host)
            .timeout(STORAGE_UPLOAD_TIMEOUT)
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                ServerApiError::Other(format!(
                    "上传文件到对象存储: 请求失败: {}",
                    crate::base::format_error_chain(&e)
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let reason = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or(body);
            return Err(ServerApiError::Other(format!(
                "上传文件到对象存储失败: HTTP {status}, {reason}"
            )));
        }

        response
            .json::<QiniuUploadResult>()
            .await
            .map_err(|e| ServerApiError::Other(format!("上传文件到对象存储: 反序列化失败: {e}")))
    }

    /// 保存文件信息，由服务端把临时对象转为正式文件
    ///
    /// `POST /api/v1/files`
    pub async fn save(
        &self,
        context: &AuthorizedClientContext,
        req: &FileSaveRequest,
    ) -> Result<FileDetail> {
        let request = self
            .api
            .new_authorized_request(Method::POST, "/files", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "保存文件信息")
            .await
    }

    /// 查询文件详情（含临时下载地址）
    ///
    /// `GET /api/v1/files/{id}`
    pub async fn detail(
        &self,
        context: &AuthorizedClientContext,
        file_id: &str,
    ) -> Result<FileDetail> {
        let path = format!("/files/{file_id}");
        let request = self.api.new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "查询文件详情")
            .await
    }

    /// 删除文件（同时删除服务端记录与对象存储中的文件）
    ///
    /// `DELETE /api/v1/files/{id}`
    pub async fn delete(
        &self,
        context: &AuthorizedClientContext,
        file_id: &str,
    ) -> Result<()> {
        let path = format!("/files/{file_id}");
        let request = self
            .api
            .new_authorized_request(Method::DELETE, &path, context)?;
        self.api
            .execute_for_success(request, Some(context), "删除文件")
            .await
    }

    /// 根据临时下载地址拉取文件字节
    pub async fn download(&self, download_url: &str) -> Result<Vec<u8>> {
        let response = self.api.http_client().get(download_url).send().await?;
        if !response.status().is_success() {
            return Err(ServerApiError::Other(format!(
                "下载文件失败: HTTP {}",
                response.status()
            )));
        }
        Ok(response.bytes().await?.to_vec())
    }
}

/// 确定上传使用的 MIME 类型：优先调用方指定，缺省时按扩展名推断
fn resolve_content_type(req: &FileUploadRequest) -> Option<String> {
    if let Some(content_type) = &req.content_type {
        if !content_type.trim().is_empty() {
            return Some(content_type.trim().to_string());
        }
    }
    Some(guess_mime(&req.file_name))
}

/// 按文件扩展名推断 MIME 类型，无法识别时返回 `application/octet-stream`
fn guess_mime(file_name: &str) -> String {
    let ext = file_name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "txt" | "log" | "md" => "text/plain",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
    .to_string()
}
