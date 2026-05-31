CREATE TABLE IF NOT EXISTS conversation_runtime_sessions (
    id INTEGER PRIMARY KEY,
    conversation_code TEXT NOT NULL,
    runtime_adapter_key TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    runtime_session_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    last_run_code TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (conversation_code, runtime_adapter_key, agent_code)
);

CREATE INDEX IF NOT EXISTS idx_conversation_runtime_sessions_lookup
    ON conversation_runtime_sessions(conversation_code, runtime_adapter_key, agent_code);
