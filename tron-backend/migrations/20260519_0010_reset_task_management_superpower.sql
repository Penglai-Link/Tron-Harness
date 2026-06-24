DROP TABLE IF EXISTS task_user_assignments;
DROP TABLE IF EXISTS task_agent_links;
DROP TABLE IF EXISTS task_app_links;
DROP TABLE IF EXISTS tasks;

CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY,
    task_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    parent_task_code TEXT NULL REFERENCES tasks(task_code),
    title TEXT NOT NULL,
    description TEXT NULL,
    execution_result TEXT NULL,
    status TEXT NOT NULL CHECK (status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed', 'Archived')),
    previous_status TEXT NULL CHECK (previous_status IS NULL OR previous_status IN ('Open', 'Designing', 'Red', 'Green', 'Regressioning', 'Review', 'InProgress', 'Reopen', 'Completed')),
    priority TEXT NOT NULL CHECK (priority IN ('Low', 'Medium', 'High', 'Extra High')),
    task_type TEXT NOT NULL CHECK (task_type IN ('Feature', 'Bug')),
    source_type TEXT NOT NULL CHECK (source_type IN ('Internal', 'GitHub', 'GitLab')),
    source_url TEXT NULL,
    progress_value INTEGER NOT NULL DEFAULT 0 CHECK (progress_value >= 0 AND progress_value <= 100),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE TABLE IF NOT EXISTS task_user_assignments (
    id INTEGER PRIMARY KEY,
    task_code TEXT NOT NULL REFERENCES tasks(task_code),
    user_code TEXT NOT NULL,
    assigned_at INTEGER NOT NULL,
    UNIQUE (task_code, user_code)
);

CREATE TABLE IF NOT EXISTS task_agent_links (
    id INTEGER PRIMARY KEY,
    task_code TEXT NOT NULL REFERENCES tasks(task_code),
    agent_code TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    UNIQUE (task_code, agent_code)
);

CREATE TABLE IF NOT EXISTS task_app_links (
    id INTEGER PRIMARY KEY,
    task_code TEXT NOT NULL REFERENCES tasks(task_code),
    app_code TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    UNIQUE (task_code, app_code)
);

CREATE INDEX IF NOT EXISTS idx_tasks_workspace_status ON tasks(workspace_code, status);
CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_code ON tasks(parent_task_code);
CREATE INDEX IF NOT EXISTS idx_task_user_assignments_user_code ON task_user_assignments(user_code);
CREATE INDEX IF NOT EXISTS idx_task_user_assignments_task_code ON task_user_assignments(task_code);
CREATE INDEX IF NOT EXISTS idx_task_agent_links_task_code ON task_agent_links(task_code);
CREATE INDEX IF NOT EXISTS idx_task_app_links_task_code ON task_app_links(task_code);
