CREATE TABLE IF NOT EXISTS chat_session_auto_model_routes (
    workspace_code TEXT NOT NULL,
    session_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    selection_json TEXT NOT NULL,
    model_code TEXT NOT NULL,
    context_window_tokens INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, session_code, agent_code)
);

CREATE INDEX IF NOT EXISTS idx_chat_session_auto_model_routes_session
    ON chat_session_auto_model_routes(session_code);
