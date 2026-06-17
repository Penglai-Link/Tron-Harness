use serde::Serialize;
use serde_json::Value;

use super::work_domain::DEFAULT_WORK_DOMAIN_CODE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRunContext {
    pub user_code: String,
    pub workspace_code: String,
    pub work_domain_code: String,
}

impl MemoryRunContext {
    pub fn new(user_code: impl Into<String>, workspace_code: impl Into<String>) -> Self {
        Self {
            user_code: user_code.into(),
            workspace_code: workspace_code.into(),
            work_domain_code: DEFAULT_WORK_DOMAIN_CODE.into(),
        }
    }

    pub fn with_work_domain_code(mut self, work_domain_code: impl Into<String>) -> Self {
        self.work_domain_code = work_domain_code.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySearchCommand {
    pub query: Option<String>,
    pub mode: Option<String>,
    pub memory_code: Option<String>,
    pub scope: Option<String>,
    pub scope_code: Option<String>,
    pub memory_type: Option<Value>,
    pub memory_status: Option<Value>,
    pub review_status: Option<Value>,
    pub injectable: Option<Value>,
    pub limit: Option<usize>,
    pub token_budget: Option<usize>,
    pub trace: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySaveCommand {
    pub memory_type: Option<String>,
    pub text: Option<String>,
    pub memory_code: Option<String>,
    pub scope: Option<String>,
    pub scope_code: Option<String>,
    pub title: Option<String>,
    pub semantic_key: Option<String>,
    pub memory_status: Option<String>,
    pub review_status: Option<String>,
    pub injectable: Option<bool>,
    pub importance_score: Option<Value>,
    pub confidence_score: Option<Value>,
    pub quality_score: Option<Value>,
    pub expires_at: Option<i64>,
    pub derived_source_type: Option<String>,
    pub derived_source_code: Option<String>,
    pub source_refs: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryDeleteCommand {
    pub memory_code: String,
    pub reason: String,
    pub delete_mode: Option<String>,
    pub source_refs: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchResponse {
    pub success: bool,
    pub items: Vec<MemoryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_pack: Option<MemoryContextPack>,
    pub count: usize,
    pub warnings: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub memory_code: String,
    pub scope: String,
    pub scope_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injectable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance_score: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_source_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    #[serde(skip_serializing_if = "is_empty_metadata")]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryContextPack {
    pub items: Vec<MemoryContextPackItem>,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryContextPackItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_code: Option<String>,
    pub text: String,
    pub tokens: usize,
    #[serde(skip_serializing_if = "is_empty_metadata")]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySaveResponse {
    pub success: bool,
    pub action: String,
    pub memory_code: String,
    pub scope: String,
    pub scope_code: String,
    pub memory_type: Option<String>,
    pub memory_status: Option<String>,
    pub review_status: Option<String>,
    pub injectable: Option<bool>,
    pub rag: MemoryRagWriteResult,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDeleteResponse {
    pub success: bool,
    pub action: String,
    pub memory_code: String,
    pub scope: String,
    pub scope_code: String,
    pub memory_type: Option<String>,
    pub memory_status: Option<String>,
    pub review_status: Option<String>,
    pub injectable: Option<bool>,
    pub deleted: bool,
    pub delete_reason: String,
    pub warnings: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRagWriteResult {
    pub status: String,
    pub text_unit_count: usize,
    pub fulltext_status: String,
    pub embedding_status: String,
    pub warnings: Vec<Value>,
}

fn is_empty_metadata(value: &Value) -> bool {
    value
        .as_object()
        .map(|object| object.is_empty())
        .unwrap_or_else(|| value.is_null())
}
