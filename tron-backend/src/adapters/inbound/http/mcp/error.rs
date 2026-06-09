#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpError {
    WorkspaceRequired,
    InvalidWorkspace,
    ServerNotFound,
    DuplicateServerCode,
    InvalidServerCode,
    InvalidServerName,
    InvalidServerUrl,
    InvalidServerDefinition,
    EffectiveEnableDenied,
    GatewayDisabled,
    HealthCheckFailed(String),
    TransportUnauthorized,
    StorageFailure,
}

impl std::fmt::Display for McpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceRequired => formatter.write_str("workspace code is required"),
            Self::InvalidWorkspace => formatter.write_str("invalid workspace"),
            Self::ServerNotFound => formatter.write_str("MCP server not found"),
            Self::DuplicateServerCode => formatter.write_str("duplicate MCP server code"),
            Self::InvalidServerCode => formatter.write_str("invalid MCP server code"),
            Self::InvalidServerName => formatter.write_str("invalid MCP server name"),
            Self::InvalidServerUrl => formatter.write_str("invalid MCP server URL"),
            Self::InvalidServerDefinition => formatter.write_str("invalid MCP server definition"),
            Self::EffectiveEnableDenied => {
                formatter.write_str("platform-disabled server cannot be effectively enabled")
            }
            Self::GatewayDisabled => formatter.write_str("MCP gateway is disabled"),
            Self::HealthCheckFailed(message) => {
                write!(formatter, "MCP health check failed: {message}")
            }
            Self::TransportUnauthorized => {
                formatter.write_str("MCP transport requires bearer auth")
            }
            Self::StorageFailure => formatter.write_str("MCP storage failure"),
        }
    }
}

impl std::error::Error for McpError {}
