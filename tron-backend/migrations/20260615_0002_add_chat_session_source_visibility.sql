ALTER TABLE chat_sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'USER_CHAT';
ALTER TABLE chat_sessions ADD COLUMN visibility TEXT NOT NULL DEFAULT 'VISIBLE';

CREATE INDEX IF NOT EXISTS idx_chat_sessions_visibility_updated
    ON chat_sessions (visibility, updated_at DESC, id DESC);
