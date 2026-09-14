ALTER TABLE chat_sessions ADD COLUMN project_code TEXT;
CREATE INDEX IF NOT EXISTS idx_chat_sessions_project_updated
    ON chat_sessions(project_code, updated_at DESC, id DESC);
