PRAGMA foreign_keys = OFF;

CREATE TABLE role_permissions_navigation_upgrade (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    role_code TEXT NOT NULL REFERENCES roles(role_code) ON DELETE CASCADE,
    permission_domain TEXT NOT NULL CHECK (
        permission_domain IN ('Workspace', 'Models', 'Tasks', 'Memory', 'Skills', 'MCP', 'Resources', 'Navigation')
    ),
    permission_code TEXT NOT NULL CHECK (
        permission_code IN ('R', 'W', 'D', 'A')
    ),
    created_at TEXT NOT NULL,
    UNIQUE(role_code, permission_domain, permission_code)
);

INSERT INTO role_permissions_navigation_upgrade (
    id, role_id, role_code, permission_domain, permission_code, created_at
)
SELECT id, role_id, role_code, permission_domain, permission_code, created_at
FROM role_permissions;

DROP TABLE role_permissions;
ALTER TABLE role_permissions_navigation_upgrade RENAME TO role_permissions;

CREATE INDEX IF NOT EXISTS idx_role_permissions_role_code
ON role_permissions (role_code);

INSERT OR IGNORE INTO role_permissions (id, role_id, role_code, permission_domain, permission_code, created_at)
SELECT id + 100000000000, id, role_code, 'Navigation', 'R', '2026-08-29T00:00:00Z'
FROM roles
WHERE role_code IN ('ADMIN', 'WORKSPACE_ADMIN', 'DEVELOPER', 'VIEWER');

INSERT OR IGNORE INTO role_permissions (id, role_id, role_code, permission_domain, permission_code, created_at)
SELECT id + 200000000000, id, role_code, 'Navigation', 'W', '2026-08-29T00:00:00Z'
FROM roles
WHERE role_code = 'ADMIN';

INSERT OR IGNORE INTO role_permissions (id, role_id, role_code, permission_domain, permission_code, created_at)
SELECT id + 300000000000, id, role_code, 'Navigation', 'D', '2026-08-29T00:00:00Z'
FROM roles
WHERE role_code = 'ADMIN';

INSERT OR IGNORE INTO role_permissions (id, role_id, role_code, permission_domain, permission_code, created_at)
SELECT id + 400000000000, id, role_code, 'Navigation', 'A', '2026-08-29T00:00:00Z'
FROM roles
WHERE role_code = 'ADMIN';

PRAGMA foreign_keys = ON;
