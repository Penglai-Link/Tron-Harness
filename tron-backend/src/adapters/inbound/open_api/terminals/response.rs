use serde::Serialize;
use utoipa::ToSchema;

use crate::application::terminal::TerminalSocketTicket;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiGenerateTerminalCommandResponse {
    pub model_code: String,
    pub command: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiIssueTerminalSocketTicketResponse {
    pub ticket: String,
    pub expires_at: i64,
    pub socket_path: String,
    pub socket_namespace: String,
}

impl From<TerminalSocketTicket> for OpenApiIssueTerminalSocketTicketResponse {
    fn from(ticket: TerminalSocketTicket) -> Self {
        Self {
            ticket: ticket.ticket,
            expires_at: ticket.expires_at,
            socket_path: ticket.socket_path,
            socket_namespace: ticket.socket_namespace,
        }
    }
}
