//! 多智能体仓库客户端
//!
//! 对应 Java: `client/repository/` 包

use reqwest::Method;
use serde::Deserialize;

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::Result;

// ─── 数据模型 ──────────────────────────────────────────────

/// 多智能体仓库项
///
/// 对应 Java: `MultiAgentRepositoryItem`
#[derive(Debug, Clone, Deserialize)]
pub struct MultiAgentRepositoryItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "cluster_name", default)]
    pub cluster_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "preview_image_url", default)]
    pub preview_image_url: Option<String>,
    #[serde(rename = "download_count", default)]
    pub download_count: Option<i32>,
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
}

/// 上传请求
///
/// 对应 Java: `MultiAgentRepositoryUploadRequest`
#[derive(Debug, Clone)]
pub struct UploadRequest {
    pub cluster_name: String,       // 必填
    pub file_name: String,          // 必填
    pub file_bytes: Vec<u8>,        // 必填
    pub content_type: Option<String>,
    pub description: Option<String>,
    pub preview_image_url: Option<String>,
}

/// 上传结果 (继承自 MultiAgentRepositoryItem)
pub type UploadResult = MultiAgentRepositoryItem;

/// 下载结果
///
/// 对应 Java: `MultiAgentRepositoryDownloadResult`
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadResult {
    #[serde(rename = "download_url")]
    pub download_url: String,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 多智能体仓库客户端
///
/// 对应 Java: `MultiAgentRepositoryClient`
///
/// 注意: `list` 接口无需认证，`upload`/`download` 需要 Bearer + Client-Id
pub struct MultiAgentRepositoryClient {
    api: AuthorizedClientApi,
}

impl MultiAgentRepositoryClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 查询多智能体仓库列表 (无需认证)
    ///
    /// `GET /api/v1/multi-agent-repositories?keyword=...`
    pub async fn list(&self, keyword: Option<&str>) -> Result<Vec<MultiAgentRepositoryItem>> {
        let mut request = self.api.new_api_request(Method::GET, "/multi-agent-repositories");
        if let Some(kw) = keyword {
            request = request.query(&[("keyword", kw)]);
        }
        self.api
            .execute_for_list(request, None, "查询多智能体仓库列表")
            .await
    }

    /// 上传多智能体配置 (multipart/form-data)
    ///
    /// `POST /api/v1/multi-agent-repositories/upload`
    ///
    /// 需要 Bearer + Client-Id
    pub async fn upload(
        &self,
        context: &AuthorizedClientContext,
        req: &UploadRequest,
    ) -> Result<UploadResult> {
        use reqwest::multipart;

        let mut form = multipart::Form::new()
            .text("cluster_name", req.cluster_name.clone())
            .part(
                "file",
                multipart::Part::bytes(req.file_bytes.clone())
                    .file_name(req.file_name.clone())
                    .mime_str(req.content_type.as_deref().unwrap_or("application/octet-stream"))
                    .map_err(|e| crate::error::ServerApiError::other(format!("设置MIME失败: {e}")))?,
            );

        if let Some(ref desc) = req.description {
            form = form.text("description", desc.clone());
        }
        if let Some(ref url) = req.preview_image_url {
            form = form.text("preview_image_url", url.clone());
        }

        let request = self
            .api
            .new_gateway_request(
                Method::POST,
                "/multi-agent-repositories/upload",
                context,
            )?
            .multipart(form);

        self.api
            .execute_for_object(request, Some(context), "上传多智能体配置")
            .await
    }

    /// 获取下载地址
    ///
    /// `GET /api/v1/multi-agent-repositories/{repositoryId}/download`
    ///
    /// 需要 Bearer + Client-Id
    pub async fn download(
        &self,
        context: &AuthorizedClientContext,
        repository_id: &str,
    ) -> Result<DownloadResult> {
        let path = format!("/multi-agent-repositories/{repository_id}/download");
        let request = self
            .api
            .new_gateway_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "获取下载地址")
            .await
    }
}
