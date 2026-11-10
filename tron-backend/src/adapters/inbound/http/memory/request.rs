use crate::application::memory::{
    CreateMemoryWorkDomainCommand, MemoryScope, MemoryWorkDomainQuery,
    UpdateMemoryWorkDomainCommand,
};
use serde::Deserialize;
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryMemoryRecordsDto {
    pub scope: MemoryScope,
    pub scope_code: String,
    #[serde(default)]
    pub keyword: String,
    pub enabled: Option<bool>,
    pub current: Option<usize>,
    pub size: Option<usize>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateMemoryRecordDto {
    pub scope: MemoryScope,
    pub scope_code: String,
    pub title: String,
    pub text: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateMemoryRecordDto {
    pub version: u32,
    pub title: Option<String>,
    pub text: Option<String>,
    pub enabled: Option<bool>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteMemoryRecordDto {
    pub version: u32,
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
    pub status: Option<MemoryDomainStatusDto>,
}

impl From<UpdateMemoryWorkDomainRequest> for UpdateMemoryWorkDomainCommand {
    fn from(request: UpdateMemoryWorkDomainRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            status: request.status.map(|s| match s {
                MemoryDomainStatusDto::Active => "ACTIVE".into(),
                MemoryDomainStatusDto::Disabled => "DISABLED".into(),
            }),
        }
    }
}

pub type MemoryWorkDomainHttpQuery = MemoryWorkDomainQuery;

#[derive(Debug, Deserialize)]
pub enum MemoryDomainStatusDto {
    Active,
    Disabled,
}
