CREATE TABLE IF NOT EXISTS chat_sessions (
  id INTEGER PRIMARY KEY,
  session_code TEXT NOT NULL UNIQUE,
  workspace_code TEXT NOT NULL,
  user_code TEXT,
  app_code TEXT,
  task_code TEXT,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  last_run_code TEXT,
  last_run_status TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_workspace_updated
  ON chat_sessions (workspace_code, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_task_updated
  ON chat_sessions (task_code, updated_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS chat_messages (
  id INTEGER PRIMARY KEY,
  message_code TEXT NOT NULL UNIQUE,
  session_code TEXT NOT NULL,
  run_code TEXT,
  role TEXT NOT NULL,
  channel TEXT NOT NULL,
  actor TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_session_created
  ON chat_messages (session_code, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_chat_messages_run_created
  ON chat_messages (run_code, created_at ASC, id ASC);
