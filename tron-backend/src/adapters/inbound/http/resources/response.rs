use serde::Serialize;

use crate::{
    application::resource::{
        ResourceBinaryResult, ResourceCommitResult, ResourceDeleteResult, ResourcePage,
        ResourceRecord, ServerCreateResult,
    },
    domain::resource::{ResourceKind, ResourceSourceKind},
    shared::time::timestamp_to_millis,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceHttpApiResponse<T> {
    pub code: String,
    pub message: String,
    pub data: T,
}

impl<T> ResourceHttpApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: "0".to_string(),
            message: "ok".to_string(),
            data,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ResourceHttpEmptyData {}

impl ResourceHttpApiResponse<ResourceHttpEmptyData> {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: ResourceHttpEmptyData {},
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePageResponse {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ResourceResponse>,
}

impl From<ResourcePage> for ResourcePageResponse {
    fn from(value: ResourcePage) -> Self {
        Self {
            current: value.current,
            size: value.size,
            total: value.total,
            records: value.records.into_iter().map(Into::into).collect(),
        }
    }
}

/// Response DTO for the connection-specific page endpoint.
///
/// Keep this separate from `ResourcePageResponse`: this endpoint is a
/// connection-only contract and must not silently gain file-resource fields
/// when the generic resource API evolves.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPageResponse {
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ConnectionResponse>,
}

impl From<ResourcePage> for ConnectionPageResponse {
    fn from(value: ResourcePage) -> Self {
        Self {
            current: value.current,
            size: value.size,
            total: value.total,
            records: value.records.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFileSummaryResponse {
    pub file_name: String,
    pub file_extension: String,
    pub byte_size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceResponse {
    pub workspace_code: String,
    pub source_kind: ResourceSourceKind,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub allow_inheritance: bool,
    pub file: Option<ResourceFileSummaryResponse>,
    pub attributes: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ResourceRecord> for ResourceResponse {
    fn from(value: ResourceRecord) -> Self {
        Self {
            workspace_code: value.workspace_code,
            source_kind: value.source_kind,
            resource_code: value.resource_code,
            kind: value.kind,
            name: value.name,
            description: value.description,
            allow_inheritance: value.allow_inheritance,
            file: value.file.map(|file| ResourceFileSummaryResponse {
                file_name: file.file_name,
                file_extension: file.file_extension,
                byte_size: file.byte_size,
            }),
            attributes: value.attributes,
            created_at: timestamp_to_millis(&value.created_at),
            updated_at: timestamp_to_millis(&value.updated_at),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionResponse {
    pub workspace_code: String,
    pub source_kind: ResourceSourceKind,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub allow_inheritance: bool,
    pub file: Option<ResourceFileSummaryResponse>,
    pub attributes: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<ResourceRecord> for ConnectionResponse {
    fn from(value: ResourceRecord) -> Self {
        Self {
            workspace_code: value.workspace_code,
            source_kind: value.source_kind,
            resource_code: value.resource_code,
            kind: value.kind,
            name: value.name,
            description: value.description,
            allow_inheritance: value.allow_inheritance,
            file: value.file.map(|file| ResourceFileSummaryResponse {
                file_name: file.file_name,
                file_extension: file.file_extension,
                byte_size: file.byte_size,
            }),
            attributes: value.attributes,
            created_at: timestamp_to_millis(&value.created_at),
            updated_at: timestamp_to_millis(&value.updated_at),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCommitResponse {
    pub resource_code: String,
    pub kind: ResourceKind,
}

impl From<ResourceCommitResult> for ResourceCommitResponse {
    fn from(value: ResourceCommitResult) -> Self {
        Self {
            resource_code: value.resource_code,
            kind: value.kind,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDeleteResponse {
    pub resource_code: String,
    pub deleted: bool,
}

impl From<ResourceDeleteResult> for ResourceDeleteResponse {
    fn from(value: ResourceDeleteResult) -> Self {
        Self {
            resource_code: value.resource_code,
            deleted: value.deleted,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDeleteResponse {
    pub resource_code: String,
    pub deleted: bool,
}

impl From<ResourceDeleteResult> for ConnectionDeleteResponse {
    fn from(value: ResourceDeleteResult) -> Self {
        Self {
            resource_code: value.resource_code,
            deleted: value.deleted,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinaryActionResponse {
    pub action: String,
    pub resource_code: String,
    pub file_name: Option<String>,
    pub byte_size: Option<u64>,
    pub target_path: Option<String>,
}

impl From<ResourceBinaryResult> for ResourceBinaryActionResponse {
    fn from(value: ResourceBinaryResult) -> Self {
        Self {
            action: value.action,
            resource_code: value.resource_code,
            file_name: value.file_name,
            byte_size: value.byte_size,
            target_path: value.target_path,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCommitOutcomeResponse {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
}

impl From<crate::application::resource::ResourceCommitOutcome> for ResourceCommitOutcomeResponse {
    fn from(value: crate::application::resource::ResourceCommitOutcome) -> Self {
        Self {
            workspace_code: value.workspace_code,
            resource_code: value.resource_code,
            kind: value.kind,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreateResponse {
    pub server: ResourceCommitOutcomeResponse,
    pub initial_account: ResourceCommitOutcomeResponse,
    pub connectivity_verified: bool,
}

impl From<ServerCreateResult> for ServerCreateResponse {
    fn from(value: ServerCreateResult) -> Self {
        Self {
            server: value.server.into(),
            initial_account: value.initial_account.into(),
            connectivity_verified: value.connectivity_verified,
        }
    }
}
