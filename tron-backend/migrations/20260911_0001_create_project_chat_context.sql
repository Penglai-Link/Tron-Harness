CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    project_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'ARCHIVED')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER,
    UNIQUE (workspace_code, name)
);

CREATE TABLE IF NOT EXISTS project_app_links (
    id INTEGER PRIMARY KEY CHECK (id > 0),
    project_code TEXT NOT NULL REFERENCES projects(project_code) ON DELETE CASCADE,
    app_code TEXT NOT NULL REFERENCES apps(app_code) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    is_primary INTEGER NOT NULL CHECK (is_primary IN (0, 1)),
    UNIQUE (project_code, app_code),
    UNIQUE (project_code, position)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_project_app_links_primary
    ON project_app_links(project_code)
    WHERE is_primary = 1;

CREATE INDEX IF NOT EXISTS idx_projects_workspace_updated
    ON projects(workspace_code, status, updated_at DESC, id DESC);
