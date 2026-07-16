INSERT OR IGNORE INTO role_permissions (
    role_id, role_code, permission_domain, permission_code, created_at
)
SELECT
    role_id,
    role_code,
    'Resources',
    permission_code,
    '2026-07-16T00:00:00Z'
FROM (
    SELECT id AS role_id, role_code, 'R' AS permission_code FROM roles WHERE role_code = 'ADMIN'
    UNION ALL SELECT id, role_code, 'W' FROM roles WHERE role_code = 'ADMIN'
    UNION ALL SELECT id, role_code, 'D' FROM roles WHERE role_code = 'ADMIN'
    UNION ALL SELECT id, role_code, 'A' FROM roles WHERE role_code = 'ADMIN'
    UNION ALL SELECT id, role_code, 'R' FROM roles WHERE role_code = 'WORKSPACE_ADMIN'
    UNION ALL SELECT id, role_code, 'W' FROM roles WHERE role_code = 'WORKSPACE_ADMIN'
    UNION ALL SELECT id, role_code, 'R' FROM roles WHERE role_code = 'DEVELOPER'
    UNION ALL SELECT id, role_code, 'W' FROM roles WHERE role_code = 'DEVELOPER'
    UNION ALL SELECT id, role_code, 'R' FROM roles WHERE role_code = 'VIEWER'
);
