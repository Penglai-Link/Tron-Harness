use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use super::PromptVariableType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    InvalidVariableName(String),
    UnclosedPlaceholder,
    MissingVariables(Vec<String>),
    UnknownVariables(Vec<String>),
    VariableSchemaMismatch,
    VariableTypeMismatch {
        variable_name: String,
        expected_type: PromptVariableType,
    },
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
            Self::VariableSchemaMismatch => {
                formatter.write_str("prompt variable schema does not match template placeholders")
            }
            Self::VariableTypeMismatch {
                variable_name,
                expected_type,
            } => write!(
                formatter,
                "prompt variable {variable_name} must be {}",
                expected_type.code()
            ),
        }
    }
}

impl Error for TemplateError {}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptError {
    NotFound,
    InvalidInput(String),
    DuplicateDisplayName,
    BuiltinMutationDenied,
    RuntimeRequired,
    VersionConflict,
    StorageFailure,
}

impl Display for PromptError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("prompt not found"),
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::DuplicateDisplayName => formatter.write_str("displayName already exists"),
            Self::BuiltinMutationDenied => {
                formatter.write_str("built-in prompts cannot be deleted")
            }
            Self::RuntimeRequired => {
                formatter.write_str("runtime-required prompts cannot be disabled")
            }
            Self::VersionConflict => formatter.write_str("prompt version conflict"),
            Self::StorageFailure => formatter.write_str("prompt storage operation failed"),
        }
    }
}

impl Error for PromptError {}

impl From<TemplateError> for PromptError {
    fn from(error: TemplateError) -> Self {
        Self::InvalidInput(error.to_string())
    }
}
