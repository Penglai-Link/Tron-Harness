CREATE TABLE IF NOT EXISTS apps (
    id INTEGER PRIMARY KEY,
    app_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    app_name TEXT NOT NULL,
    description TEXT NULL,
    local_directory TEXT NOT NULL,
    runtime_role TEXT NOT NULL CHECK (runtime_role IN ('frontend', 'backend')),
    repository_url TEXT NULL,
    default_branch TEXT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER NULL
);

CREATE TABLE IF NOT EXISTS app_tags (
    id INTEGER PRIMARY KEY,
    app_code TEXT NOT NULL REFERENCES apps(app_code),
    tag_name TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    UNIQUE (app_code, tag_name)
);

CREATE INDEX IF NOT EXISTS idx_apps_workspace_status ON apps(workspace_code, status);
CREATE INDEX IF NOT EXISTS idx_apps_workspace_name ON apps(workspace_code, app_name);
CREATE INDEX IF NOT EXISTS idx_app_tags_app_code ON app_tags(app_code);
