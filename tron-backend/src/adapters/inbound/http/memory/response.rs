use serde::Serialize;

use crate::application::memory::MemoryWorkDomain;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetadataItemResponse {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecordResponse {
    pub domain_code: Option<String>,
    pub id: String,
    pub injectable: bool,
    pub memory_code: String,
    pub memory_status: String,
    pub memory_type: String,
    pub metadata: Vec<MemoryMetadataItemResponse>,
    pub review_status: String,
    pub scope: String,
    pub score: f64,
    pub semantic_key: String,
    pub source_code: String,
    pub source_label: String,
    pub tags: Vec<String>,
    pub target_code: String,
    pub target_id: String,
    pub target_label: String,
    pub text: String,
    pub title: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWorkDomainResponse {
    pub domain_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MemoryWorkDomain> for MemoryWorkDomainResponse {
    fn from(domain: MemoryWorkDomain) -> Self {
        Self {
            domain_code: domain.domain_code,
            name: domain.name,
            description: domain.description,
            status: domain.status,
            created_at: domain.created_at,
            updated_at: domain.updated_at,
        }
    }
}
