use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::mpsc::UnboundedSender;

use crate::application::terminal::TerminalSessionAuthorization;

use super::protocol::{
    ActiveCommand, COMMAND_MARKER_SUFFIX, TerminalSocketGatewayError, TerminalSocketMessage,
    command_marker_prefix,
};

#[derive(Clone)]
pub(super) struct PtySession {
    active_command: Arc<Mutex<Option<ActiveCommand>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    output: UnboundedSender<TerminalSocketMessage>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl PtySession {
    pub(super) fn spawn(
        authorization: TerminalSessionAuthorization,
        output: UnboundedSender<TerminalSocketMessage>,
    ) -> Result<(Self, String), TerminalSocketGatewayError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        let current_directory = std::fs::canonicalize(&authorization.current_directory)
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .to_string_lossy()
            .into_owned();
        let mut command = terminal_shell_command();
        command.cwd(&current_directory);
        command.env("TERM", "xterm-256color");
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
        initialize_terminal(&mut writer).map_err(|_| TerminalSocketGatewayError::PtyFailed)?;

        let active_command = Arc::new(Mutex::new(None));
        let session_directory = Arc::new(Mutex::new(current_directory.clone()));
        let reader_active_command = active_command.clone();
        let reader_current_directory = session_directory.clone();
        let reader_output = output.clone();
        std::thread::Builder::new()
            .name("tron-terminal-pty".to_owned())
            .spawn(move || {
                forward_terminal_output(
                    reader,
                    reader_active_command,
                    reader_current_directory,
                    reader_output,
                );
            })
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;

        Ok((
            Self {
                active_command,
                child: Arc::new(Mutex::new(child)),
                master: Arc::new(Mutex::new(pair.master)),
                output,
                writer: Arc::new(Mutex::new(writer)),
            },
            current_directory,
        ))
    }

