CREATE TABLE IF NOT EXISTS profile_avatar_cache (
    person_ai_user_id TEXT PRIMARY KEY NOT NULL,
    content_type TEXT NOT NULL,
    image_bytes BLOB NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (person_ai_user_id) REFERENCES profiles(person_ai_user_id) ON DELETE CASCADE
);
