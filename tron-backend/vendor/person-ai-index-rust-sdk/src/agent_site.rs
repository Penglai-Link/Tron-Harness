//! 智能体页面发布客户端
//!
//! 客户端应用通过本模块将智能体生成的静态网页发布到用户的二级域名下。
//! 每个用户拥有独立二级域名（`用户名.personai.chat`），其下可发布多个项目页面。
//!
//! - **子域名 = 用户名**：服务端从 JWT 中提取当前登录用户名作为二级域名前缀，客户端不传递子域名
//! - **项目名自动生成**：每次发布时服务端自动生成唯一的项目名称，格式为 `年月日_6位小写字母`
//!   （如 `20260810_abcdef`）
//! - **网站名称可选**：发布时可传 `site_name` 作为网站展示名称（最长 50 字符，服务端去除首尾空白），
//!   查询列表会返回；未设置时为 `None`

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::error::Result;

// ─── 数据模型 ──────────────────────────────────────────────

/// 静态资源文件
#[derive(Debug, Clone, Serialize)]
pub struct SiteFile {
    /// 文件相对路径，支持多层目录（如 `index.html`、`css/theme/dark.css`）
    pub path: String,
    /// 文件文本内容
    pub content: String,
}

impl SiteFile {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }
}

/// 发布智能体页面请求
#[derive(Debug, Clone, Serialize)]
pub struct PublishRequest {
    /// 静态资源文件列表（必填，不能为空），支持多层目录结构
    pub files: Vec<SiteFile>,
    /// 网站名称（可选，最长 50 字符，服务端去除首尾空白），用于客户端展示
    #[serde(rename = "site_name", skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
}

impl PublishRequest {
    pub fn of(files: Vec<SiteFile>) -> Self {
        Self {
            files,
            site_name: None,
        }
    }

    /// 设置网站名称（可选，最长 50 字符，服务端去除首尾空白）
    pub fn with_site_name(mut self, site_name: impl Into<String>) -> Self {
        self.site_name = Some(site_name.into());
        self
    }
}

/// 发布智能体页面响应
#[derive(Debug, Clone, Deserialize)]
pub struct PublishResult {
    /// 项目名称（服务端自动生成，格式 `年月日_6位小写字母`）
    #[serde(rename = "project_name", default)]
    pub project_name: Option<String>,
    /// 项目访问地址
    #[serde(default)]
    pub url: Option<String>,
    /// 当前登录用户名（二级域名前缀）
    #[serde(default)]
    pub username: Option<String>,
    /// 网站名称（发布时未传时为 `None`）
    #[serde(rename = "site_name", default)]
    pub site_name: Option<String>,
    /// 发布时间 (ISO8601)
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
}

/// 已发布项目信息
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectItem {
    /// 项目名称
    #[serde(rename = "project_name", default)]
    pub project_name: Option<String>,
    /// 项目访问地址
    #[serde(default)]
    pub url: Option<String>,
    /// 网站名称（未设置时为 `None`）
    #[serde(rename = "site_name", default)]
    pub site_name: Option<String>,
    /// 发布时间 (ISO8601)
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
}

/// 项目列表查询结果
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectListResult {
    /// 当前登录用户名
    #[serde(default)]
    pub username: Option<String>,
    /// 用户二级域名根地址
    #[serde(default)]
    pub url: Option<String>,
    /// 已发布项目列表
    #[serde(default)]
    pub projects: Option<Vec<ProjectItem>>,
}

