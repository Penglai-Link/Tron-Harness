use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalError {
    WorkspaceRequired,
    UserRequired,
    AppNotFound,
    AppWorkspaceMismatch,
    AppArchived,
    LocalDirectoryUnavailable,
    WorkingDirectoryUnavailable,
    WorkspaceAccessDenied,
    WorkspaceArchived,
    WorkspaceNotFound,
    TicketInvalid,
    TicketExpired,
    StorageFailure,
}

impl fmt::Display for TerminalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceRequired => formatter.write_str("workspaceCode is required"),
            Self::UserRequired => formatter.write_str("authenticated user is required"),
            Self::AppNotFound => formatter.write_str("terminal app was not found"),
            Self::AppWorkspaceMismatch => {
                formatter.write_str("appCode does not belong to workspaceCode")
            }
            Self::AppArchived => formatter.write_str("terminal app must be active"),
            Self::LocalDirectoryUnavailable => {
                formatter.write_str("terminal app does not define a local directory")
            }
            Self::WorkingDirectoryUnavailable => {
                formatter.write_str("terminal working directory is unavailable")
            }
            Self::WorkspaceAccessDenied => {
                formatter.write_str("user cannot access the terminal workspace")
            }
            Self::WorkspaceArchived => formatter.write_str("terminal workspace must be active"),
            Self::WorkspaceNotFound => formatter.write_str("terminal workspace was not found"),
            Self::TicketInvalid => formatter.write_str("terminal socket ticket is invalid"),
            Self::TicketExpired => formatter.write_str("terminal socket ticket has expired"),
            Self::StorageFailure => formatter.write_str("terminal ticket storage failed"),
        }
    }
}

impl Error for TerminalError {}
