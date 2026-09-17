//! 技能仓库客户端
//!
//! 对应 Java: `client/skill/` 包

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext, PageResult};
use crate::error::Result;

// ─── 数据模型 ──────────────────────────────────────────────

/// 技能列表查询参数
///
/// 对应 Java: `SkillListQuery`
#[derive(Debug, Clone, Serialize, Default)]
pub struct SkillListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(rename = "page_size", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

impl SkillListQuery {
    pub fn of(page: i32, page_size: i32) -> Self {
        Self {
            page: Some(page),
            page_size: Some(page_size),
            ..Default::default()
        }
    }
}

/// 技能项
///
/// 对应 Java: `SkillItem`
#[derive(Debug, Clone, Deserialize)]
pub struct SkillItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "skill_name", default)]
    pub skill_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(rename = "source_type", default)]
    pub source_type: Option<String>,
    #[serde(rename = "source_git_url", default)]
    pub source_git_url: Option<String>,
    /// 累计下载次数
    #[serde(rename = "download_count", default)]
    pub download_count: Option<i32>,
}

/// 技能资源项
///
/// 对应 Java: `SkillResourceItem`
#[derive(Debug, Clone, Deserialize)]
pub struct SkillResourceItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "skill_id", default)]
    pub skill_id: Option<String>,
    #[serde(rename = "parent_id", default)]
    pub parent_id: Option<String>,
    #[serde(rename = "resource_type", default)]
    pub resource_type: Option<i32>, // 0=目录, 1=文件
    #[serde(rename = "resource_name", default)]
    pub resource_name: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at", default)]
    pub updated_at: Option<String>,
}

impl SkillResourceItem {
    pub const TYPE_DIRECTORY: i32 = 0;
    pub const TYPE_FILE: i32 = 1;

    pub fn is_file(&self) -> bool {
        self.resource_type == Some(Self::TYPE_FILE)
    }

    pub fn is_directory(&self) -> bool {
        self.resource_type == Some(Self::TYPE_DIRECTORY)
    }
}

/// 技能下载结果
///
/// 对应 Java: `SkillDownloadResult`
#[derive(Debug, Clone, Deserialize)]
pub struct SkillDownloadResult {
    #[serde(default)]
    pub skill: Option<SkillItem>,
    #[serde(default)]
    pub resources: Option<Vec<SkillResourceItem>>,
    #[serde(rename = "download_count", default)]
    pub download_count: Option<i32>,
}

// ─── 客户端 ────────────────────────────────────────────────

/// 技能仓库客户端
///
/// 对应 Java: `SkillRepositoryClient`
///
/// 需要 Bearer token 认证 (不需要 X-Client-Id)
pub struct SkillRepositoryClient {
    api: AuthorizedClientApi,
}

impl SkillRepositoryClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 分页查询技能列表
    ///
    /// `GET /api/v1/skills?keyword=...&tag=...&page=...&page_size=...`
    pub async fn list(
        &self,
        context: &AuthorizedClientContext,
        query: &SkillListQuery,
    ) -> Result<PageResult<SkillItem>> {
        let request = self
            .api
            .new_authorized_request(Method::GET, "/skills", context)?;
        let mut request = request.query(&[("page", query.page.unwrap_or(1).to_string())]);
        request = request.query(&[
            ("page_size", query.page_size.unwrap_or(20).to_string()),
        ]);
        if let Some(ref keyword) = query.keyword {
            request = request.query(&[("keyword", keyword.as_str())]);
        }
        if let Some(ref tag) = query.tag {
            request = request.query(&[("tag", tag.as_str())]);
        }

        self.api
            .execute_for_page(request, Some(context), "查询技能列表")
            .await
    }

    /// 获取技能详情
    ///
    /// `GET /api/v1/skills/{skillId}`
    pub async fn detail(
        &self,
        context: &AuthorizedClientContext,
        skill_id: &str,
    ) -> Result<SkillItem> {
        let path = format!("/skills/{skill_id}");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_object(request, Some(context), "获取技能详情")
            .await
    }

    /// 获取技能资源列表
    ///
    /// `GET /api/v1/skills/{skillId}/resources`
    pub async fn resources(
        &self,
        context: &AuthorizedClientContext,
        skill_id: &str,
    ) -> Result<Vec<SkillResourceItem>> {
        let path = format!("/skills/{skill_id}/resources");
        let request = self
            .api
            .new_authorized_request(Method::GET, &path, context)?;
        self.api
            .execute_for_list(request, Some(context), "获取技能资源")
            .await
    }

    /// 下载技能
    ///
    /// `POST /api/v1/skills/{skillId}/download`
    pub async fn download(
        &self,
        context: &AuthorizedClientContext,
        skill_id: &str,
    ) -> Result<SkillDownloadResult> {
        let path = format!("/skills/{skill_id}/download");
        let request = self
            .api
            .new_authorized_request(Method::POST, &path, context)?
            .json(&serde_json::json!({}));
        self.api
            .execute_for_object(request, Some(context), "下载技能")
            .await
    }
}
