PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS platform_mcp_gateway_configs (
    singleton_key TEXT PRIMARY KEY,
    gateway_enabled INTEGER NOT NULL,
    variables_endpoint_enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
