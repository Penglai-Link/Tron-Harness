use chrono::Utc;
use socketioxide::{
    SocketIo,
    extract::{SocketRef, State, TryData},
};
use std::future::{Ready, ready};
use tokio::sync::mpsc;

use crate::{application::terminal::TERMINAL_SOCKET_NAMESPACE, shared::locale::SupportedLocale};
use tron_i18n::t;

use super::{
    TerminalSocketGateway,
    protocol::{
        TERMINAL_EXIT_EVENT, TERMINAL_OUTPUT_EVENT, TERMINAL_SESSION_EVENT, TerminalExitMessage,
        TerminalInputPayload, TerminalOutputMessage, TerminalResizePayload,
        TerminalSessionErrorMessage, TerminalSessionEvent, TerminalSessionMessage,
        TerminalSignalPayload,
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
            emit_session_error(
                &socket,
                t!(
                    "socket.terminal.ticket_invalid",
                    locale = socket_locale(&socket)
                ),
            );
            socket.disconnect().ok();
            return;
        }
    };

    let socket_id = socket.id.to_string();
    let (output, mut output_events) = mpsc::unbounded_channel();
    let descriptor = match gateway.open_session(socket_id, auth.ticket, output).await {
        Ok(descriptor) => descriptor,
        Err(error) => {
            emit_session_error(
                &socket,
                t!(error.message_key(), locale = socket_locale(&socket)),
            );
            socket.disconnect().ok();
            return;
        }
    };

    let output_socket = socket.clone();
    tokio::spawn(async move {
        while let Some(event) = output_events.recv().await {
            let emitted = match event {
                TerminalSessionEvent::Output(data) => {
                    output_socket.emit(TERMINAL_OUTPUT_EVENT, &TerminalOutputMessage::new(data))
                }
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

// Socketioxide calls the handler to obtain its future before spawning it.
// Perform these synchronous PTY operations during that call: an async body
// would schedule each packet independently and could reorder typed bytes.
fn write_terminal_input(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<TerminalInputPayload>,
) -> Ready<()> {
    let Ok(payload) = payload else {
        return ready(());
    };
    if let Err(error) = gateway.write_input(&socket.id.to_string(), payload) {
        emit_session_error(
            &socket,
            t!(error.message_key(), locale = socket_locale(&socket)),
        );
    }
    ready(())
}

fn resize_terminal(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<TerminalResizePayload>,
) -> Ready<()> {
    let Ok(payload) = payload else {
        return ready(());
    };
    if let Err(error) = gateway.resize(&socket.id.to_string(), payload) {
        emit_session_error(
            &socket,
            t!(error.message_key(), locale = socket_locale(&socket)),
        );
    }
    ready(())
}

fn signal_terminal(
    socket: SocketRef,
    State(gateway): State<TerminalSocketGateway>,
    TryData(payload): TryData<TerminalSignalPayload>,
) -> Ready<()> {
    let Ok(payload) = payload else {
        return ready(());
    };
    if let Err(error) = gateway.signal(&socket.id.to_string(), payload) {
        emit_session_error(
            &socket,
            t!(error.message_key(), locale = socket_locale(&socket)),
        );
    }
    ready(())
}

async fn terminal_disconnected(socket: SocketRef, State(gateway): State<TerminalSocketGateway>) {
    gateway.close_session(&socket.id.to_string());
}

/// Locale negotiated by the Socket.IO handshake (`Accept-Language`), defaulting
/// to zh-CN when the client does not send one. Request/session locales are not
/// threaded into socket frames, so the handshake is the only available source.
fn socket_locale(socket: &SocketRef) -> SupportedLocale {
    SupportedLocale::from_request(
        socket
            .req_parts()
            .headers
            .get("accept-language")
            .and_then(|value| value.to_str().ok()),
    )
}

fn emit_session_error(socket: &SocketRef, message: impl Into<String>) {
    let _ = socket.emit(
        TERMINAL_SESSION_EVENT,
        &TerminalSessionErrorMessage::new(message),
    );
}
