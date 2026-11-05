use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ResourceKind, ServerAuthenticationType};

pub const DEFAULT_BINARY_STORAGE_PROFILE: &str = "@default";

/// A binary attachment owned exclusively by one chat session.
///
/// This deliberately does not implement `ManagedResource`: session attachments
/// must never participate in resource projection, inheritance, or resource
/// management pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSessionAttachmentRecord {
    pub workspace_code: String,
    pub session_code: String,
    pub attachment_code: String,
    pub file_name: String,
    pub content_type: String,
    pub byte_size: u64,
    pub storage_locator: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFileRecord {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub file_name: String,
    pub file_extension: String,
    pub byte_size: u64,
    pub storage_locator: String,
    pub allow_inheritance: bool,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSourceConnectionRecord {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
    pub name: String,
    pub description: String,
    pub attributes: Value,
    pub allow_inheritance: bool,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRecord {
    pub workspace_code: String,
    pub resource_code: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: u16,
    pub allow_inheritance: bool,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerAccountServer {
    pub workspace_code: String,
    pub server_code: String,
}

// Credentials are deliberately absent from serialization and Debug output.
#[derive(Clone, PartialEq)]
pub struct ServerAccountRecord {
    pub workspace_code: String,
    pub resource_code: String,
    pub servers: Vec<ServerAccountServer>,
    pub name: String,
    pub description: String,
    pub username: String,
    pub authentication_type: ServerAuthenticationType,
    pub encrypted_credentials: String,
    pub allow_inheritance: bool,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl std::fmt::Debug for ServerAccountRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerAccountRecord")
            .field("workspace_code", &self.workspace_code)
            .field("resource_code", &self.resource_code)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManagedResource {
    File(ResourceFileRecord),
    DataSourceConnection(DataSourceConnectionRecord),
    Server(ServerRecord),
    ServerAccount(ServerAccountRecord),
}

impl ManagedResource {
    pub fn workspace_code(&self) -> &str {
        match self {
            Self::File(record) => &record.workspace_code,
            Self::DataSourceConnection(record) => &record.workspace_code,
            Self::Server(record) => &record.workspace_code,
            Self::ServerAccount(record) => &record.workspace_code,
        }
    }

    pub fn resource_code(&self) -> &str {
        match self {
            Self::File(record) => &record.resource_code,
            Self::DataSourceConnection(record) => &record.resource_code,
            Self::Server(record) => &record.resource_code,
            Self::ServerAccount(record) => &record.resource_code,
        }
    }

    pub fn allow_inheritance(&self) -> bool {
        match self {
            Self::File(record) => record.allow_inheritance,
            Self::DataSourceConnection(record) => record.allow_inheritance,
            Self::Server(record) => record.allow_inheritance,
            Self::ServerAccount(record) => record.allow_inheritance,
        }
    }

    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::File(record) => record.kind,
            Self::DataSourceConnection(record) => record.kind,
            Self::Server(_) => ResourceKind::ServerConnection,
            Self::ServerAccount(_) => ResourceKind::ServerAccount,
        }
    }
}
