ALTER TABLE workspace_internal_agent_policies
    ADD COLUMN mcp_access_mode TEXT NOT NULL DEFAULT 'DISABLED';

CREATE TABLE IF NOT EXISTS workspace_internal_agent_mcp_server_assignments (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    server_code TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code, server_code),
    FOREIGN KEY (workspace_code, agent_code)
        REFERENCES workspace_internal_agent_policies (workspace_code, agent_code)
        ON DELETE CASCADE,
    FOREIGN KEY (server_code)
        REFERENCES platform_mcp_servers (server_code)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workspace_internal_agent_mcp_server_assignments_workspace
    ON workspace_internal_agent_mcp_server_assignments (workspace_code, agent_code);
