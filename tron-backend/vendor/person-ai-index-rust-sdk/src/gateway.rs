//! 网关客户端 - 对话补全、检索、向量嵌入
//!
//! 对应 Java: `client/gateway/` 包
//!
//! 所有网关客户端使用 `ClientEnvelopeEncryptor` 加密请求体，
//! 并携带 `Authorization` + `X-Client-Id` 头。

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::base::{AuthorizedClientApi, AuthorizedClientContext};
use crate::crypto::ClientEnvelopeEncryptor;
use crate::error::Result;

// ─── 数据模型 ──────────────────────────────────────────────

/// 质量类型
///
/// 对应 Java: `QualityType`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QualityType {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "middle")]
    Middle,
    #[serde(rename = "low")]
    Low,
}

impl QualityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Middle => "middle",
            Self::Low => "low",
        }
    }
}

/// 公开模型信息（模型列表查询返回值）
///
/// 对应 Java: `ChatModelPublicResponse`
#[derive(Debug, Clone, Deserialize)]
pub struct ChatModelInfo {
    /// 模型编码（可填入对话补全请求体 `model` 字段）
    pub model_code: String,
    /// 模型名称（展示用）
    #[serde(default)]
    pub model_name: Option<String>,
    /// 厂商编码（如 deepseek/glm/gpt，`gpt` 为兼容历史数据的 OpenAI 编码）
    #[serde(default)]
    pub provider_code: Option<String>,
    /// 厂商展示名（服务端按厂商目录反查，如 DeepSeek/GLM/OpenAI；目录未收录时回退厂商编码）
    #[serde(default)]
    pub provider_name: Option<String>,
    /// 模型最大上下文长度（Token 数）
    #[serde(default)]
    pub context_length: Option<i32>,
    /// 输入系数（对话计费按未命中缓存的输入 token 计算；未配置时服务端兜底 0.1）
    #[serde(default)]
    pub input_coefficient: Option<f64>,
    /// 缓存命中系数（对话计费按缓存命中的输入 token 计算；未配置时服务端兜底 0.1）
    #[serde(default)]
    pub cache_hit_coefficient: Option<f64>,
    /// 输出系数（对话计费按输出 token 计算；embedding 等非对话场景消耗积分 = token 数 × 输出系数；
    /// 未配置时服务端兜底 0.1）
    #[serde(default)]
    pub output_coefficient: Option<f64>,
}

/// 检索结果项
///
/// 对应 Java: `SearchResult`
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

/// 检索响应
///
/// 对应 Java: `SearchResponse`
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub count: Option<i32>,
    #[serde(default)]
    pub results: Option<Vec<SearchResult>>,
    #[serde(rename = "provider_code", default)]
    pub provider_code: Option<String>,
}

// ─── 对话补全客户端 ────────────────────────────────────────

/// 对话补全网关客户端
///
/// 对应 Java: `ChatCompletionsClient`
pub struct ChatCompletionsClient {
    api: AuthorizedClientApi,
}

