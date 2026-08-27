use chrono::Utc;
use socketioxide::{
    SocketIo,
    extract::{SocketRef, State, TryData},
};
use tokio::sync::mpsc;

use crate::application::terminal::TERMINAL_SOCKET_NAMESPACE;

use super::{
    TerminalSocketGateway,
    protocol::{
        TERMINAL_EXIT_EVENT, TERMINAL_OUTPUT_EVENT, TERMINAL_SESSION_EVENT, TerminalExitMessage,
        TerminalInputPayload, TerminalOutputMessage, TerminalResizePayload, TerminalSessionEvent,
        TerminalSessionErrorMessage, TerminalSessionMessage, TerminalSignalPayload,
    },
};

pub fn configure(io: &SocketIo) {
    io.ns(TERMINAL_SOCKET_NAMESPACE, terminal_connected);
}

async fn terminal_connected(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(auth): TryData<super::protocol::TerminalSocketAuth>,
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
    let descriptor = match gateway.open_session(socket_id, auth.ticket, output).await {
        Ok(descriptor) => descriptor,
        Err(error) => {
            emit_session_error(&socket, error.to_string());
            socket.disconnect().ok();
            return;
        }
    };

    let output_socket = socket.clone();
    tokio::spawn(async move {
        while let Some(event) = output_events.recv().await {
            let emitted = match event {
                TerminalSessionEvent::Output(data) => output_socket
                    .emit(TERMINAL_OUTPUT_EVENT, &TerminalOutputMessage::new(data)),
                TerminalSessionEvent::Exit(exit_code) => {
                    let _ = output_socket
                        .emit(TERMINAL_EXIT_EVENT, &TerminalExitMessage::new(exit_code));
                    break;
                }
            };
            if emitted.is_err() {
                break;
            }
        }
    });

    let session_message = TerminalSessionMessage {
        current_directory: descriptor.current_directory,
        shell: descriptor.shell,
        supports_blocks: descriptor.supports_blocks,
        subshell_bootstrap: descriptor.subshell_bootstrap,
        timestamp: Utc::now().timestamp_millis(),
    };
    let _ = socket.emit(TERMINAL_SESSION_EVENT, &session_message);

    socket.on("terminal:input", write_terminal_input);
    socket.on("terminal:resize", resize_terminal);
    socket.on("terminal:signal", signal_terminal);
    socket.on_disconnect(terminal_disconnected);
}

async fn write_terminal_input(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<TerminalInputPayload>,
) {
    let Ok(payload) = payload else {
        return;
    };
    if let Err(error) = gateway.write_input(&socket.id.to_string(), payload) {
        emit_session_error(&socket, error.to_string());
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

async fn signal_terminal(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<TerminalSignalPayload>,
) {
    let Ok(payload) = payload else {
        return;
    };
    if let Err(error) = gateway.signal(&socket.id.to_string(), payload) {
        emit_session_error(&socket, error.to_string());
    }
}

async fn terminal_disconnected(socket: SocketRef, State(gateway): State<TerminalSocketGateway>) {
    gateway.close_session(&socket.id.to_string());
}

fn emit_session_error(socket: &SocketRef, message: impl Into<String>) {
    let _ = socket.emit(
        TERMINAL_SESSION_EVENT,
        &TerminalSessionErrorMessage::new(message),
    );
}
