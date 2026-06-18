CREATE TABLE IF NOT EXISTS registry_skills (
    skill_name TEXT PRIMARY KEY,
    source_ref TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('Marketplace', 'GitHub', 'Git', 'Url', 'Local', 'Manual')),
    registry_path TEXT NOT NULL,
    declared_version TEXT NULL,
    compatibility TEXT NULL,
    update_status TEXT NOT NULL CHECK (update_status IN ('Unknown', 'UpToDate', 'UpdateAvailable', 'Manual', 'Error')),
    managed_by TEXT NOT NULL CHECK (managed_by IN ('Cli', 'Manual')),
    last_checked_at INTEGER NULL,
    last_updated_at INTEGER NULL,
    last_error TEXT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_skill_bindings (
    workspace_code TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, skill_name)
);

CREATE INDEX IF NOT EXISTS idx_registry_skills_source_type
    ON registry_skills(source_type);

CREATE INDEX IF NOT EXISTS idx_registry_skills_managed_by
    ON registry_skills(managed_by);

CREATE INDEX IF NOT EXISTS idx_workspace_skill_bindings_workspace
    ON workspace_skill_bindings(workspace_code);

CREATE INDEX IF NOT EXISTS idx_workspace_skill_bindings_skill
    ON workspace_skill_bindings(skill_name);

CREATE TABLE IF NOT EXISTS skill_rag_index_states (
    workspace_code TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    source_version TEXT NOT NULL,
    desired_enabled INTEGER NOT NULL,
    vector_status TEXT NOT NULL CHECK (vector_status IN ('PENDING', 'INDEXED', 'DISABLED', 'WARNING', 'FAILED', 'STALE')),
    graph_status TEXT NOT NULL CHECK (graph_status IN ('PENDING', 'INDEXED', 'DISABLED', 'WARNING', 'FAILED', 'STALE')),
    content_hash TEXT NULL,
    structure_hash TEXT NULL,
    last_indexed_at INTEGER NULL,
    last_error TEXT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (workspace_code, skill_name)
);

CREATE INDEX IF NOT EXISTS idx_skill_rag_index_states_status
    ON skill_rag_index_states(workspace_code, vector_status, graph_status);
