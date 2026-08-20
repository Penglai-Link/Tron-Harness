use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use chrono::Utc;

use crate::{
    application::app::AppService,
    application::workspace::WorkspaceService,
    domain::{
        app::{AppError, AppStatus},
        workspace::{WorkspaceError, WorkspaceStatus},
    },
    shared::random::random_base64url,
};

use super::{
    IssueTerminalSocketTicketCommand, TerminalError, TerminalSessionAuthorization,
    TerminalSocketTicket,
};

pub const TERMINAL_SOCKET_NAMESPACE: &str = "/terminals";
pub const TERMINAL_SOCKET_PATH: &str = "/socket.io";

const TERMINAL_TICKET_TTL_MILLIS: i64 = 60_000;

#[derive(Clone)]
pub struct TerminalService {
    apps: AppService,
    terminal_working_directory: String,
    tickets: Arc<Mutex<HashMap<String, StoredTerminalSocketTicket>>>,
    workspaces: WorkspaceService,
}

#[derive(Debug, Clone)]
struct StoredTerminalSocketTicket {
    authorization: TerminalSessionAuthorization,
    expires_at: i64,
}

impl TerminalService {
    pub fn new(
        apps: AppService,
        workspaces: WorkspaceService,
        terminal_working_directory: String,
    ) -> Self {
        Self {
            apps,
            terminal_working_directory,
            tickets: Arc::new(Mutex::new(HashMap::new())),
            workspaces,
        }
    }

    pub fn issue_socket_ticket(
        &self,
        command: IssueTerminalSocketTicketCommand,
    ) -> Result<TerminalSocketTicket, TerminalError> {
        let workspace_code = required(command.workspace_code, TerminalError::WorkspaceRequired)?;
        let user_code = required(command.user_code, TerminalError::UserRequired)?;
        let (app_code, current_directory) =
            self.resolve_terminal_directory(&workspace_code, command.app_code)?;
        self.authorize_workspace_user(&workspace_code, &user_code)?;
        let expires_at = Utc::now().timestamp_millis() + TERMINAL_TICKET_TTL_MILLIS;
        let ticket = random_base64url(32);
        let authorization = TerminalSessionAuthorization {
            app_code,
            current_directory,
            user_code,
            workspace_code,
        };

        let mut tickets = self
            .tickets
            .lock()
            .map_err(|_| TerminalError::StorageFailure)?;
        tickets.retain(|_, record| record.expires_at > Utc::now().timestamp_millis());
        tickets.insert(
            ticket.clone(),
            StoredTerminalSocketTicket {
                authorization,
                expires_at,
            },
        );

        Ok(TerminalSocketTicket {
            ticket,
            expires_at,
            socket_path: TERMINAL_SOCKET_PATH.into(),
            socket_namespace: TERMINAL_SOCKET_NAMESPACE.into(),
        })
    }

    fn resolve_terminal_directory(
        &self,
        workspace_code: &str,
        app_code: Option<String>,
    ) -> Result<(Option<String>, String), TerminalError> {
        let Some(app_code) = optional(app_code) else {
            let directory = required(
                self.terminal_working_directory.clone(),
                TerminalError::WorkingDirectoryUnavailable,
            )?;
            if !Path::new(&directory).is_dir() {
                return Err(TerminalError::WorkingDirectoryUnavailable);
            }
            return Ok((None, directory));
        };
        let app = self.apps.get_app(&app_code).map_err(map_app_error)?;

        if app.workspace_code != workspace_code {
            return Err(TerminalError::AppWorkspaceMismatch);
        }
        if app.status != AppStatus::Active {
            return Err(TerminalError::AppArchived);
        }
        let current_directory = required(
            app.local_directory,
            TerminalError::LocalDirectoryUnavailable,
        )?;
        Ok((Some(app_code), current_directory))
    }

    pub fn consume_socket_ticket(
        &self,
        ticket: &str,
    ) -> Result<TerminalSessionAuthorization, TerminalError> {
        let ticket = required(ticket.to_owned(), TerminalError::TicketInvalid)?;
        let record = self
            .tickets
            .lock()
            .map_err(|_| TerminalError::StorageFailure)?
            .remove(&ticket)
            .ok_or(TerminalError::TicketInvalid)?;

        if record.expires_at <= Utc::now().timestamp_millis() {
            return Err(TerminalError::TicketExpired);
        }

        Ok(record.authorization)
    }

    pub fn authorize_workspace_user(
        &self,
        workspace_code: &str,
        user_code: &str,
    ) -> Result<(), TerminalError> {
        let workspace_code = required(workspace_code.to_owned(), TerminalError::WorkspaceRequired)?;
        let user_code = required(user_code.to_owned(), TerminalError::UserRequired)?;
        let workspace = self
            .workspaces
            .get_workspace(&workspace_code)
            .map_err(map_workspace_error)?;
        if workspace.status != WorkspaceStatus::Active {
            return Err(TerminalError::WorkspaceArchived);
        }
        let has_access = self
            .workspaces
            .user_has_effective_access(&workspace_code, &user_code)
            .map_err(map_workspace_error)?;
        if !has_access {
            return Err(TerminalError::WorkspaceAccessDenied);
        }
        Ok(())
    }
}

fn required(value: String, error: TerminalError) -> Result<String, TerminalError> {
    let value = value.trim();
    if value.is_empty() {
        Err(error)
    } else {
        Ok(value.to_owned())
    }
}

fn optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn map_app_error(error: AppError) -> TerminalError {
    match error {
        AppError::NotFound => TerminalError::AppNotFound,
        AppError::StorageFailure => TerminalError::StorageFailure,
        AppError::InvalidAppCode
        | AppError::WorkspaceRequired
        | AppError::InvalidWorkspace
        | AppError::NameRequired
        | AppError::LocalDirectoryRequired
        | AppError::DuplicateName
        | AppError::ArchivedMutationDenied => TerminalError::AppNotFound,
    }
}

fn map_workspace_error(error: WorkspaceError) -> TerminalError {
    match error {
        WorkspaceError::NotFound | WorkspaceError::InvalidWorkspaceCode => {
            TerminalError::WorkspaceNotFound
        }
        WorkspaceError::InvalidAccessUser => TerminalError::WorkspaceAccessDenied,
        WorkspaceError::DuplicateWorkspaceCode
        | WorkspaceError::WorkspaceNameRequired
        | WorkspaceError::InvalidHierarchy
        | WorkspaceError::ArchivedMutationDenied
        | WorkspaceError::InvalidAccessRole
        | WorkspaceError::DuplicateAccessGrant
        | WorkspaceError::StorageFailure => TerminalError::StorageFailure,
    }
}
