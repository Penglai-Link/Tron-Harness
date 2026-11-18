use crate::application::memory::{MemoryRecord, MemoryScope, MemoryWorkDomain};
use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageData<T> {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<T>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryMemoryRecordVo {
    pub memory_code: String,
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
    pub enabled: bool,
    pub version: u32,
    pub source_refs: Vec<serde_json::Value>,
    pub source_label: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub index_status: String,
    pub index_error: Option<String>,
}
impl From<MemoryRecord> for QueryMemoryRecordVo {
    fn from(r: MemoryRecord) -> Self {
        Self {
            memory_code: r.memory_code,
            scope: r.scope,
            scope_code: r.scope_code,
            title: r.title,
            text: r.text,
            enabled: r.enabled,
            version: r.version,
            source_refs: r.source_refs,
            source_label: r.source_label,
            expires_at: r.expires_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            index_status: r.index_status,
            index_error: r.index_error,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMemoryRecordVo {
    pub memory_code: String,
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
    pub enabled: bool,
    pub version: u32,
    pub source_refs: Vec<serde_json::Value>,
    pub source_label: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub index_status: String,
    pub index_error: Option<String>,
}
impl From<MemoryRecord> for GetMemoryRecordVo {
    fn from(r: MemoryRecord) -> Self {
        Self {
            memory_code: r.memory_code,
            scope: r.scope,
            scope_code: r.scope_code,
            title: r.title,
            text: r.text,
            enabled: r.enabled,
            version: r.version,
            source_refs: r.source_refs,
            source_label: r.source_label,
            expires_at: r.expires_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            index_status: r.index_status,
            index_error: r.index_error,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryRecordVo {
    pub saved: bool,
    pub searchable: bool,
    pub memory_code: String,
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
    pub enabled: bool,
    pub version: u32,
    pub source_refs: Vec<serde_json::Value>,
    pub source_label: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub index_status: String,
    pub index_error: Option<String>,
}
impl From<MemoryRecord> for CreateMemoryRecordVo {
    fn from(r: MemoryRecord) -> Self {
        let searchable = r.enabled && r.index_status == "Ready";
        Self {
            saved: true,
            searchable,
            memory_code: r.memory_code,
            scope: r.scope,
            scope_code: r.scope_code,
            title: r.title,
            text: r.text,
            enabled: r.enabled,
            version: r.version,
            source_refs: r.source_refs,
            source_label: r.source_label,
            expires_at: r.expires_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            index_status: r.index_status,
            index_error: r.index_error,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryRecordVo {
    pub saved: bool,
    pub searchable: bool,
    pub memory_code: String,
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
    pub enabled: bool,
    pub version: u32,
    pub source_refs: Vec<serde_json::Value>,
    pub source_label: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub index_status: String,
    pub index_error: Option<String>,
}
impl From<MemoryRecord> for UpdateMemoryRecordVo {
    fn from(r: MemoryRecord) -> Self {
        let searchable = r.enabled && r.index_status == "Ready";
        Self {
            saved: true,
            searchable,
            memory_code: r.memory_code,
            scope: r.scope,
            scope_code: r.scope_code,
            title: r.title,
            text: r.text,
            enabled: r.enabled,
            version: r.version,
            source_refs: r.source_refs,
            source_label: r.source_label,
            expires_at: r.expires_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            index_status: r.index_status,
            index_error: r.index_error,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMemoryRecordVo {
    pub memory_code: String,
    pub deleted: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWorkDomainResponse {
    pub domain_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<MemoryWorkDomain> for MemoryWorkDomainResponse {
    /// `status` stays a stable `Active`/`Disabled` protocol value; the client
    /// localizes the label from it (see the memory domains page adapter).
    fn from(domain: MemoryWorkDomain) -> Self {
        Self {
            domain_code: domain.domain_code,
            name: domain.name,
            description: domain.description,
            status: if domain.status == "ACTIVE" {
                "Active"
            } else {
                "Disabled"
            }
            .into(),
            created_at: crate::shared::time::timestamp_to_millis(&domain.created_at),
            updated_at: crate::shared::time::timestamp_to_millis(&domain.updated_at),
        }
    }
}
