use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{application::terminal::TerminalError, shared::random::random_base64url};

pub(super) const TERMINAL_COMMAND_MESSAGE_EVENT: &str = "command:message";
pub(super) const TERMINAL_SESSION_EVENT: &str = "terminal:session";
pub(super) const COMMAND_MARKER_PREFIX: &[u8] = b"\x1eTRON_COMMAND_END:";
pub(super) const COMMAND_MARKER_SUFFIX: u8 = b'\x1f';
pub(super) const MAX_COMMAND_BYTES: usize = 64 * 1024;
pub(super) const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub(super) enum TerminalSocketGatewayError {
    CommandAlreadyRunning,
    CommandNotFound,
    CommandRequired,
    InputTooLarge,
    MessageIdRequired,
    PtyFailed,
    ResizeFailed,
    SessionNotFound,
    Terminal(TerminalError),
    WriteFailed,
}

impl std::fmt::Display for TerminalSocketGatewayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandAlreadyRunning => formatter.write_str("已有命令正在执行"),
            Self::CommandNotFound => formatter.write_str("未找到正在执行的命令"),
            Self::CommandRequired => formatter.write_str("命令不能为空"),
            Self::InputTooLarge => formatter.write_str("终端输入过长"),
            Self::MessageIdRequired => formatter.write_str("命令消息 ID 不能为空"),
            Self::PtyFailed => formatter.write_str("终端会话无法启动"),
            Self::ResizeFailed => formatter.write_str("终端窗口大小无法调整"),
            Self::SessionNotFound => formatter.write_str("终端会话不存在"),
            Self::Terminal(error) => write!(formatter, "{error}"),
            Self::WriteFailed => formatter.write_str("终端命令无法发送"),
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
pub(super) struct CommandExecutePayload {
    pub(super) command: String,
    pub(super) interactive: Option<bool>,
    pub(super) message_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CommandCancelPayload {
    pub(super) message_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CommandInputPayload {
    pub(super) data: String,
    pub(super) message_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TerminalResizePayload {
    pub(super) cols: u16,
    pub(super) rows: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TerminalSocketMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) current_directory: Option<String>,
    pub(super) delta: String,
    pub(super) message_id: String,
    pub(super) timestamp: i64,
    pub(super) r#type: TerminalMessageType,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum TerminalMessageType {
    TextMessageContent,
    TextMessageEnd,
    TextMessageError,
    TextMessageStart,
}

impl TerminalSocketMessage {
    pub(super) fn content(message_id: impl Into<String>, delta: impl Into<String>) -> Self {
        Self {
            current_directory: None,
            delta: delta.into(),
            message_id: message_id.into(),
            timestamp: Utc::now().timestamp_millis(),
            r#type: TerminalMessageType::TextMessageContent,
        }
    }

    pub(super) fn end(message_id: impl Into<String>, current_directory: String) -> Self {
        Self {
            current_directory: Some(current_directory),
            delta: String::new(),
            message_id: message_id.into(),
            timestamp: Utc::now().timestamp_millis(),
            r#type: TerminalMessageType::TextMessageEnd,
        }
    }

    pub(super) fn error(message_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            current_directory: None,
            delta: message.into(),
            message_id: message_id.into(),
            timestamp: Utc::now().timestamp_millis(),
            r#type: TerminalMessageType::TextMessageError,
        }
    }

    pub(super) fn start(message_id: impl Into<String>) -> Self {
        Self {
            current_directory: None,
            delta: String::new(),
            message_id: message_id.into(),
            timestamp: Utc::now().timestamp_millis(),
            r#type: TerminalMessageType::TextMessageStart,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ActiveCommand {
    pub(super) interactive: bool,
    pub(super) marker_prefix: Vec<u8>,
    pub(super) message_id: String,
    pub(super) output_buffer: Vec<u8>,
}

pub(super) fn command_marker_prefix(token: &str) -> Vec<u8> {
    let mut marker = COMMAND_MARKER_PREFIX.to_vec();
    marker.extend_from_slice(token.as_bytes());
    marker.push(b':');
    marker
}

pub(super) fn required_message_id(
    message_id: String,
) -> Result<String, TerminalSocketGatewayError> {
    let message_id = message_id.trim();
    if message_id.is_empty() {
        Err(TerminalSocketGatewayError::MessageIdRequired)
    } else {
        Ok(message_id.to_owned())
    }
}

pub(super) fn generated_message_id() -> String {
    random_base64url(12)
}
