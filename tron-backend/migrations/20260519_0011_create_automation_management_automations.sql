CREATE TABLE IF NOT EXISTS automations (
    id INTEGER PRIMARY KEY,
    automation_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NULL,
    prompt_text TEXT NOT NULL,
    cron_expression TEXT NOT NULL,
    scheduler_expression TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    status TEXT NOT NULL CHECK (status IN ('Active', 'Paused', 'Archived')),
    previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Active', 'Paused')),
    last_execution_status TEXT NULL CHECK (last_execution_status IS NULL OR last_execution_status IN ('Pending', 'Running', 'Success', 'Failed', 'Canceled', 'Skipped')),
    last_execution_code TEXT NULL,
    last_execution_summary TEXT NULL,
    last_started_at INTEGER NULL,
    last_finished_at INTEGER NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE TABLE IF NOT EXISTS automation_execution_records (
    id INTEGER PRIMARY KEY,
    execution_code TEXT NOT NULL UNIQUE,
    automation_code TEXT NOT NULL REFERENCES automations(automation_code),
    workspace_code TEXT NOT NULL,
    trigger_source TEXT NOT NULL CHECK (trigger_source IN ('Scheduler', 'Manual')),
    status TEXT NOT NULL CHECK (status IN ('Pending', 'Running', 'Success', 'Failed', 'Canceled', 'Skipped')),
    summary TEXT NULL,
    scheduled_at INTEGER NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_automations_workspace_status ON automations(workspace_code, status);
CREATE INDEX IF NOT EXISTS idx_automations_workspace_updated ON automations(workspace_code, updated_at);
CREATE INDEX IF NOT EXISTS idx_automation_executions_automation_started ON automation_execution_records(automation_code, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_automation_executions_workspace_started ON automation_execution_records(workspace_code, started_at DESC);
