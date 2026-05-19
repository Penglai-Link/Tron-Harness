use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub id: SnowflakeId,
    pub workspace_code: String,
    pub key: String,
    pub value: String,
    pub description: Option<String>,
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
    pub updated_at: String,
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
    DuplicateKey,
    InheritedMutationDenied,
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
            Self::DuplicateKey => formatter.write_str("variable key already exists in the workspace"),
            Self::InheritedMutationDenied => {
                formatter.write_str("inherited variable must be overridden locally first")
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
