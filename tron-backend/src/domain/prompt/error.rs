use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    InvalidVariableName(String),
    UnclosedPlaceholder,
    MissingVariables(Vec<String>),
    UnknownVariables(Vec<String>),
}

impl Display for TemplateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVariableName(name) => {
                write!(formatter, "invalid prompt variable name: {name}")
            }
            Self::UnclosedPlaceholder => {
                formatter.write_str("prompt template contains an unclosed placeholder")
            }
            Self::MissingVariables(names) => {
                write!(formatter, "missing prompt variables: {}", names.join(", "))
            }
            Self::UnknownVariables(names) => {
                write!(formatter, "unknown prompt variables: {}", names.join(", "))
            }
        }
    }
}

impl Error for TemplateError {}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptError {
    WorkspaceRequired,
    InvalidWorkspace,
    NotFound,
    InvalidInput(String),
    DuplicateDisplayName,
    InheritedMutationDenied,
    AllowInheritanceUnsupported,
    RevertUnsupported,
    VersionConflict,
    StorageFailure,
    PreviewFailed(String),
}

impl Display for PromptError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::InvalidWorkspace => {
                formatter.write_str("prompt workspace must reference an active workspace")
            }
            Self::NotFound => formatter.write_str("prompt not found"),
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::DuplicateDisplayName => {
                formatter.write_str("displayName already exists in prompt scope")
            }
            Self::InheritedMutationDenied => formatter.write_str(
                "inherited prompt cannot be deleted; use workspace-revert for overrides",
            ),
            Self::AllowInheritanceUnsupported => formatter
                .write_str("allowInheritance is supported only for workspace-local prompts"),
            Self::RevertUnsupported => {
                formatter.write_str("prompt has no workspace override or status patch to revert")
            }
            Self::VersionConflict => formatter.write_str("prompt version conflict"),
            Self::StorageFailure => formatter.write_str("prompt storage operation failed"),
            Self::PreviewFailed(message) => write!(formatter, "prompt preview failed: {message}"),
        }
    }
}

impl Error for PromptError {}

impl From<TemplateError> for PromptError {
    fn from(error: TemplateError) -> Self {
        Self::InvalidInput(error.to_string())
    }
}
