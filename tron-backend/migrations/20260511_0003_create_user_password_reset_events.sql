PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS user_password_reset_events (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    reset_code TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    user_code TEXT NOT NULL,
    password_hash_algorithm TEXT NOT NULL DEFAULT 'bcrypt',
    password_reset_required_after INTEGER NOT NULL DEFAULT 1,
    temporary_password_returned INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    created_by INTEGER,
    CONSTRAINT ck_user_password_reset_events_reset_code CHECK (
        length(trim(reset_code)) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_user_password_reset_events_user_id CHECK (user_id > 0),
    CONSTRAINT ck_user_password_reset_events_user_code_format CHECK (
        length(trim(user_code)) > 0
        AND user_code = upper(user_code)
        AND user_code NOT GLOB '*[^A-Z0-9_]*'
    ),
    CONSTRAINT ck_user_password_reset_events_hash_algorithm CHECK (
        password_hash_algorithm = 'bcrypt'
    ),
    CONSTRAINT ck_user_password_reset_events_reset_required_after CHECK (
        password_reset_required_after IN (0, 1)
    ),
    CONSTRAINT ck_user_password_reset_events_temporary_password_returned CHECK (
        temporary_password_returned IN (0, 1)
    ),
    CONSTRAINT ck_user_password_reset_events_created_at CHECK (created_at >= 0),
    CONSTRAINT ck_user_password_reset_events_created_by CHECK (
        created_by IS NULL
        OR created_by > 0
    ),
    CONSTRAINT fk_user_password_reset_events_user FOREIGN KEY (user_id, user_code)
        REFERENCES users (id, user_code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_user_password_reset_events_reset_code
    ON user_password_reset_events (reset_code);

CREATE INDEX IF NOT EXISTS idx_user_password_reset_events_user_code
    ON user_password_reset_events (user_code);

CREATE INDEX IF NOT EXISTS idx_user_password_reset_events_created_at
    ON user_password_reset_events (created_at);
