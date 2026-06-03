use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

pub const VARIABLE_CRYPTO_ALGORITHM: &str = "SM4_GCM";
pub const VARIABLE_MASK: &str = "********";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub id: SnowflakeId,
    pub workspace_code: String,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_sensitive: bool,
    pub value_nonce: Option<String>,
    pub value_tag: Option<String>,
    pub value_algorithm: Option<String>,
    pub value_key_version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformVariable {
    pub id: SnowflakeId,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_sensitive: bool,
    pub value_nonce: Option<String>,
    pub value_tag: Option<String>,
    pub value_algorithm: Option<String>,
    pub value_key_version: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum VariableSourceKind {
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveVariable {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub source_kind: VariableSourceKind,
    pub source_workspace_code: String,
    pub source_workspace_name: String,
    pub source_label: String,
    pub shadowed_workspace_code: Option<String>,
    pub shadowed_workspace_name: Option<String>,
    pub shadowed_label: Option<String>,
    pub is_sensitive: bool,
    pub value_masked: bool,
    pub has_value: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformVariableDisplay {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_sensitive: bool,
    pub value_masked: bool,
    pub has_value: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedVariableValue {
    pub ciphertext: String,
    pub nonce: String,
    pub tag: String,
    pub algorithm: String,
    pub key_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVariable {
    pub key: String,
    pub value: String,
    pub source: String,
    pub is_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableMutationResult {
    pub key: String,
    pub workspace_code: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableCatalogQuery {
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableError {
    NotFound,
    WorkspaceRequired,
    InvalidWorkspace,
    InvalidKey,
    ValueRequired,
    DuplicateKey,
    InheritedMutationDenied,
    EncryptionNotConfigured,
    EncryptFailed,
    DecryptFailed,
    SensitiveUpgradeValueRequired,
    SensitiveDowngradeValueRequired,
    SensitiveOverrideRequired,
    StorageFailure,
}

impl fmt::Display for VariableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("variable was not found"),
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => {
                formatter.write_str("variable workspace must reference an active workspace")
            }
            Self::InvalidKey => formatter.write_str("variable key is invalid"),
            Self::ValueRequired => formatter.write_str("variable value is required"),
            Self::DuplicateKey => {
                formatter.write_str("variable key already exists in the workspace")
            }
            Self::InheritedMutationDenied => {
                formatter.write_str("inherited variable must be overridden locally first")
            }
            Self::EncryptionNotConfigured => {
                formatter.write_str("variable encryption is not configured")
            }
            Self::EncryptFailed => formatter.write_str("sensitive value could not be encrypted"),
            Self::DecryptFailed => formatter.write_str("sensitive value could not be decrypted"),
            Self::SensitiveUpgradeValueRequired => {
                formatter.write_str("sensitive upgrade requires a replacement value")
            }
            Self::SensitiveDowngradeValueRequired => {
                formatter.write_str("sensitive downgrade requires a replacement value")
            }
            Self::SensitiveOverrideRequired => {
                formatter.write_str("sensitive inherited variables require sensitive overrides")
            }
            Self::StorageFailure => formatter.write_str("variable storage failure"),
        }
    }
}

impl Error for VariableError {}

pub fn validate_variable_key(value: &str) -> Result<(), VariableError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(VariableError::InvalidKey);
    }

    if trimmed
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
    {
        return Err(VariableError::InvalidKey);
    }

    Ok(())
}
