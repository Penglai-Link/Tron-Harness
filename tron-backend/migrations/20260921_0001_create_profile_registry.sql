DROP TABLE IF EXISTS profile_runtime_state;

CREATE TABLE IF NOT EXISTS profiles (
    profile_id TEXT PRIMARY KEY NOT NULL,
    person_ai_user_id TEXT NOT NULL DEFAULT '' UNIQUE,
    person_ai_username TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TEXT NOT NULL,
    last_activated_at TEXT
);
