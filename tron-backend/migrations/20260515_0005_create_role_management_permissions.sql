CREATE TABLE IF NOT EXISTS role_permissions (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    role_code TEXT NOT NULL REFERENCES roles(role_code) ON DELETE CASCADE,
    permission_domain TEXT NOT NULL CHECK (
        permission_domain IN ('Workspace', 'Models', 'Tasks', 'Memory', 'Skills', 'MCP')
    ),
    permission_code TEXT NOT NULL CHECK (
        permission_code IN ('R', 'W', 'D', 'A')
    ),
    created_at TEXT NOT NULL,
    UNIQUE(role_code, permission_domain, permission_code)
);

CREATE INDEX IF NOT EXISTS idx_role_permissions_role_code
    ON role_permissions (role_code);
