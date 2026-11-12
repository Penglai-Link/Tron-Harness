use url::Url;

use super::error::McpError;

pub fn normalize_mcp_server_code(value: &str) -> Result<String, McpError> {
    let normalized = value.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(McpError::InvalidServerCode);
    }
    if normalized.chars().any(|character| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '.'
            || character == '_'
            || character == '-')
    }) {
        return Err(McpError::InvalidServerCode);
    }
    Ok(normalized)
}

pub fn validate_mcp_server_name(value: &str) -> Result<String, McpError> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(McpError::InvalidServerName);
    }
    Ok(normalized)
}

pub fn validate_mcp_server_url(value: &str) -> Result<String, McpError> {
    let normalized = value.trim().to_string();
    let parsed = Url::parse(&normalized).map_err(|_| McpError::InvalidServerUrl)?;
    match parsed.scheme() {
        "http" | "https" => Ok(normalized),
        _ => Err(McpError::InvalidServerUrl),
    }
}
