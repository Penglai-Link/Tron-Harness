use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use portable_pty::{Child, MasterPty, PtySize, native_pty_system};
use tokio::sync::mpsc::UnboundedSender;

use crate::application::terminal::TerminalSessionAuthorization;

use super::{
    protocol::{TerminalSessionEvent, TerminalSignal, TerminalSocketGatewayError, Utf8Stream},
    shell_integration::{ShellIntegration, subshell_bootstrap},
};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const READ_BUFFER_BYTES: usize = 8 * 1024;

/// Everything the client needs to configure itself once the PTY is up.
pub(super) struct SessionDescriptor {
    pub(super) current_directory: String,
    pub(super) shell: String,
    pub(super) supports_blocks: bool,
    pub(super) subshell_bootstrap: String,
}

#[derive(Clone)]
pub(super) struct PtySession {
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Holds the generated rc files on disk for as long as the shell runs.
    _integration: Arc<ShellIntegration>,
    /// When set, suppress output until this time (used for clearBuffer)
    suppress_output_until: Arc<Mutex<Option<Instant>>>,
}

impl PtySession {
    pub(super) fn spawn(
        authorization: TerminalSessionAuthorization,
        output: UnboundedSender<TerminalSessionEvent>,
    ) -> Result<(Self, SessionDescriptor), TerminalSocketGatewayError> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;

        let current_directory = std::fs::canonicalize(&authorization.current_directory)
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .to_string_lossy()
            .into_owned();

        let integration = ShellIntegration::prepare();
        let mut command = integration.command();
        command.cwd(&current_directory);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TRON_TERMINAL", "1");

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;

        let session = Self {
            child: Arc::new(Mutex::new(child)),
            master: Arc::new(Mutex::new(pair.master)),
            writer: Arc::new(Mutex::new(writer)),
            _integration: Arc::new(integration),
            suppress_output_until: Arc::new(Mutex::new(None)),
        };

        let reader_session = session.clone();
        std::thread::Builder::new()
            .name("tron-terminal-pty".to_owned())
            .spawn(move || forward_terminal_output(reader, reader_session, output))
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;

        let kind = session._integration.kind();
        Ok((
            session,
            SessionDescriptor {
                current_directory,
                shell: kind.as_str().to_owned(),
                supports_blocks: kind.reports_blocks(),
                subshell_bootstrap: subshell_bootstrap(),
            },
        ))
    }

    /// Forwards keystrokes straight to the PTY. The shell owns echo, line
    /// editing and prompt rendering, so nothing here interprets the bytes.
    ///
    /// `clear_buffer` cancels the pending readline input before
    /// writing new data and hides the resulting prompt redraw from the client.
    pub(super) fn write_input(
        &self,
        data: &str,
        clear_buffer: bool,
        _input_line: Option<&str>,
    ) -> Result<(), TerminalSocketGatewayError> {
        if clear_buffer {
            if let Ok(mut suppress) = self.suppress_output_until.lock() {
                *suppress = Some(Instant::now() + Duration::from_millis(250));
            }

            let mut input = String::with_capacity(data.len() + 1);
            input.push('\x03');
            input.push_str(data);
            return write_to_terminal(&self.writer, input.as_bytes());
        }

        write_to_terminal(&self.writer, data.as_bytes())
    }

    pub(super) fn signal(&self, signal: TerminalSignal) -> Result<(), TerminalSocketGatewayError> {
        match signal {
            // Let the line discipline turn this into SIGINT for the whole
            // foreground process group, which is what Ctrl-C does natively.
            TerminalSignal::Sigint => write_to_terminal(&self.writer, b"\x03"),
            TerminalSignal::Sigkill => {
                self.close();
                Ok(())
            }
        }
    }

    pub(super) fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalSocketGatewayError> {
        self.master
            .lock()
            .map_err(|_| TerminalSocketGatewayError::ResizeFailed)?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| TerminalSocketGatewayError::ResizeFailed)
    }

    pub(super) fn close(&self) {
        let _ = self
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.kill().ok());
    }

    fn exit_code(&self) -> Option<i32> {
        self.child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok().flatten())
            .map(|status| status.exit_code() as i32)
    }
}

fn write_to_terminal(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    input: &[u8],
) -> Result<(), TerminalSocketGatewayError> {
    let mut writer = writer
        .lock()
        .map_err(|_| TerminalSocketGatewayError::WriteFailed)?;
    writer
        .write_all(input)
        .and_then(|_| writer.flush())
        .map_err(|_| TerminalSocketGatewayError::WriteFailed)
}

/// Streams the whole PTY output to the client. Unlike the previous protocol
/// there is no notion of an "active command" here: prompt output, full-screen
/// programs and remote shells all reach the client untouched, and the client
/// derives command blocks from the OSC 133 markers inside the stream.
fn forward_terminal_output(
    mut reader: Box<dyn Read + Send>,
    session: PtySession,
    output: UnboundedSender<TerminalSessionEvent>,
) {
    let mut bytes = [0_u8; READ_BUFFER_BYTES];
    let mut stream = Utf8Stream::default();

    loop {
        let count = match reader.read(&mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };

        let text = stream.push(&bytes[..count]);
        if text.is_empty() {
            continue;
        }

        // Check if output should be suppressed
        let should_suppress = session
            .suppress_output_until
            .lock()
            .ok()
            .and_then(|suppress| suppress.as_ref().map(|&until| Instant::now() < until))
            .unwrap_or(false);

        if !should_suppress {
            if output.send(TerminalSessionEvent::Output(text)).is_err() {
                return;
            }
        }
    }

    let trailing = stream.flush();
    if !trailing.is_empty() {
        let _ = output.send(TerminalSessionEvent::Output(trailing));
    }
    let _ = output.send(TerminalSessionEvent::Exit(session.exit_code()));
}
