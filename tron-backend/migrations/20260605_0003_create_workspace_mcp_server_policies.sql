PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS workspace_mcp_server_policies (
    workspace_code TEXT NOT NULL,
    server_code TEXT NOT NULL REFERENCES platform_mcp_servers(server_code) ON DELETE CASCADE,
    enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, server_code)
);

CREATE INDEX IF NOT EXISTS idx_workspace_mcp_server_policies_workspace_code
    ON workspace_mcp_server_policies (workspace_code);
