use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::application::terminal::TerminalError;

pub(super) const TERMINAL_OUTPUT_EVENT: &str = "terminal:output";
pub(super) const TERMINAL_SESSION_EVENT: &str = "terminal:session";
pub(super) const TERMINAL_EXIT_EVENT: &str = "terminal:exit";
pub(super) const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub(super) enum TerminalSocketGatewayError {
    InputTooLarge,
    PtyFailed,
    ResizeFailed,
    SessionNotFound,
    Terminal(TerminalError),
    WriteFailed,
}

impl std::fmt::Display for TerminalSocketGatewayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge => formatter.write_str("终端输入过长"),
            Self::PtyFailed => formatter.write_str("终端会话无法启动"),
            Self::ResizeFailed => formatter.write_str("终端窗口大小无法调整"),
            Self::SessionNotFound => formatter.write_str("终端会话不存在"),
            Self::Terminal(error) => write!(formatter, "{error}"),
            Self::WriteFailed => formatter.write_str("终端输入无法发送"),
        }
    }
}

impl From<TerminalError> for TerminalSocketGatewayError {
    fn from(error: TerminalError) -> Self {
        Self::Terminal(error)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TerminalSocketAuth {
    pub(super) ticket: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TerminalInputPayload {
    pub(super) data: String,
    /// When true, clear the input buffer before processing.
    /// Used when frontend detects natural language to prevent execution.
    #[serde(default)]
    pub(super) clear_buffer: bool,
    /// Complete rendered input line when `data` submits it.
    #[serde(default)]
    pub(super) input_line: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TerminalResizePayload {
    pub(super) cols: u16,
    pub(super) rows: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TerminalSignalPayload {
    pub(super) signal: TerminalSignal,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum TerminalSignal {
    Sigint,
    Sigkill,
}

/// Anything the PTY reader thread hands back to the socket task.
#[derive(Debug, Clone)]
pub(super) enum TerminalSessionEvent {
    Output(String),
    Exit(Option<i32>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TerminalOutputMessage {
    pub(super) data: String,
    pub(super) timestamp: i64,
}

impl TerminalOutputMessage {
    pub(super) fn new(data: String) -> Self {
        Self {
            data,
            timestamp: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TerminalExitMessage {
    pub(super) exit_code: Option<i32>,
    pub(super) timestamp: i64,
}

impl TerminalExitMessage {
    pub(super) fn new(exit_code: Option<i32>) -> Self {
        Self {
            exit_code,
            timestamp: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TerminalSessionMessage {
    pub(super) current_directory: String,
    /// `bash`, `zsh` or `sh`.
    pub(super) shell: String,
    /// False when the shell cannot report OSC 133 prompt boundaries, so the
    /// client must degrade to a plain terminal without command blocks.
    pub(super) supports_blocks: bool,
    /// One-liner that installs the same hooks into a subshell the server did
    /// not spawn, used to keep blocks working across `ssh`.
    pub(super) subshell_bootstrap: String,
    pub(super) timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TerminalSessionErrorMessage {
    pub(super) error: String,
    pub(super) timestamp: i64,
}

impl TerminalSessionErrorMessage {
    pub(super) fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            timestamp: Utc::now().timestamp_millis(),
        }
    }
}

/// Splits a byte stream into UTF-8 safe chunks.
///
/// PTY reads land on arbitrary byte boundaries, so a multi-byte character can
/// straddle two reads. Decoding each read on its own would replace those halves
/// with U+FFFD and corrupt any non-ASCII output.
#[derive(Debug, Default)]
pub(super) struct Utf8Stream {
    carry: Vec<u8>,
}

impl Utf8Stream {
    /// Returns the text that is complete so far, holding back at most three
    /// bytes of a partial trailing character until the next read completes it.
    pub(super) fn push(&mut self, bytes: &[u8]) -> String {
        self.carry.extend_from_slice(bytes);
        let mut text = String::new();

        loop {
            match std::str::from_utf8(&self.carry) {
                Ok(valid) => {
                    text.push_str(valid);
                    self.carry.clear();
                    return text;
                }
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    // Borrowed, not copied: this prefix was just reported valid.
                    text.push_str(&String::from_utf8_lossy(&self.carry[..valid_up_to]));

                    match error.error_len() {
                        // An incomplete trailing character: hold it back until
                        // the next read completes it.
                        None => {
                            self.carry.drain(..valid_up_to);
                            return text;
                        }
                        // Genuinely invalid bytes: substitute them and keep
                        // decoding, so valid text behind them is not delayed.
                        Some(length) => {
                            text.push(char::REPLACEMENT_CHARACTER);
                            self.carry.drain(..valid_up_to + length);
                        }
                    }
                }
            }
        }
    }

    /// Flushes any bytes still held back, used when the PTY reaches EOF.
    pub(super) fn flush(&mut self) -> String {
        if self.carry.is_empty() {
            return String::new();
        }

        let carry = std::mem::take(&mut self.carry);
        String::from_utf8_lossy(&carry).into_owned()
    }
}
