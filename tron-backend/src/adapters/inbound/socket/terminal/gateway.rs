use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::application::terminal::TerminalService;

use super::{
    protocol::{
        MAX_INPUT_BYTES, TerminalInputPayload, TerminalResizePayload, TerminalSessionEvent,
        TerminalSignalPayload, TerminalSocketGatewayError,
    },
    session::{PtySession, SessionDescriptor},
};

#[derive(Clone)]
pub struct TerminalSocketGateway {
    terminal: TerminalService,
    pty: eliteforge_agent_capability_tools::PtyService,
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl TerminalSocketGateway {
    pub fn new(terminal: TerminalService) -> Self {
        Self {
            terminal,
            pty: Default::default(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_pty(mut self, pty: eliteforge_agent_capability_tools::PtyService) -> Self {
        self.pty = pty;
        self
    }

    pub(super) async fn open_session(
        &self,
        socket_id: String,
        ticket: String,
        output: UnboundedSender<TerminalSessionEvent>,
    ) -> Result<SessionDescriptor, TerminalSocketGatewayError> {
        let authorization = self.terminal.consume_socket_ticket(&ticket)?;
        let pty = self.pty.clone();
        let owner = format!("terminal-socket:{socket_id}");
        let (session, descriptor) = tokio::task::spawn_blocking(move || {
            PtySession::spawn(pty, owner, authorization, output)
        })
        .await
        .map_err(|_| TerminalSocketGatewayError::PtyFailed)??;

        self.sessions
            .lock()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .insert(socket_id, session);
        Ok(descriptor)
    }

    pub(super) fn write_input(
        &self,
        socket_id: &str,
        payload: TerminalInputPayload,
    ) -> Result<(), TerminalSocketGatewayError> {
        // Allow empty data when clearBuffer is true
        if payload.data.is_empty() && !payload.clear_buffer {
            return Ok(());
        }
        if payload.data.len() > MAX_INPUT_BYTES
            || payload
                .input_line
                .as_ref()
                .is_some_and(|line| line.len() > MAX_INPUT_BYTES)
        {
            return Err(TerminalSocketGatewayError::InputTooLarge);
        }
        self.session(socket_id)?.write_input(
            &payload.data,
            payload.clear_buffer,
            payload.input_line.as_deref(),
        )
    }

    pub(super) fn signal(
        &self,
        socket_id: &str,
        payload: TerminalSignalPayload,
    ) -> Result<(), TerminalSocketGatewayError> {
        self.session(socket_id)?.signal(payload.signal)
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
