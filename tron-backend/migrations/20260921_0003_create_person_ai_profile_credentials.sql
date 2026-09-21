CREATE TABLE IF NOT EXISTS person_ai_profile_credentials (
    person_ai_user_id TEXT PRIMARY KEY,
    person_ai_username TEXT NOT NULL,
    server_base_url TEXT NOT NULL,
    credentials_encrypted TEXT NOT NULL,
    status TEXT NOT NULL,
    authorized_at INTEGER NOT NULL,
    authorization_expires_at INTEGER,
    last_error TEXT,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_person_ai_profile_credentials_status
    ON person_ai_profile_credentials(status, updated_at DESC);
