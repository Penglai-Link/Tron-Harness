ALTER TABLE chat_sessions
  ADD COLUMN source_type TEXT NOT NULL DEFAULT 'MANUAL';

ALTER TABLE chat_sessions
  ADD COLUMN source_code TEXT;

UPDATE chat_sessions
SET source_type = CASE
  WHEN task_code IS NOT NULL AND trim(task_code) <> '' THEN 'TASK'
  ELSE 'MANUAL'
END
WHERE source_type IS NULL OR source_type = 'MANUAL';

CREATE INDEX IF NOT EXISTS idx_chat_sessions_source_updated
  ON chat_sessions (source_type, source_code, updated_at DESC, id DESC);
