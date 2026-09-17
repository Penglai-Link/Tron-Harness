//! 皮肤市场客户端
//!
//! 对应服务端 API: `/api/v1/skins/market`

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext, PageResult};
use crate::error::{Result, ServerApiError};

const MAX_SKIN_ZIP_BYTES: u64 = 20 * 1024 * 1024;

// ─── 数据模型 ──────────────────────────────────────────────

/// 皮肤市场列表查询参数
#[derive(Debug, Clone, Serialize, Default)]
pub struct SkinMarketQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(rename = "page_size", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

impl SkinMarketQuery {
    pub fn of(page: i32, page_size: i32) -> Self {
        Self {
            page: Some(page),
            page_size: Some(page_size),
            ..Default::default()
        }
    }
}

/// 皮肤市场列表项（不含 description）
#[derive(Debug, Clone, Deserialize)]
pub struct SkinMarketItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub preview_url: Option<String>,
    #[serde(default)]
    pub file_size: Option<i64>,
    #[serde(default)]
    pub download_count: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// 皮肤详情（含 description、file_url、updated_at）
#[derive(Debug, Clone, Deserialize)]
pub struct SkinDetail {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub preview_url: Option<String>,
    #[serde(default)]
    pub file_url: Option<String>,
    #[serde(default)]
    pub file_size: Option<i64>,
    #[serde(default)]
    pub download_count: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// 皮肤下载结果
#[derive(Debug, Clone, Deserialize)]
pub struct SkinDownloadResult {
    #[serde(default)]
    pub file_url: Option<String>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 皮肤市场客户端
///
/// 需要 Bearer token 认证
pub struct SkinMarketClient {
    api: AuthorizedClientApi,
}

impl SkinMarketClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 分页查询皮肤市场列表
    ///
    /// `GET /api/v1/skins/market?keyword=...&tags=...&page=...&page_size=...`
    pub async fn list(
        &self,
        context: &AuthorizedClientContext,
        query: &SkinMarketQuery,
    ) -> Result<PageResult<SkinMarketItem>> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/skins/market", context)?;
        let mut request = request.query(&[("page", query.page.unwrap_or(1).to_string())]);
        request = request.query(&[
            ("page_size", query.page_size.unwrap_or(20).to_string()),
        ]);
        if let Some(ref keyword) = query.keyword {
            request = request.query(&[("keyword", keyword.as_str())]);
        }
        if let Some(ref tags) = query.tags {
            request = request.query(&[("tags", tags.as_str())]);
        }

        self.api
            .execute_for_page(request, Some(context), "查询皮肤市场列表")
            .await
    }

    /// 获取皮肤详情
    ///
    /// `GET /api/v1/skins/market/{id}`
    pub async fn detail(
        &self,
        context: &AuthorizedClientContext,
        skin_id: &str,
    ) -> Result<SkinDetail> {
        let path = format!("/skins/market/{skin_id}");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "获取皮肤详情")
            .await
    }

    /// 下载皮肤（下载次数 +1，返回临时下载 URL）
    ///
    /// `POST /api/v1/skins/market/{id}/download`
    pub async fn download(
        &self,
        context: &AuthorizedClientContext,
        skin_id: &str,
    ) -> Result<SkinDownloadResult> {
        let path = format!("/skins/market/{skin_id}/download");
        let request = self
            .api
            .new_authorized_request(Method::POST, &path, context)?
            .json(&serde_json::json!({}));
        self.api
            .execute_for_object(request, Some(context), "下载皮肤")
            .await
    }

    /// 根据下载 URL 拉取皮肤 zip 字节（上限 20MB）
    pub async fn download_zip(&self, file_url: &str) -> Result<Vec<u8>> {
        let response = self.api.http_client().get(file_url).send().await?;
        if !response.status().is_success() {
            return Err(ServerApiError::Other(format!(
                "下载皮肤包失败: HTTP {}",
                response.status()
            )));
        }
        if let Some(len) = response.content_length() {
            if len > MAX_SKIN_ZIP_BYTES {
                return Err(ServerApiError::Other(format!(
                    "皮肤包大小{}字节，超过上限{}字节",
                    len, MAX_SKIN_ZIP_BYTES
                )));
            }
        }
        let bytes = response.bytes().await?;
        if bytes.len() as u64 > MAX_SKIN_ZIP_BYTES {
            return Err(ServerApiError::Other(format!(
                "皮肤包大小{}字节，超过上限{}字节",
                bytes.len(),
                MAX_SKIN_ZIP_BYTES
            )));
        }
        Ok(bytes.to_vec())
    }
}
