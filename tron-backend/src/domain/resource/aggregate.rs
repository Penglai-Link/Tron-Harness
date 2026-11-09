use serde::{Deserialize, Serialize};

use super::ResourceKind;

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

#[derive(Debug, Clone, PartialEq)]
pub enum ManagedResource {
    File(ResourceFileRecord),
}

impl ManagedResource {
    pub fn workspace_code(&self) -> &str {
        match self {
            Self::File(record) => &record.workspace_code,
        }
    }

    pub fn resource_code(&self) -> &str {
        match self {
            Self::File(record) => &record.resource_code,
        }
    }

    pub fn allow_inheritance(&self) -> bool {
        match self {
            Self::File(record) => record.allow_inheritance,
        }
    }

    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::File(record) => record.kind,
        }
    }
}