    pub(super) fn execute(
        &self,
        command: &str,
        message_id: String,
        interactive: bool,
        token: &str,
    ) -> Result<(), TerminalSocketGatewayError> {
        {
            let mut active_command = self
                .active_command
                .lock()
                .map_err(|_| TerminalSocketGatewayError::PtyFailed)?;
            if active_command.is_some() {
                return Err(TerminalSocketGatewayError::CommandAlreadyRunning);
            }
            *active_command = Some(ActiveCommand {
                interactive,
                marker_prefix: command_marker_prefix(token),
                message_id: message_id.clone(),
                output_buffer: Vec::new(),
            });
        }

        if self
            .output
            .send(TerminalSocketMessage::start(&message_id))
            .is_err()
        {
            self.clear_active_command(&message_id);
            return Err(TerminalSocketGatewayError::SessionNotFound);
        }
        if let Err(error) =
            write_to_terminal(&self.writer, framed_command(command, token, interactive))
        {
            self.clear_active_command(&message_id);
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn cancel(&self, message_id: &str) -> Result<(), TerminalSocketGatewayError> {
        let active_message_id = self
            .active_command
            .lock()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .as_ref()
            .map(|command| command.message_id.clone())
            .ok_or(TerminalSocketGatewayError::CommandNotFound)?;
        if active_message_id != message_id {
            return Err(TerminalSocketGatewayError::CommandNotFound);
        }
        write_to_terminal(&self.writer, b"\x03".to_vec())
    }

    pub(super) fn write_input(
        &self,
        message_id: &str,
        data: String,
    ) -> Result<(), TerminalSocketGatewayError> {
        let accepts_input = self
            .active_command
            .lock()
            .map_err(|_| TerminalSocketGatewayError::PtyFailed)?
            .as_ref()
            .is_some_and(|command| command.interactive && command.message_id == message_id);
        if !accepts_input {
            return Ok(());
        }
        write_to_terminal(&self.writer, data.into_bytes())
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

    fn clear_active_command(&self, message_id: &str) {
        if let Ok(mut active_command) = self.active_command.lock()
            && active_command
                .as_ref()
                .is_some_and(|command| command.message_id == message_id)
        {
            *active_command = None;
        }
    }
}

fn write_to_terminal(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    input: Vec<u8>,
) -> Result<(), TerminalSocketGatewayError> {
    let mut writer = writer
        .lock()
        .map_err(|_| TerminalSocketGatewayError::WriteFailed)?;
    writer
        .write_all(&input)
        .and_then(|_| writer.flush())
        .map_err(|_| TerminalSocketGatewayError::WriteFailed)
}

fn forward_terminal_output(
    mut reader: Box<dyn Read + Send>,
    active_command: Arc<Mutex<Option<ActiveCommand>>>,
    current_directory: Arc<Mutex<String>>,
    output: UnboundedSender<TerminalSocketMessage>,
) {
    let mut bytes = [0_u8; 4_096];
    loop {
        let count = match reader.read(&mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        let events = {
            let Ok(mut current_directory) = current_directory.lock() else {
                break;
            };
            let Ok(mut active_command) = active_command.lock() else {
                break;
            };
            let Some(command) = active_command.as_mut() else {
                continue;
            };
            command.output_buffer.extend_from_slice(&bytes[..count]);
            let (events, completed) = flush_command_output(command, &mut current_directory);
            if completed {
                *active_command = None;
            }
            events
        };
        for event in events {
            if output.send(event).is_err() {
                return;
            }
        }
    }

    let terminal_closed = active_command
        .lock()
        .ok()
        .and_then(|mut active_command| active_command.take());
    if let Some(command) = terminal_closed {
        let _ = output.send(TerminalSocketMessage::error(
            command.message_id,
            "终端会话已关闭",
        ));
    }
}

fn flush_command_output(
    command: &mut ActiveCommand,
    current_directory: &mut String,
) -> (Vec<TerminalSocketMessage>, bool) {
    let marker_start = find_bytes(&command.output_buffer, &command.marker_prefix);
    let Some(marker_start) = marker_start else {
        let retained = longest_suffix_prefix_length(&command.output_buffer, &command.marker_prefix);
        let output_length = command.output_buffer.len().saturating_sub(retained);
        if output_length == 0 {
            return (Vec::new(), false);
        }
        let content = command
            .output_buffer
            .drain(..output_length)
            .collect::<Vec<_>>();
        return (
            vec![TerminalSocketMessage::content(
                &command.message_id,
                String::from_utf8_lossy(&content).into_owned(),
            )],
            false,
        );
    };

    let marker_end = command.output_buffer[marker_start..]
        .iter()
        .position(|byte| *byte == COMMAND_MARKER_SUFFIX)
        .map(|offset| marker_start + offset);
    let Some(marker_end) = marker_end else {
        let content = command
            .output_buffer
            .drain(..marker_start)
            .collect::<Vec<_>>();
        return (
            (!content.is_empty())
                .then(|| {
                    TerminalSocketMessage::content(
                        &command.message_id,
                        String::from_utf8_lossy(&content).into_owned(),
                    )
                })
                .into_iter()
                .collect(),
            false,
        );
    };

    let content = command
        .output_buffer
        .drain(..marker_start)
        .collect::<Vec<_>>();
    let marker_payload_start = command.marker_prefix.len();
    let marker_payload =
        command.output_buffer[marker_payload_start..marker_end - marker_start].to_vec();
    let _ = command
        .output_buffer
        .split_off(marker_end + 1 - marker_start);
    command.output_buffer.clear();

    if let Some(separator) = marker_payload.iter().position(|byte| *byte == b':') {
        let directory = String::from_utf8_lossy(&marker_payload[separator + 1..])
            .trim()
            .to_owned();
        if !directory.is_empty() {
            *current_directory = directory;
        }
    }

    let mut events = Vec::new();
    if !content.is_empty() {
        events.push(TerminalSocketMessage::content(
            &command.message_id,
            String::from_utf8_lossy(&content).into_owned(),
        ));
    }
    events.push(TerminalSocketMessage::end(
        &command.message_id,
        current_directory.clone(),
    ));
    (events, true)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|candidate| candidate == needle)
}

fn longest_suffix_prefix_length(output: &[u8], marker_prefix: &[u8]) -> usize {
    let max_length = output.len().min(marker_prefix.len().saturating_sub(1));
    (1..=max_length)
        .rev()
        .find(|length| output.ends_with(&marker_prefix[..*length]))
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn terminal_shell_command() -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-i");
    command
}

#[cfg(windows)]
fn terminal_shell_command() -> CommandBuilder {
    CommandBuilder::new("cmd.exe")
}

#[cfg(not(windows))]
fn initialize_terminal(writer: &mut Box<dyn Write + Send>) -> std::io::Result<()> {
    writer.write_all(b"stty -echo; PS1=''; PS2=''\r")?;
    writer.flush()
}

#[cfg(windows)]
fn initialize_terminal(writer: &mut Box<dyn Write + Send>) -> std::io::Result<()> {
    writer.write_all(b"@echo off\r\n")?;
    writer.flush()
}

#[cfg(not(windows))]
fn framed_command(command: &str, token: &str, interactive: bool) -> Vec<u8> {
    let command = shell_single_quote(command);
    let terminal_mode = if interactive {
        "stty echo -echoctl; "
    } else {
        ""
    };
    let restore_terminal_mode = if interactive {
        "stty -echo echoctl; "
    } else {
        ""
    };
    format!(
        "{terminal_mode}eval {command}; tron_command_status=$?; {restore_terminal_mode}printf '\\036TRON_COMMAND_END:{token}:%s:%s\\037\\n' \"$tron_command_status\" \"$PWD\"\r"
    )
    .into_bytes()
}

#[cfg(windows)]
fn framed_command(command: &str, token: &str, _interactive: bool) -> Vec<u8> {
    format!(
        "{command} & set tron_command_status=%ERRORLEVEL% & <nul set /p =\x1eTRON_COMMAND_END:{token}:%tron_command_status%:%cd%\x1f\r\n"
    )
    .into_bytes()
}

#[cfg(not(windows))]
fn shell_single_quote(command: &str) -> String {
    format!("'{}'", command.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_command(token: &str) -> ActiveCommand {
        ActiveCommand {
            interactive: false,
            marker_prefix: command_marker_prefix(token),
            message_id: "MSG-1".into(),
            output_buffer: Vec::new(),
        }
    }

    #[test]
    fn terminal_output_is_completed_by_its_matching_marker() {
        let mut command = active_command("TOKEN-1");
        command.output_buffer.extend_from_slice(
            b"first line\r\n\x1eTRON_COMMAND_END:TOKEN-1:0:/workspace/tron\x1fprompt",
        );
        let mut current_directory = "/workspace/old".into();

        let (events, completed) = flush_command_output(&mut command, &mut current_directory);

        assert!(completed);
        assert_eq!(current_directory, "/workspace/tron");
        assert_eq!(events.len(), 2);
        assert_eq!(
            serde_json::to_value(&events[0]).expect("content event should serialize"),
            serde_json::json!({
                "delta": "first line\r\n",
                "messageId": "MSG-1",
                "timestamp": events[0].timestamp,
                "type": "TEXT_MESSAGE_CONTENT"
            })
        );
        assert_eq!(
            serde_json::to_value(&events[1]).expect("end event should serialize"),
            serde_json::json!({
                "currentDirectory": "/workspace/tron",
                "delta": "",
                "messageId": "MSG-1",
                "timestamp": events[1].timestamp,
                "type": "TEXT_MESSAGE_END"
            })
        );
        assert!(command.output_buffer.is_empty());
    }

    #[test]
    fn terminal_output_retains_partial_marker_until_completed() {
        let mut command = active_command("TOKEN-2");
        command
            .output_buffer
            .extend_from_slice(b"\x1eTRON_COMMAND_END:TOK");
        let mut current_directory = "/workspace/tron".into();

        let (events, completed) = flush_command_output(&mut command, &mut current_directory);

        assert!(!completed);
        assert!(events.is_empty());
        assert_eq!(command.output_buffer, b"\x1eTRON_COMMAND_END:TOK".to_vec());
    }

    #[test]
    fn terminal_output_is_forwarded_without_waiting_for_marker_length() {
        let mut command = active_command("TOKEN-3");
        command.output_buffer.extend_from_slice(b"\x1b[1;1Hh");
        let mut current_directory = "/workspace/tron".into();

        let (events, completed) = flush_command_output(&mut command, &mut current_directory);

        assert!(!completed);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].delta, "\x1b[1;1Hh");
        assert!(command.output_buffer.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn framed_command_preserves_single_quoted_shell_input() {
        let frame = String::from_utf8(framed_command("printf 'hello'", "TOKEN-4", false))
            .expect("command frame should be UTF-8");

        assert!(frame.starts_with("eval 'printf '\"'\"'hello'\"'\"'';"));
        assert!(frame.contains("TRON_COMMAND_END:TOKEN-4"));
    }

    #[cfg(not(windows))]
    #[test]
    fn interactive_framed_command_restores_terminal_echo_after_exit() {
        let frame = String::from_utf8(framed_command("vim edit.txt", "TOKEN-5", true))
            .expect("command frame should be UTF-8");

        assert!(frame.starts_with("stty echo -echoctl; eval 'vim edit.txt';"));
        assert!(frame.contains("tron_command_status=$?; stty -echo echoctl; printf"));
        assert!(frame.contains("TRON_COMMAND_END:TOKEN-5"));
    }
}
