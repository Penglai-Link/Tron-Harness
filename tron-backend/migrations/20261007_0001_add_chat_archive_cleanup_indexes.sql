CREATE INDEX IF NOT EXISTS idx_chat_sessions_archive_updated
    ON chat_sessions(status, visibility, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_archive_workspace_updated
    ON chat_sessions(status, visibility, workspace_code, updated_at DESC, id DESC);