impl ChatCompletionsClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 非流式对话补全
    ///
    /// `POST /api/v1/chat/completions` (加密请求体)
    ///
    /// `raw_json` 为原始请求 JSON，若不含 `quality_type` 字段则自动注入。
    /// 若需显式指定模型，在 `raw_json` 的 `model` 字段传入模型编码
    /// （见 [`Self::list_models`]，编码不存在或未启用时服务端返回 400）。
    /// `conversation_id` 为会话标识：多轮对话（如 React 循环）应传入同一会话 ID，
    /// 服务端会将该会话固定路由到同一模型，提高前缀缓存命中率、降低 Token 消耗；
    /// 若 `raw_json` 已包含 `conversation_id` 字段则以其为准
    pub async fn create(
        &self,
        context: &AuthorizedClientContext,
        raw_json: &str,
        quality: Option<QualityType>,
        conversation_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        // 注入 quality_type 和 conversation_id
        let mut json: serde_json::Value =
            serde_json::from_str(raw_json).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(q) = quality {
            if json.get("quality_type").is_none() {
                json["quality_type"] = serde_json::Value::String(q.as_str().to_string());
            }
        }
        if let Some(cid) = conversation_id {
            if json.get("conversation_id").is_none() {
                json["conversation_id"] = serde_json::Value::String(cid.to_string());
            }
        }
        let raw = serde_json::to_string(&json)?;

        // 加密请求体
        let envelope = ClientEnvelopeEncryptor::encrypt_with_context(context, &raw)?;

        let request = self
            .api
            .new_gateway_request(Method::POST, "/chat/completions", context)?
            .json(&envelope);

        self.api
            .execute_for_raw_json(request, Some(context), "对话补全")
            .await
    }

    /// 流式对话补全
    ///
    /// `POST /api/v1/chat/completions` (加密请求体，SSE 响应)
    ///
    /// 通过 `on_data` 回调逐条返回 SSE `data:` 载荷。
    /// 若需显式指定模型，在 `raw_json` 的 `model` 字段传入模型编码
    /// （见 [`Self::list_models`]，编码不存在或未启用时服务端返回 400）。
    /// `conversation_id` 为会话标识：多轮对话（如 React 循环）应传入同一会话 ID，
    /// 服务端会将该会话固定路由到同一模型，提高前缀缓存命中率、降低 Token 消耗；
    /// 若 `raw_json` 已包含 `conversation_id` 字段则以其为准
    pub async fn create_stream<F>(
        &self,
        context: &AuthorizedClientContext,
        raw_json: &str,
        quality: QualityType,
        conversation_id: Option<&str>,
        on_data: F,
    ) -> Result<()>
    where
        F: FnMut(&str) -> Result<()>,
    {
        // 注入 quality_type、stream=true 和 conversation_id
        let mut json: serde_json::Value =
            serde_json::from_str(raw_json).unwrap_or_else(|_| serde_json::json!({}));
        json["quality_type"] = serde_json::Value::String(quality.as_str().to_string());
        json["stream"] = serde_json::Value::Bool(true);
        if let Some(cid) = conversation_id {
            if json.get("conversation_id").is_none() {
                json["conversation_id"] = serde_json::Value::String(cid.to_string());
            }
        }
        let raw = serde_json::to_string(&json)?;

        // 加密请求体
        let envelope = ClientEnvelopeEncryptor::encrypt_with_context(context, &raw)?;

        let request = self
            .api
            .new_gateway_request(Method::POST, "/chat/completions", context)?
            .json(&envelope);

        self.api
            .execute_for_stream(request, Some(context), "流式对话补全", on_data)
            .await
    }

    /// 查询可用模型列表
    ///
    /// `GET /api/v1/chat/models` (仅需用户 Bearer Token，无需加密信封与 `X-Client-Id`)
    ///
    /// 返回**启用状态**模型的去重列表（同一模型编码配置多条时仅保留一条）。
    /// 调用对话补全前可先通过此接口获取可用的 `model_code`，
    /// 再填入 `create`/`create_stream` 的 `raw_json.model` 字段显式指定模型。
    ///
    /// `quality_type` 为可选模型等级过滤（high/middle/low），未传时按服务端默认低质量处理；
    /// 模型等级出现变化时，可通过响应中的 `context_length` 感知模型上下文容量。
    ///
    /// 计费系数：对话消耗积分按分段系数合计：(输入 token − 缓存命中 token) × `input_coefficient`
    /// + 缓存命中 token × `cache_hit_coefficient` + 输出 token × `output_coefficient`；
    /// embedding 等非对话场景消耗积分 = token 数 × `output_coefficient`。
    /// 各系数未配置时服务端按 0.1 兜底返回，可直接用于消耗估算。
    pub async fn list_models(
        &self,
        context: &AuthorizedClientContext,
        quality_type: Option<QualityType>,
    ) -> Result<Vec<ChatModelInfo>> {
        let mut request = self.api.new_authorized_request(
            Method::GET,
            "/chat/models",
            context,
        )?;
        if let Some(q) = quality_type {
            request = request.query(&[("quality_type", q.as_str())]);
        }
        self.api
            .execute_for_list(request, Some(context), "模型列表查询")
            .await
    }
}

// ─── 检索客户端 ────────────────────────────────────────────

/// 检索网关客户端
///
/// 对应 Java: `SearchClient`
pub struct SearchClient {
    api: AuthorizedClientApi,
}

impl SearchClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 检索
    ///
    /// `POST /api/v1/search` (加密请求体)
    ///
    /// `query` 检索词，`count` 结果数量 [1, 20]，默认 10
    pub async fn search(
        &self,
        context: &AuthorizedClientContext,
        query: &str,
        count: Option<i32>,
    ) -> Result<SearchResponse> {
        let count = count.unwrap_or(10).clamp(1, 20);
        let request_body = serde_json::json!({
            "query": query,
            "count": count,
        });
        let raw = serde_json::to_string(&request_body)?;

        // 加密请求体
        let envelope = ClientEnvelopeEncryptor::encrypt_with_context(context, &raw)?;

        let request = self
            .api
            .new_gateway_request(Method::POST, "/search", context)?
            .json(&envelope);

        self.api
            .execute_for_object(request, Some(context), "检索")
            .await
    }
}

// ─── 向量嵌入客户端 ────────────────────────────────────────

/// 向量嵌入网关客户端
///
/// 对应 Java: `EmbeddingsClient`
pub struct EmbeddingsClient {
    api: AuthorizedClientApi,
}

impl EmbeddingsClient {
    pub fn new(server_base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api: AuthorizedClientApi::new(server_base_url)?,
        })
    }

    /// 创建向量嵌入
    ///
    /// `POST /api/v1/embeddings` (加密请求体)
    pub async fn create(
        &self,
        context: &AuthorizedClientContext,
        raw_json: &str,
    ) -> Result<serde_json::Value> {
        // 加密请求体
        let envelope = ClientEnvelopeEncryptor::encrypt_with_context(context, raw_json)?;

        let request = self
            .api
            .new_gateway_request(Method::POST, "/embeddings", context)?
            .json(&envelope);

        self.api
            .execute_for_raw_json(request, Some(context), "向量嵌入")
            .await
    }
}
