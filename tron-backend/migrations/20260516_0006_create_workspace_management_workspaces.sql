CREATE TABLE IF NOT EXISTS workspaces (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NULL,
    parent_workspace_code TEXT NULL REFERENCES workspaces(workspace_code),
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE INDEX IF NOT EXISTS idx_workspaces_parent_workspace_code
    ON workspaces(parent_workspace_code);

CREATE INDEX IF NOT EXISTS idx_workspaces_status
    ON workspaces(status);
