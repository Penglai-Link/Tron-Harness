use serde::Deserialize;

use crate::application::memory::{
    CreateMemoryWorkDomainCommand, MemoryWorkDomainQuery, UpdateMemoryWorkDomainCommand,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecordListQuery {
    pub scope: Option<String>,
    pub scope_code: Option<String>,
    pub query: Option<String>,
    pub memory_status: Option<String>,
    pub review_status: Option<String>,
    pub injectable: Option<String>,
    pub memory_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecordMutationRequest {
    pub scope: Option<String>,
    pub scope_code: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub memory_type: Option<String>,
    pub memory_status: Option<String>,
    pub review_status: Option<String>,
    pub injectable: Option<bool>,
    pub semantic_key: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: Vec<MemoryMetadataItemRequest>,
    pub source_label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetadataItemRequest {
    pub label: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDeleteRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryWorkDomainRequest {
    pub domain_code: String,
    pub name: String,
    pub description: Option<String>,
}

impl From<CreateMemoryWorkDomainRequest> for CreateMemoryWorkDomainCommand {
    fn from(request: CreateMemoryWorkDomainRequest) -> Self {
        Self {
            domain_code: request.domain_code,
            name: request.name,
            description: request.description,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryWorkDomainRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

impl From<UpdateMemoryWorkDomainRequest> for UpdateMemoryWorkDomainCommand {
    fn from(request: UpdateMemoryWorkDomainRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            status: request.status,
        }
    }
}

pub type MemoryWorkDomainHttpQuery = MemoryWorkDomainQuery;
