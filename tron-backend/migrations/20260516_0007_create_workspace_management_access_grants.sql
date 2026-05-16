CREATE TABLE IF NOT EXISTS workspace_user_access (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL REFERENCES workspaces(workspace_code),
    user_code TEXT NOT NULL,
    is_owner INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (workspace_code, user_code)
);

CREATE TABLE IF NOT EXISTS workspace_role_access (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL REFERENCES workspaces(workspace_code),
    role_code TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (workspace_code, role_code)
);

CREATE INDEX IF NOT EXISTS idx_workspace_user_access_workspace_code
    ON workspace_user_access(workspace_code);

CREATE INDEX IF NOT EXISTS idx_workspace_role_access_workspace_code
    ON workspace_role_access(workspace_code);
