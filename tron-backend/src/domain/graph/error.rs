use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphScopeError {
    InvalidNamespace,
    InvalidWorkspaceCode,
    ArchivedWorkspace,
    EmptyWorkspaceLineage,
    WorkspaceResolutionFailed,
    ReservedSentinelValue,
    ReservedPropertyName,
}

impl fmt::Display for GraphScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNamespace => formatter.write_str("invalid graph namespace"),
            Self::InvalidWorkspaceCode => formatter.write_str("invalid graph workspace code"),
            Self::ArchivedWorkspace => formatter.write_str("graph workspace is archived"),
            Self::EmptyWorkspaceLineage => formatter.write_str("graph workspace lineage is empty"),
            Self::WorkspaceResolutionFailed => {
                formatter.write_str("graph workspace resolution failed")
            }
            Self::ReservedSentinelValue => formatter.write_str("reserved graph sentinel value"),
            Self::ReservedPropertyName => {
                formatter.write_str("reserved graph storage property name")
            }
        }
    }
}

impl Error for GraphScopeError {}
