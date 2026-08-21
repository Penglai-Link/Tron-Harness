CREATE TABLE IF NOT EXISTS chat_user_input_requests (
    id INTEGER PRIMARY KEY,
    request_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    run_code TEXT NOT NULL,
    assistant_message_code TEXT,
    intro TEXT NOT NULL,
    questions_json TEXT NOT NULL,
    answers_json TEXT,
    model_code TEXT,
    reasoning_effort TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING',
    requested_at INTEGER NOT NULL,
    answered_at INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_chat_user_input_requests_pending_session
    ON chat_user_input_requests (session_code)
    WHERE status = 'PENDING';

CREATE INDEX IF NOT EXISTS idx_chat_user_input_requests_session_requested
    ON chat_user_input_requests (session_code, requested_at DESC, id DESC);
