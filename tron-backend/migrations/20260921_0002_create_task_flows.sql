CREATE TABLE IF NOT EXISTS task_flows (
    flow_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS task_flow_statuses (
    flow_code TEXT NOT NULL REFERENCES task_flows(flow_code) ON DELETE CASCADE,
    status_code TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    required INTEGER NOT NULL DEFAULT 0 CHECK (required IN (0, 1)),
    PRIMARY KEY (flow_code, status_code),
    UNIQUE (flow_code, position)
);

CREATE TABLE IF NOT EXISTS workspace_task_flows (
    workspace_code TEXT PRIMARY KEY,
    flow_code TEXT NOT NULL REFERENCES task_flows(flow_code),
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_flow_statuses_flow_position
    ON task_flow_statuses(flow_code, position);

INSERT OR IGNORE INTO task_flows (flow_code, name, created_at, updated_at)
VALUES (
    'DEFAULT',
    '默认流程',
    CAST(strftime('%s', 'now') AS INTEGER) * 1000,
    CAST(strftime('%s', 'now') AS INTEGER) * 1000
);

INSERT OR IGNORE INTO task_flow_statuses (flow_code, status_code, name, position, required)
VALUES
    ('DEFAULT', 'NOT_STARTED', '未开始', 0, 1),
    ('DEFAULT', 'IN_PROGRESS', '进行中', 1, 0),
    ('DEFAULT', 'REOPEN', '重新打开', 2, 0),
    ('DEFAULT', 'CLOSED', '关闭', 3, 1);
