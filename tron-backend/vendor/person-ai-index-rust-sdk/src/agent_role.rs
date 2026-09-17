//! 角色仓库客户端
//!
//! 对应 Java: `client/agentRole/` 包

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext, PageResult};
use crate::error::Result;

// ─── 数据模型 ──────────────────────────────────────────────

/// 角色列表查询参数
///
/// 对应 Java: `AgentRoleRepositoryListQuery`
#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentRoleListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(rename = "page_size", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

impl AgentRoleListQuery {
    pub fn of(page: i32, page_size: i32) -> Self {
        Self {
            page: Some(page),
            page_size: Some(page_size),
            ..Default::default()
        }
    }
}

/// 角色项
///
/// 对应 Java: `AgentRoleRepositoryItem`
#[derive(Debug, Clone, Deserialize)]
pub struct AgentRoleItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "role_name", default)]
    pub role_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub industry: Option<String>,
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at", default)]
    pub updated_at: Option<String>,
}

/// 角色详情 (继承自 AgentRoleItem，额外包含下载次数)
///
/// 对应 Java: `AgentRoleRepositoryDetail`
#[derive(Debug, Clone, Deserialize)]
pub struct AgentRoleDetail {
    #[serde(flatten)]
    pub base: AgentRoleItem,
    #[serde(rename = "download_count", default)]
    pub download_count: Option<i32>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 角色仓库客户端
///
/// 对应 Java: `AgentRoleRepositoryClient`
///
/// 需要 Bearer token 认证
pub struct AgentRoleRepositoryClient {
    api: AuthorizedClientApi,
}

impl AgentRoleRepositoryClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 分页查询角色列表
    ///
    /// `GET /api/v1/agent-role-repositories?keyword=...&industry=...&page=...&page_size=...`
    pub async fn list(
        &self,
        context: &AuthorizedClientContext,
        query: &AgentRoleListQuery,
    ) -> Result<PageResult<AgentRoleItem>> {
        let request = self.api.new_authorized_request(
            Method::GET,
            "/agent-role-repositories",
            context,
        )?;
        let mut request = request.query(&[("page", query.page.unwrap_or(1).to_string())]);
        request = request.query(&[
            ("page_size", query.page_size.unwrap_or(20).to_string()),
        ]);
        if let Some(ref keyword) = query.keyword {
            request = request.query(&[("keyword", keyword.as_str())]);
        }
        if let Some(ref industry) = query.industry {
            request = request.query(&[("industry", industry.as_str())]);
        }

        self.api
            .execute_for_page(request, Some(context), "查询角色列表")
            .await
    }

    /// 获取角色详情
    ///
    /// `GET /api/v1/agent-role-repositories/{roleId}`
    pub async fn detail(
        &self,
        context: &AuthorizedClientContext,
        role_id: &str,
    ) -> Result<AgentRoleDetail> {
        let path = format!("/agent-role-repositories/{role_id}");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "获取角色详情")
            .await
    }
}