/// 项目导出文件信息
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectFile {
    /// 文件相对路径（相对项目根目录，如 index.html、css/theme/dark.css）
    #[serde(default)]
    pub path: Option<String>,
    /// 文件文本内容 (UTF-8)
    #[serde(default)]
    pub content: Option<String>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 智能体页面发布客户端
///
/// 需要 `Authorization: Bearer {token}` + `X-Client-Id: {clientId}` 认证
pub struct AgentSiteClient {
    api: AuthorizedClientApi,
}

impl AgentSiteClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 发布智能体页面
    ///
    /// `POST /api/v1/agent-sites/publish`
    ///
    /// 将智能体生成的静态页面写入用户二级域名目录。
    /// 服务端自动生成项目名（`年月日_6位小写字母`），子域名取自当前登录用户名；
    /// 可选通过 [`PublishRequest::with_site_name`] 指定网站展示名称。
    pub async fn publish(
        &self,
        context: &AuthorizedClientContext,
        req: &PublishRequest,
    ) -> Result<PublishResult> {
        let request = self
            .api
            .new_gateway_request(Method::POST, "/agent-sites/publish", context)?
            .json(req);
        self.api
            .execute_for_object(request, Some(context), "发布智能体页面")
            .await
    }

    /// 查询项目列表
    ///
    /// `GET /api/v1/agent-sites`
    ///
    /// 查询当前登录用户已发布的所有项目。
    pub async fn list(&self, context: &AuthorizedClientContext) -> Result<ProjectListResult> {
        let request = self
            .api
            .new_gateway_request(Method::GET, "/agent-sites", context)?;
        self.api
            .execute_for_object(request, Some(context), "查询项目列表")
            .await
    }

    /// 删除项目
    ///
    /// `DELETE /api/v1/agent-sites/{project_name}`
    ///
    /// 递归删除当前登录用户下的指定项目目录，仅能删除自己的项目。
    pub async fn delete(
        &self,
        context: &AuthorizedClientContext,
        project_name: &str,
    ) -> Result<()> {
        let path = format!("/agent-sites/{project_name}");
        let request = self
            .api
            .new_gateway_request(Method::DELETE, &path, context)?;
        self.api
            .execute_for_success(request, Some(context), "删除项目")
            .await
    }

    /// 导出项目文件列表
    ///
    /// `GET /api/v1/agent-sites/{project_name}/files`
    ///
    /// 返回当前登录用户指定项目下所有文件的相对路径与文本内容，
    /// 供客户端打包为本地压缩包（仅能导出自己的项目）。
    pub async fn export_files(
        &self,
        context: &AuthorizedClientContext,
        project_name: &str,
    ) -> Result<Vec<ProjectFile>> {
        let path = format!("/agent-sites/{project_name}/files");
        let request = self
            .api
            .new_gateway_request(Method::GET, &path, context)?;
        self.api
            .execute_for_list(request, Some(context), "导出项目文件")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 未设置网站名称时，发布请求 JSON 不应包含 site_name 字段
    #[test]
    fn should_omit_site_name_when_not_set() {
        let req = PublishRequest::of(vec![SiteFile::new("index.html", "<h1>Hi</h1>")]);
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("site_name").is_none());
        assert!(json.get("files").is_some());
    }

    /// 设置网站名称后，发布请求 JSON 应包含 snake_case 的 site_name 字段
    #[test]
    fn should_serialize_site_name_when_set() {
        let req = PublishRequest::of(vec![SiteFile::new("index.html", "<h1>Hi</h1>")])
            .with_site_name("我的智能体小站");
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["site_name"], "我的智能体小站");
    }

    /// 发布响应与项目列表应能反序列化 site_name，字段缺失时应为 None
    #[test]
    fn should_deserialize_site_name_when_present_or_missing() {
        let result: PublishResult = serde_json::from_value(serde_json::json!({
            "project_name": "20260810_abcdef",
            "url": "https://john.personai.chat/20260810_abcdef/",
            "username": "john",
            "site_name": "我的智能体小站",
            "created_at": "2026-08-10T14:30:00+08:00"
        }))
        .unwrap();
        assert_eq!(result.site_name.as_deref(), Some("我的智能体小站"));

        let item: ProjectItem =
            serde_json::from_value(serde_json::json!({ "project_name": "20260810_abcdef" }))
                .unwrap();
        assert!(item.site_name.is_none());
    }
}
