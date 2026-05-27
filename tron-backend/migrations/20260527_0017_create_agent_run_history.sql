CREATE TABLE IF NOT EXISTS agent_runs (
  id INTEGER PRIMARY KEY,
  run_code TEXT NOT NULL UNIQUE,
  conversation_code TEXT,
  workspace_code TEXT NOT NULL,
  user_code TEXT,
  app_code TEXT,
  task_code TEXT,
  model_code TEXT NOT NULL,
  status TEXT NOT NULL,
  outcome TEXT NOT NULL,
  step_count INTEGER NOT NULL,
  observation_count INTEGER NOT NULL,
  output_preview TEXT,
  error_code TEXT,
  error_message TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_workspace_created
  ON agent_runs (workspace_code, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_agent_runs_conversation_created
  ON agent_runs (conversation_code, created_at DESC, id DESC);
