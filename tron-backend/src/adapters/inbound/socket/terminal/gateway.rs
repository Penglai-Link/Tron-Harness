use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::{application::terminal::TerminalService, shared::random::random_base64url};

use super::{
    protocol::{
        CommandCancelPayload, CommandExecutePayload, CommandInputPayload, MAX_COMMAND_BYTES,
        MAX_INPUT_BYTES, TerminalResizePayload, TerminalSocketGatewayError, TerminalSocketMessage,
        required_message_id,
    },
    session::PtySession,
};

#[derive(Clone)]
pub struct TerminalSocketGateway {
    terminal: TerminalService,
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl TerminalSocketGateway {
    pub fn new(terminal: TerminalService) -> Self {
        Self {
            terminal,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(super) async fn open_session(
        &self,
        socket_id: String,
        ticket: String,
        output: UnboundedSender<TerminalSocketMessage>,
    ) -> Result<String, TerminalSocketGatewayError> {
        let authorization = self.terminal.consume_socket_ticket(&ticket)?;
        let (session, current_directory) =
            tokio::task::spawn_blocking(move || PtySession::spawn(authorization, output))
                .await
                .map_err(|_| TerminalSocketGatewayError::PtyFailed)??;

        self.sessions
            .lock()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .insert(socket_id, session);
        Ok(current_directory)
    }

    pub(super) fn execute(
        &self,
        socket_id: &str,
        payload: CommandExecutePayload,
    ) -> Result<(), TerminalSocketGatewayError> {
        let command = payload.command.trim();
        if command.is_empty() {
            return Err(TerminalSocketGatewayError::CommandRequired);
        }
        if command.len() > MAX_COMMAND_BYTES {
            return Err(TerminalSocketGatewayError::InputTooLarge);
        }
        let message_id = required_message_id(payload.message_id)?;
        self.session(socket_id)?.execute(
            command,
            message_id,
            payload.interactive.unwrap_or(false),
            &random_base64url(16),
        )
    }

    pub(super) fn cancel(
        &self,
        socket_id: &str,
        payload: CommandCancelPayload,
    ) -> Result<(), TerminalSocketGatewayError> {
        let message_id = required_message_id(payload.message_id)?;
        self.session(socket_id)?.cancel(&message_id)
    }

    pub(super) fn write_input(
        &self,
        socket_id: &str,
        payload: CommandInputPayload,
    ) -> Result<(), TerminalSocketGatewayError> {
        if payload.data.is_empty() {
            return Ok(());
        }
        if payload.data.len() > MAX_INPUT_BYTES {
            return Err(TerminalSocketGatewayError::InputTooLarge);
        }
        let message_id = required_message_id(payload.message_id)?;
        self.session(socket_id)?
            .write_input(&message_id, payload.data)
    }

    pub(super) fn resize(
        &self,
        socket_id: &str,
        payload: TerminalResizePayload,
    ) -> Result<(), TerminalSocketGatewayError> {
        if payload.cols == 0 || payload.rows == 0 {
            return Ok(());
        }
        self.session(socket_id)?.resize(payload.cols, payload.rows)
    }

    pub(super) fn close_session(&self, socket_id: &str) {
        let session = self
            .sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(socket_id));
        if let Some(session) = session {
            session.close();
        }
    }

    fn session(&self, socket_id: &str) -> Result<PtySession, TerminalSocketGatewayError> {
        self.sessions
            .lock()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .get(socket_id)
            .cloned()
            .ok_or(TerminalSocketGatewayError::SessionNotFound)
    }
}
