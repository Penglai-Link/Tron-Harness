CREATE TABLE IF NOT EXISTS platform_variables (
    id INTEGER PRIMARY KEY,
    variable_key TEXT NOT NULL UNIQUE,
    variable_value TEXT NOT NULL,
    description TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_platform_variables_variable_key
    ON platform_variables(variable_key);
