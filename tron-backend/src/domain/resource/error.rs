use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    NotFound,
    Forbidden,
    WorkspaceRequired,
    InvalidWorkspace,
    InvalidCode,
    InvalidInput,
    InvalidField { field: String, reason: String },
    InvalidVariableReference,
    Conflict,
    StorageFailure,
    MissingBinaryObject,
    PathDenied,
    StorageProfileNotFound,
    StorageLocatorInvalid,
    ConnectionVerificationFailed(String),
    ServerHasAccounts,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NotFound => "resource record was not found",
            Self::Forbidden => "resource operation is not authorized",
            Self::WorkspaceRequired => "workspaceCode is required",
            Self::InvalidWorkspace => "workspace is not active",
            Self::InvalidCode => "resource business code is invalid",
            Self::InvalidInput => "resource input is invalid",
            Self::InvalidField { field, reason } => {
                return write!(formatter, "resource field '{field}' is invalid: {reason}");
            }
            Self::InvalidVariableReference => "resource variable reference is invalid",
            Self::Conflict => "resource state conflicts with the requested operation",
            Self::StorageFailure => "resource storage operation failed",
            Self::MissingBinaryObject => "resource binary object is unavailable",
            Self::PathDenied => "resource binary path is not allowed",
            Self::StorageProfileNotFound => "resource storage profile was not found",
            Self::StorageLocatorInvalid => "resource storage locator is invalid",
            Self::ConnectionVerificationFailed(message) => message,
            Self::ServerHasAccounts => "server still has accounts",
        };
        formatter.write_str(message)
    }
}

impl Error for ResourceError {}
