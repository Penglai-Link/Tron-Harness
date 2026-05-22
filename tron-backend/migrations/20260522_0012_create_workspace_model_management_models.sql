CREATE TABLE IF NOT EXISTS platform_models (
    id INTEGER PRIMARY KEY,
    model_code TEXT NOT NULL UNIQUE,
    model_name TEXT NOT NULL,
    purpose TEXT NOT NULL,
    vendor TEXT NOT NULL CHECK (vendor IN ('openai', 'anthropic', 'google')),
    context_window TEXT NOT NULL,
    max_output TEXT NOT NULL,
    training_data TEXT NOT NULL,
    speed TEXT NOT NULL,
    price TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('enabled', 'disabled')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_model_removals (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL,
    model_code TEXT NOT NULL REFERENCES platform_models(model_code),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (workspace_code, model_code)
);

CREATE INDEX IF NOT EXISTS idx_platform_models_status ON platform_models(status);
CREATE INDEX IF NOT EXISTS idx_platform_models_vendor ON platform_models(vendor);
CREATE INDEX IF NOT EXISTS idx_workspace_model_removals_workspace_code
    ON workspace_model_removals(workspace_code);
CREATE INDEX IF NOT EXISTS idx_workspace_model_removals_model_code
    ON workspace_model_removals(model_code);
