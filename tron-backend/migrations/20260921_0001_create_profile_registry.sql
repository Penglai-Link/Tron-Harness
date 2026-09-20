CREATE TABLE IF NOT EXISTS profiles (
    profile_id TEXT PRIMARY KEY NOT NULL,
    person_ai_user_id TEXT NOT NULL DEFAULT '' UNIQUE,
    person_ai_username TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TEXT NOT NULL,
    last_activated_at TEXT
);

CREATE TABLE IF NOT EXISTS profile_runtime_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    active_profile_id TEXT REFERENCES profiles(profile_id)
);

INSERT OR IGNORE INTO profile_runtime_state(singleton, active_profile_id)
VALUES (1, NULL);
