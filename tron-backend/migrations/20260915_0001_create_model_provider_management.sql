CREATE TABLE IF NOT EXISTS provider_connections (
    provider_code TEXT PRIMARY KEY,
    server_base_url TEXT NOT NULL,
    status TEXT NOT NULL,
    credentials_encrypted TEXT,
    authorized_at INTEGER,
    authorization_expires_at INTEGER,
    last_sync_at INTEGER,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_authorization_sessions (
    session_id TEXT PRIMARY KEY,
    provider_code TEXT NOT NULL,
    server_base_url TEXT NOT NULL,
    client_name TEXT NOT NULL,
    login_url TEXT NOT NULL,
    status TEXT NOT NULL,
    expires_at INTEGER,
    requested_by TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_provider_authorization_sessions_provider_updated
    ON provider_authorization_sessions(provider_code, updated_at DESC);

CREATE TABLE IF NOT EXISTS provider_catalog_models (
    provider_code TEXT NOT NULL,
    model_code TEXT NOT NULL,
    model_name TEXT NOT NULL,
    upstream_vendor_code TEXT,
    upstream_vendor_name TEXT,
    context_window_tokens INTEGER,
    protocols TEXT NOT NULL DEFAULT '["ChatCompletions"]',
    quality_type TEXT NOT NULL DEFAULT 'High',
    input_coefficient REAL,
    cache_hit_coefficient REAL,
    output_coefficient REAL,
    status TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(provider_code, model_code)
);

CREATE INDEX IF NOT EXISTS idx_provider_catalog_models_provider_status
    ON provider_catalog_models(provider_code, status, model_name);

CREATE TABLE IF NOT EXISTS provider_multimedia_catalog_models (
    provider_code TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_code TEXT NOT NULL,
    model_name TEXT NOT NULL,
    media_type INTEGER NOT NULL,
    status TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(provider_code, model_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_multimedia_catalog_models_provider_status
    ON provider_multimedia_catalog_models(provider_code, status, media_type, model_name);

CREATE TABLE IF NOT EXISTS provider_sync_runs (
    sync_run_code TEXT PRIMARY KEY,
    provider_code TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    model_count INTEGER,
    error_summary TEXT
);

CREATE INDEX IF NOT EXISTS idx_provider_sync_runs_provider_started
    ON provider_sync_runs(provider_code, started_at DESC);

CREATE TABLE IF NOT EXISTS default_model_selections (
    scope_key TEXT NOT NULL,
    workspace_code TEXT,
    model_type TEXT NOT NULL,
    selection_kind TEXT NOT NULL,
    provider_code TEXT,
    model_code TEXT,
    quality_type TEXT,
    route_code TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(scope_key, model_type)
);

CREATE TABLE IF NOT EXISTS provider_model_run_snapshots (
    run_code TEXT PRIMARY KEY,
    session_code TEXT,
    workspace_code TEXT NOT NULL,
    selection_kind TEXT NOT NULL,
    provider_code TEXT NOT NULL,
    requested_model_code TEXT,
    quality_type TEXT,
    route_code TEXT,
    resolved_model_code TEXT,
    resolved_model_name TEXT,
    resolved_upstream_vendor_code TEXT,
    resolved_upstream_vendor_name TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
