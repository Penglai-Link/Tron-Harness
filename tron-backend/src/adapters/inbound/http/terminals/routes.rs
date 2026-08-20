use axum::{Router, routing::post};

use crate::app::AppState;

use super::handler::{generate_command, issue_socket_ticket};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/terminals/socket-tickets",
            post(issue_socket_ticket),
        )
        .route(
            "/api/v1/terminals/commands/generate",
            post(generate_command),
        )
}
