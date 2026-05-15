CREATE TABLE IF NOT EXISTS roles (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    role_code TEXT NOT NULL UNIQUE
        CHECK (role_code = upper(role_code))
        CHECK (role_code NOT GLOB '*[^A-Z0-9_]*'),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL CHECK (kind IN ('system', 'custom')),
    managed_by TEXT NOT NULL CHECK (managed_by IN ('platform', 'workspace')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_roles_role_code
    ON roles (role_code);
