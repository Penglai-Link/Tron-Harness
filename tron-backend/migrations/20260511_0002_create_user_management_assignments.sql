PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS user_role_assignments (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    user_id INTEGER NOT NULL,
    user_code TEXT NOT NULL,
    role_code TEXT NOT NULL,
    assigned_at INTEGER NOT NULL,
    assigned_by INTEGER,
    CONSTRAINT ck_user_role_assignments_user_id CHECK (user_id > 0),
    CONSTRAINT ck_user_role_assignments_user_code_format CHECK (
        length(trim(user_code)) > 0
        AND user_code = upper(user_code)
        AND user_code NOT GLOB '*[^A-Z0-9_]*'
    ),
    CONSTRAINT ck_user_role_assignments_role_code CHECK (
        role_code IN ('ADMIN', 'WORKSPACE_ADMIN', 'DEVELOPER', 'VIEWER')
    ),
    CONSTRAINT ck_user_role_assignments_assigned_at CHECK (assigned_at >= 0),
    CONSTRAINT ck_user_role_assignments_assigned_by CHECK (
        assigned_by IS NULL
        OR assigned_by > 0
    ),
    CONSTRAINT fk_user_role_assignments_user FOREIGN KEY (user_id, user_code)
        REFERENCES users (id, user_code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_user_role_assignments_user_role
    ON user_role_assignments (user_id, role_code);

CREATE INDEX IF NOT EXISTS idx_user_role_assignments_user_code
    ON user_role_assignments (user_code);

CREATE INDEX IF NOT EXISTS idx_user_role_assignments_role_code
    ON user_role_assignments (role_code);

CREATE TABLE IF NOT EXISTS user_space_assignments (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    user_id INTEGER NOT NULL,
    user_code TEXT NOT NULL,
    space_code TEXT NOT NULL,
    assigned_at INTEGER NOT NULL,
    assigned_by INTEGER,
    CONSTRAINT ck_user_space_assignments_user_id CHECK (user_id > 0),
    CONSTRAINT ck_user_space_assignments_user_code_format CHECK (
        length(trim(user_code)) > 0
        AND user_code = upper(user_code)
        AND user_code NOT GLOB '*[^A-Z0-9_]*'
    ),
    CONSTRAINT ck_user_space_assignments_space_code CHECK (
        length(trim(space_code)) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_user_space_assignments_assigned_at CHECK (assigned_at >= 0),
    CONSTRAINT ck_user_space_assignments_assigned_by CHECK (
        assigned_by IS NULL
        OR assigned_by > 0
    ),
    CONSTRAINT fk_user_space_assignments_user FOREIGN KEY (user_id, user_code)
        REFERENCES users (id, user_code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_user_space_assignments_user_space
    ON user_space_assignments (user_id, space_code);

CREATE INDEX IF NOT EXISTS idx_user_space_assignments_user_code
    ON user_space_assignments (user_code);

CREATE INDEX IF NOT EXISTS idx_user_space_assignments_space_code
    ON user_space_assignments (space_code);
