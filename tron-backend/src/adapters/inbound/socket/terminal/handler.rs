use chrono::Utc;
use socketioxide::{
    SocketIo,
    extract::{SocketRef, State, TryData},
};
use tokio::sync::mpsc;

use crate::{application::terminal::TERMINAL_SOCKET_NAMESPACE, shared::random::random_base64url};

use super::{
    TerminalSocketGateway,
    protocol::{
        CommandCancelPayload, CommandExecutePayload, CommandInputPayload,
        TERMINAL_COMMAND_MESSAGE_EVENT, TERMINAL_SESSION_EVENT, TerminalMessageType,
        TerminalResizePayload, TerminalSocketAuth, TerminalSocketGatewayError,
        TerminalSocketMessage, generated_message_id,
    },
};

pub fn configure(io: &SocketIo) {
    io.ns(TERMINAL_SOCKET_NAMESPACE, terminal_connected);
}

async fn terminal_connected(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(auth): TryData<TerminalSocketAuth>,
) {
    let auth = match auth {
        Ok(auth) if !auth.ticket.trim().is_empty() => auth,
        Ok(_) | Err(_) => {
            emit_session_error(&socket, "终端连接票据无效");
            socket.disconnect().ok();
            return;
        }
    };

    let socket_id = socket.id.to_string();
    let (output, mut output_events) = mpsc::unbounded_channel();
    let current_directory = match gateway.open_session(socket_id, auth.ticket, output).await {
        Ok(current_directory) => current_directory,
        Err(error) => {
            emit_session_error(&socket, error.to_string());
            socket.disconnect().ok();
            return;
        }
    };

    let output_socket = socket.clone();
    tokio::spawn(async move {
        while let Some(message) = output_events.recv().await {
            if output_socket
                .emit(TERMINAL_COMMAND_MESSAGE_EVENT, &message)
                .is_err()
            {
                break;
            }
        }
    });

    let session_message = TerminalSocketMessage {
        current_directory: Some(current_directory),
        delta: String::new(),
        message_id: generated_message_id(),
        timestamp: Utc::now().timestamp_millis(),
        r#type: TerminalMessageType::TextMessageContent,
    };
    let _ = socket.emit(TERMINAL_SESSION_EVENT, &session_message);
    socket.on("command:execute", execute_command);
    socket.on("command:cancel", cancel_command);
    socket.on("command:input", write_command_input);
    socket.on("terminal:resize", resize_terminal);
    socket.on_disconnect(terminal_disconnected);
}

async fn execute_command(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<CommandExecutePayload>,
) {
    let fallback_message_id = match &payload {
        Ok(payload) => payload.message_id.clone(),
        Err(_) => random_base64url(12),
    };
    let result = payload
        .map_err(|_| TerminalSocketGatewayError::CommandRequired)
        .and_then(|payload| gateway.execute(&socket.id.to_string(), payload));
    if let Err(error) = result {
        emit_command_error(&socket, fallback_message_id, error);
    }
}

async fn cancel_command(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<CommandCancelPayload>,
) {
    let fallback_message_id = match &payload {
        Ok(payload) => payload.message_id.clone(),
        Err(_) => random_base64url(12),
    };
    let result = payload
        .map_err(|_| TerminalSocketGatewayError::MessageIdRequired)
        .and_then(|payload| gateway.cancel(&socket.id.to_string(), payload));
    if let Err(error) = result {
        emit_command_error(&socket, fallback_message_id, error);
    }
}

async fn write_command_input(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<CommandInputPayload>,
) {
    let fallback_message_id = match &payload {
        Ok(payload) => payload.message_id.clone(),
        Err(_) => random_base64url(12),
    };
    let result = payload
        .map_err(|_| TerminalSocketGatewayError::MessageIdRequired)
        .and_then(|payload| gateway.write_input(&socket.id.to_string(), payload));
    if let Err(error) = result {
        emit_command_error(&socket, fallback_message_id, error);
    }
}

async fn resize_terminal(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<TerminalResizePayload>,
) {
    let Ok(payload) = payload else {
        return;
    };
    if let Err(error) = gateway.resize(&socket.id.to_string(), payload) {
        emit_session_error(&socket, error.to_string());
    }
}

async fn terminal_disconnected(socket: SocketRef, State(gateway): State<TerminalSocketGateway>) {
    gateway.close_session(&socket.id.to_string());
}

fn emit_command_error(
    socket: &SocketRef,
    message_id: impl Into<String>,
    error: TerminalSocketGatewayError,
) {
    let message = TerminalSocketMessage::error(message_id, error.to_string());
    let _ = socket.emit(TERMINAL_COMMAND_MESSAGE_EVENT, &message);
}

fn emit_session_error(socket: &SocketRef, message: impl Into<String>) {
    let message = TerminalSocketMessage::error(random_base64url(12), message);
    let _ = socket.emit(TERMINAL_SESSION_EVENT, &message);
}
