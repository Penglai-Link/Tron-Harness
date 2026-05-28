CREATE TABLE IF NOT EXISTS models (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    workspace_code TEXT NULL,
    model_code TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    vendor TEXT NOT NULL,
    protocol TEXT NOT NULL,
    model_type TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_path TEXT NULL,
    api_key_ref TEXT NOT NULL,
    model_name TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    description TEXT NULL,
    thinking_enabled INTEGER NOT NULL,
    structured_output_enabled INTEGER NOT NULL,
    temperature REAL NULL,
    max_tokens INTEGER NULL,
    context_window_tokens INTEGER NULL,
    top_p REAL NULL,
    timeout_ms INTEGER NULL,
    max_retries INTEGER NULL,
    last_verified_at INTEGER NULL,
    last_verify_status TEXT NULL,
    last_verify_message TEXT NULL,
    source_kind TEXT NOT NULL,
    source_model_code TEXT NULL,
    allow_inheritance INTEGER NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_models_platform_display_name
    ON models(display_name)
    WHERE scope_kind = 'platform';

CREATE UNIQUE INDEX IF NOT EXISTS idx_models_workspace_display_name
    ON models(workspace_code, display_name)
    WHERE scope_kind = 'workspace';

CREATE INDEX IF NOT EXISTS idx_models_scope_workspace
    ON models(scope_kind, workspace_code);

CREATE INDEX IF NOT EXISTS idx_models_source_model_code
    ON models(source_model_code);

CREATE TABLE IF NOT EXISTS workspace_default_models (
    workspace_code TEXT NOT NULL,
    model_type TEXT NOT NULL,
    model_code TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, model_type)
);

CREATE INDEX IF NOT EXISTS idx_workspace_default_models_model_code
    ON workspace_default_models(model_code);

CREATE TABLE IF NOT EXISTS platform_default_models (
    model_type TEXT NOT NULL PRIMARY KEY,
    model_code TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_platform_default_models_model_code
    ON platform_default_models(model_code);
