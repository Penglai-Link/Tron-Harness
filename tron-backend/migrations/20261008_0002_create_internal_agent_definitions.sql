CREATE TABLE IF NOT EXISTS workspace_internal_agent_definitions (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    name TEXT,
    role_prompt TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code)
);

CREATE TABLE IF NOT EXISTS platform_internal_agent_configs (
    agent_code TEXT PRIMARY KEY,
    config_json TEXT NOT NULL
);
