//! Socket presentation and shell hooks; all PTY execution belongs to the framework.
use super::{
    protocol::{TerminalSessionEvent, TerminalSignal, TerminalSocketGatewayError, Utf8Stream},
    shell_integration::{ShellIntegration, subshell_bootstrap},
};
use crate::application::terminal::TerminalSessionAuthorization;
use eliteforge_agent_capability_tools::{PtyEvent, PtyService};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(super) struct SessionDescriptor {
    pub(super) current_directory: String,
    pub(super) shell: String,
    pub(super) supports_blocks: bool,
    pub(super) subshell_bootstrap: String,
}
#[derive(Clone)]
pub(super) struct PtySession(Arc<SessionBinding>);
struct SessionBinding {
    service: PtyService,
    owner: String,
    id: String,
    // Product shell hooks must stay on disk while the framework runs this shell.
    _integration: ShellIntegration,
    suppress_output_until: Arc<Mutex<Option<Instant>>>,
}
impl Drop for SessionBinding {
    fn drop(&mut self) {
        let _ = self.service.release(&self.owner, &self.id);
    }
}
impl PtySession {
    pub(super) fn spawn(
        service: PtyService,
        owner: String,
        authorization: TerminalSessionAuthorization,
        output: UnboundedSender<TerminalSessionEvent>,
    ) -> Result<(Self, SessionDescriptor), TerminalSocketGatewayError> {
        let current_directory = std::fs::canonicalize(&authorization.current_directory)
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .to_string_lossy()
            .into_owned();
        let integration = ShellIntegration::prepare();
        let kind = integration.kind();
        let mut command = integration.command();
        command.cwd(&current_directory);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TRON_TERMINAL", "1");
        let (snapshot, events) = service
            .start_stream(owner.clone(), command, 80, 24)
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        let suppress = Arc::new(Mutex::new(None));
        tokio::spawn(forward_terminal_events(events, output, suppress.clone()));
        let session = Self(Arc::new(SessionBinding {
            service,
            owner,
            id: snapshot.terminal_id,
            _integration: integration,
            suppress_output_until: suppress,
        }));
        Ok((
            session,
            SessionDescriptor {
                current_directory,
                shell: kind.as_str().into(),
                supports_blocks: kind.reports_blocks(),
                subshell_bootstrap: subshell_bootstrap(),
            },
        ))
    }
    pub(super) fn write_input(
        &self,
        data: &str,
        clear_buffer: bool,
        _input_line: Option<&str>,
    ) -> Result<(), TerminalSocketGatewayError> {
        if clear_buffer {
            if let Ok(mut suppress) = self.0.suppress_output_until.lock() {
                *suppress = Some(Instant::now() + Duration::from_millis(250));
            }
            self.write("\x03")?;
        }
        self.write(data)
    }
    fn write(&self, data: &str) -> Result<(), TerminalSocketGatewayError> {
        self.0
            .service
            .write(&self.0.owner, &self.0.id, data)
            .map_err(|_| TerminalSocketGatewayError::WriteFailed)
    }
    pub(super) fn signal(&self, signal: TerminalSignal) -> Result<(), TerminalSocketGatewayError> {
        match signal {
            TerminalSignal::Sigint => self.write("\x03"),
            TerminalSignal::Sigkill => self
                .0
                .service
                .stop(&self.0.owner, &self.0.id)
                .map_err(|_| TerminalSocketGatewayError::WriteFailed),
        }
    }
    pub(super) fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalSocketGatewayError> {
        self.0
            .service
            .resize(&self.0.owner, &self.0.id, cols, rows)
            .map_err(|_| TerminalSocketGatewayError::ResizeFailed)
    }
    pub(super) fn close(&self) {
        let _ = self.0.service.release(&self.0.owner, &self.0.id);
    }
}
/// Translate framework byte events to the existing socket protocol without
/// interpreting shell output, OSC markers or full-screen program escape codes.
async fn forward_terminal_events(
    mut events: UnboundedReceiver<PtyEvent>,
    output: UnboundedSender<TerminalSessionEvent>,
    suppress: Arc<Mutex<Option<Instant>>>,
) {
    let mut stream = Utf8Stream::default();
    while let Some(event) = events.recv().await {
        match event {
            PtyEvent::Output(bytes) => {
                let text = stream.push(&bytes);
                let hidden = suppress
                    .lock()
                    .ok()
                    .and_then(|value| *value)
                    .is_some_and(|until| Instant::now() < until);
                if !hidden
                    && !text.is_empty()
                    && output.send(TerminalSessionEvent::Output(text)).is_err()
                {
                    return;
                }
            }
            PtyEvent::Exit(code) => {
                let trailing = stream.flush();
                if !trailing.is_empty() {
                    let _ = output.send(TerminalSessionEvent::Output(trailing));
                }
                let _ = output.send(TerminalSessionEvent::Exit(
                    code.and_then(|code| i32::try_from(code).ok()),
                ));
                return;
            }
        }
    }
}
