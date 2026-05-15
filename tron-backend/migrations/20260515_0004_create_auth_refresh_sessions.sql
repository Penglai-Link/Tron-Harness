PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS auth_refresh_sessions (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    session_code TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    user_code TEXT NOT NULL,
    auth_method TEXT NOT NULL,
    provider_code TEXT,
    refresh_token_hash TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_used_at INTEGER,
    revoked_at INTEGER,
    CONSTRAINT ck_auth_refresh_sessions_session_code CHECK (
        length(trim(session_code)) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_auth_refresh_sessions_user_id CHECK (user_id > 0),
    CONSTRAINT ck_auth_refresh_sessions_user_code_format CHECK (
        length(trim(user_code)) > 0
        AND user_code = upper(user_code)
        AND user_code NOT GLOB '*[^A-Z0-9_-]*'
    ),
    CONSTRAINT ck_auth_refresh_sessions_auth_method CHECK (
        auth_method IN ('internal', 'oauth2')
    ),
    CONSTRAINT ck_auth_refresh_sessions_provider_code CHECK (
        provider_code IS NULL
        OR length(trim(provider_code)) BETWEEN 1 AND 128
    ),
    CONSTRAINT ck_auth_refresh_sessions_refresh_token_hash CHECK (
        length(trim(refresh_token_hash)) > 0
    ),
    CONSTRAINT ck_auth_refresh_sessions_expires_at CHECK (expires_at >= 0),
    CONSTRAINT ck_auth_refresh_sessions_created_at CHECK (created_at >= 0),
    CONSTRAINT ck_auth_refresh_sessions_updated_at CHECK (updated_at >= created_at),
    CONSTRAINT ck_auth_refresh_sessions_last_used_at CHECK (
        last_used_at IS NULL
        OR last_used_at >= created_at
    ),
    CONSTRAINT ck_auth_refresh_sessions_revoked_at CHECK (
        revoked_at IS NULL
        OR revoked_at >= created_at
    ),
    CONSTRAINT fk_auth_refresh_sessions_user FOREIGN KEY (user_id, user_code)
        REFERENCES users (id, user_code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_auth_refresh_sessions_session_code
    ON auth_refresh_sessions (session_code);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_user_code
    ON auth_refresh_sessions (user_code);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_sessions_expires_at
    ON auth_refresh_sessions (expires_at);
