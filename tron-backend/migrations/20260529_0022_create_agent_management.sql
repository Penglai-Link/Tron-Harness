CREATE TABLE IF NOT EXISTS platform_main_agent_configs (
    singleton_key TEXT PRIMARY KEY,
    personality TEXT NOT NULL DEFAULT '',
    instructions TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS system_agent_registries (
    agent_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    command_name TEXT NOT NULL,
    source TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    available INTEGER NOT NULL DEFAULT 0,
    last_scanned_at INTEGER,
    last_seen_at INTEGER,
    last_error TEXT
);

CREATE TABLE IF NOT EXISTS workspace_system_agent_policies (
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, agent_code)
);

CREATE INDEX IF NOT EXISTS idx_workspace_system_agent_policies_workspace_code
    ON workspace_system_agent_policies (workspace_code);
