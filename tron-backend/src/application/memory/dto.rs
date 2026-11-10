use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRunContext {
    pub user_code: String,
    pub workspace_code: String,
}
impl MemoryRunContext {
    pub fn new(user_code: impl Into<String>, workspace_code: impl Into<String>) -> Self {
        Self {
            user_code: user_code.into(),
            workspace_code: workspace_code.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryScope {
    User,
    Space,
    WorkDomain,
}
impl MemoryScope {
    pub fn rag_type(self) -> &'static str {
        match self {
            Self::User => "USER",
            Self::Space => "SPACE",
            Self::WorkDomain => "AGENT",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecord {
    pub memory_code: String,
    pub user_code: String,
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
    pub enabled: bool,
    pub version: u32,
    pub source_refs: Vec<Value>,
    pub source_label: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub index_status: String,
    pub index_error: Option<String>,
    // Internal deduplication identity, never exposed by HTTP or tools.
    #[serde(skip_serializing)]
    pub semantic_key: String,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemorySearchCommand {
    pub query: String,
    pub scope: Option<MemoryScope>,
    pub scope_code: Option<String>,
    pub limit: Option<usize>,
}
#[derive(Debug, Clone, Default)]
pub struct MemorySaveCommand {
    pub memory_code: Option<String>,
    pub version: Option<u32>,
    pub scope: Option<MemoryScope>,
    pub scope_code: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub enabled: Option<bool>,
    pub semantic_key: Option<String>,
    pub source_refs: Vec<Value>,
    pub source_label: String,
    pub expires_at: Option<i64>,
}
#[derive(Debug, Clone)]
pub struct MemoryDeleteCommand {
    pub memory_code: String,
    pub version: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchResponse {
    pub items: Vec<MemoryItem>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub memory_code: String,
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
    pub source_refs: Vec<Value>,
    pub source_label: String,
    pub version: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySaveResponse {
    pub saved: bool,
    pub searchable: bool,
    pub action: String,
    pub record: MemoryRecord,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDeleteResponse {
    pub memory_code: String,
    pub deleted: bool,
}
#[derive(Debug, Clone)]
pub struct MemoryRecordQuery {
    pub scope: MemoryScope,
    pub scope_code: String,
    pub keyword: String,
    pub enabled: Option<bool>,
    pub current: usize,
    pub size: usize,
}
#[derive(Debug, Clone)]
pub struct MemoryRecordPage {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<MemoryRecord>,
}
