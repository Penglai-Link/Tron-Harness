use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    NotFound,
    Forbidden,
    WorkspaceRequired,
    InvalidWorkspace,
    InvalidScope,
    InvalidCode,
    InvalidVersion,
    InvalidInput,
    InvalidSchema,
    InvalidBinarySchema,
    InvalidVariableReference,
    ValidationFailed,
    Conflict,
    DraftRevisionConflict,
    ActiveVersionConflict,
    LifecycleDenied,
    DefinitionDisabled,
    DefinitionInUse,
    BuiltinDefinition,
    StorageFailure,
    MissingBinaryObject,
    PathDenied,
    StorageProfileNotFound,
    StorageLocatorInvalid,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NotFound => "resource record was not found",
            Self::Forbidden => "resource operation is not authorized",
            Self::WorkspaceRequired => "workspaceCode is required",
            Self::InvalidWorkspace => "workspace is not active",
            Self::InvalidScope => "resource scope is invalid",
            Self::InvalidCode => "resource business code is invalid",
            Self::InvalidVersion => "resource version is invalid",
            Self::InvalidInput => "resource input is invalid",
            Self::InvalidSchema => "resource schema is invalid",
            Self::InvalidBinarySchema => "resource binary schema is invalid",
            Self::InvalidVariableReference => "resource variable reference is invalid",
            Self::ValidationFailed => "resource validation failed",
            Self::Conflict => "resource state conflicts with the requested operation",
            Self::DraftRevisionConflict => "resource draft revision no longer matches",
            Self::ActiveVersionConflict => "resource active version no longer matches",
            Self::LifecycleDenied => "resource lifecycle transition is not allowed",
            Self::DefinitionDisabled => "resource definition is disabled",
            Self::DefinitionInUse => "resource definition is still referenced by resources",
            Self::BuiltinDefinition => "built-in resource definitions cannot be deleted",
            Self::StorageFailure => "resource storage operation failed",
            Self::MissingBinaryObject => "resource binary object is unavailable",
            Self::PathDenied => "resource binary path is not allowed",
            Self::StorageProfileNotFound => "resource storage profile was not found",
            Self::StorageLocatorInvalid => "resource storage locator is invalid",
        };
        formatter.write_str(message)
    }
}

impl Error for ResourceError {}
