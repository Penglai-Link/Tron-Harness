CREATE TABLE IF NOT EXISTS resource_chat_session_attachments (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    session_code TEXT NOT NULL COLLATE BINARY,
    attachment_code TEXT NOT NULL COLLATE BINARY,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    byte_size INTEGER NOT NULL CHECK (byte_size > 0),
    storage_locator TEXT NOT NULL COLLATE BINARY,
    created_at INTEGER NOT NULL,
    UNIQUE(session_code, attachment_code)
);

CREATE INDEX IF NOT EXISTS idx_resource_chat_session_attachments_workspace
    ON resource_chat_session_attachments (workspace_code);
CREATE INDEX IF NOT EXISTS idx_resource_chat_session_attachments_session
    ON resource_chat_session_attachments (session_code, created_at, id);
