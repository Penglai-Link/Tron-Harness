#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueTerminalSocketTicketCommand {
    pub workspace_code: String,
    pub app_code: Option<String>,
    pub user_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSocketTicket {
    pub ticket: String,
    pub expires_at: i64,
    pub socket_path: String,
    pub socket_namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSessionAuthorization {
    pub app_code: Option<String>,
    pub current_directory: String,
    pub user_code: String,
    pub workspace_code: String,
}
