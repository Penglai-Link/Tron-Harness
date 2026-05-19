CREATE TABLE IF NOT EXISTS workspace_variables (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL,
    variable_key TEXT NOT NULL,
    variable_value TEXT NOT NULL,
    description TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (workspace_code, variable_key)
);

CREATE INDEX IF NOT EXISTS idx_workspace_variables_workspace_code
    ON workspace_variables(workspace_code);

CREATE INDEX IF NOT EXISTS idx_workspace_variables_variable_key
    ON workspace_variables(variable_key);
