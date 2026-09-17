//! 版本查询与下载客户端
//!
//! 客户端应用通过本模块实现「检查更新 + 下载安装包」：
//! 查询最新版本号与各平台安装包信息，再按需获取签名下载地址。
//!
//! 接口返回的 `download_url` 为带签名的临时下载地址，应实时获取后立即使用，不要缓存复用。

use reqwest::Method;
use serde::Deserialize;

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::Result;

// ─── 产品编码 ──────────────────────────────────────────────

/// 默认产品编码（未指定 product_code 时服务端按此查询）
pub const DEFAULT_PRODUCT_CODE: &str = "person-ai";

// ─── os_type 平台编码 ──────────────────────────────────────

/// os_type 平台编码约定
pub mod os_type {
    /// 通用
    pub const GENERIC: i32 = 0;
    /// Windows x86 32-bit
    pub const WINDOWS_X86: i32 = 110;
    /// Windows x86_64 64-bit
    pub const WINDOWS_X64: i32 = 120;
    /// Windows arm64 64-bit
    pub const WINDOWS_ARM64: i32 = 130;
    /// macOS x86_64 64-bit
    pub const MACOS_X64: i32 = 220;
    /// macOS arm64 64-bit
    pub const MACOS_ARM64: i32 = 230;
    /// Linux x86 32-bit
    pub const LINUX_X86: i32 = 310;
    /// Linux x86_64 64-bit
    pub const LINUX_X64: i32 = 320;
    /// Linux arm64 64-bit
    pub const LINUX_ARM64: i32 = 330;
    /// Android arm 32-bit
    pub const ANDROID_ARM: i32 = 410;
    /// Android arm64 64-bit
    pub const ANDROID_ARM64: i32 = 420;
    /// Android x86_64 64-bit
    pub const ANDROID_X64: i32 = 430;
    /// iOS arm64 64-bit
    pub const IOS_ARM64: i32 = 510;
}

// ─── 数据模型 ──────────────────────────────────────────────

/// 安装包信息
#[derive(Debug, Clone, Deserialize)]
pub struct VersionPackage {
    /// 安装包 ID
    #[serde(default)]
    pub id: Option<String>,
    /// 平台类型编码 (见 [`os_type`])
    #[serde(default)]
    pub os_type: Option<i32>,
    /// 签名下载地址 (临时有效，不要缓存复用)
    #[serde(rename = "download_url", default)]
    pub download_url: Option<String>,
    /// 安装包原始文件名 (可为空)
    #[serde(rename = "file_name", default)]
    pub file_name: Option<String>,
}

/// 最新版本信息
#[derive(Debug, Clone, Deserialize)]
pub struct LatestVersion {
    /// 版本 ID
    #[serde(default)]
    pub id: Option<String>,
    /// 版本号，如 `v1.2.0`
    #[serde(rename = "version_number", default)]
    pub version_number: Option<String>,
    /// 发布日期
    #[serde(rename = "release_date", default)]
    pub release_date: Option<String>,
    /// 安装包列表
    #[serde(default)]
    pub packages: Option<Vec<VersionPackage>>,
}

/// 下载地址结果
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadUrl {
    /// 签名下载地址
    #[serde(rename = "download_url", default)]
    pub download_url: Option<String>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 版本查询与下载客户端
///
/// 需要 Bearer token 认证
pub struct VersionClient {
    api: AuthorizedClientApi,
}

impl VersionClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 查询最新版本及安装包 (检查更新)
    ///
    /// `GET /api/v1/versions/latest`
    ///
    /// 按发布日期倒序取最新一条；无可用版本时服务端返回 404。
    /// 未指定产品编码时服务端默认按 `person-ai` 查询。
    pub async fn latest(&self, context: &AuthorizedClientContext) -> Result<LatestVersion> {
        self.latest_with_product_code(context, None).await
    }

    /// 按产品编码查询最新版本及安装包
    ///
    /// `GET /api/v1/versions/latest?product_code=...`
    ///
    /// `product_code` 传 `None` 时等价于 [`Self::latest`]（服务端默认 `person-ai`）
    pub async fn latest_with_product_code(
        &self,
        context: &AuthorizedClientContext,
        product_code: Option<&str>,
    ) -> Result<LatestVersion> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/versions/latest", context)?;
        let request = match product_code {
            Some(code) => request.query(&[("product_code", code)]),
            None => request,
        };
        self.api
            .execute_for_object(request, Some(context), "查询最新版本")
            .await
    }

    /// 查询指定版本安装包列表
    ///
    /// `GET /api/v1/versions/{versionId}/packages`
    pub async fn packages(
        &self,
        context: &AuthorizedClientContext,
        version_id: &str,
    ) -> Result<Vec<VersionPackage>> {
        let path = format!("/versions/{version_id}/packages");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_list(request, Some(context), "查询版本安装包列表")
            .await
    }

    /// 查询指定版本按平台下载地址
    ///
    /// `GET /api/v1/versions/{versionId}/download?os_type=...`
    ///
    /// 成功返回时该版本下载计数自动 +1
    pub async fn download(
        &self,
        context: &AuthorizedClientContext,
        version_id: &str,
        os_type: i32,
    ) -> Result<DownloadUrl> {
        let path = format!("/versions/{version_id}/download");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?
            .query(&[("os_type", os_type.to_string())]);
        self.api
            .execute_for_object(request, Some(context), "查询下载地址")
            .await
    }

    /// 上报下载计数
    ///
    /// `POST /api/v1/versions/{versionId}/download-count`
    ///
    /// 客户端通过 `latest` / `packages` 拿到地址并实际完成下载后，单独上报一次
    /// 下载计数（`download` 接口已自动计数，无需重复上报）
    pub async fn report_download_count(
        &self,
        context: &AuthorizedClientContext,
        version_id: &str,
    ) -> Result<()> {
        let path = format!("/versions/{version_id}/download-count");
        let request = self
            .api
            .new_authorized_request(Method::POST, &path, context)?;
        self.api
            .execute_for_success(request, Some(context), "上报下载计数")
            .await
    }
}
