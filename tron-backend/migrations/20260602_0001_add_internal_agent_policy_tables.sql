CREATE TABLE IF NOT EXISTS workspace_internal_agent_policies (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    tool_assignment_mode TEXT NOT NULL,
    tool_exposure_mode TEXT NOT NULL,
    expanded_domain_persistence_mode TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code)
);

CREATE INDEX IF NOT EXISTS idx_workspace_internal_agent_policies_workspace_code
    ON workspace_internal_agent_policies (workspace_code);

CREATE TABLE IF NOT EXISTS workspace_internal_agent_tool_assignments (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    tool_code TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code, tool_code)
);

CREATE INDEX IF NOT EXISTS idx_workspace_internal_agent_tool_assignments_workspace_code
    ON workspace_internal_agent_tool_assignments (workspace_code);

CREATE INDEX IF NOT EXISTS idx_workspace_internal_agent_tool_assignments_agent_code
    ON workspace_internal_agent_tool_assignments (agent_code);
