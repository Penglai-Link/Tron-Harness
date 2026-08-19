CREATE TABLE IF NOT EXISTS resource_files (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    kind TEXT NOT NULL CHECK (kind IN ('FILE', 'IMAGE', 'PDF')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    file_name TEXT NOT NULL,
    file_extension TEXT NOT NULL DEFAULT '',
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    storage_locator TEXT NOT NULL COLLATE BINARY,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code)
);

CREATE INDEX IF NOT EXISTS idx_resource_files_workspace
    ON resource_files (workspace_code);

CREATE TABLE IF NOT EXISTS resource_data_source_connections (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    kind TEXT NOT NULL CHECK (kind IN ('MYSQL_CONNECTION', 'POSTGRESQL_CONNECTION', 'REDIS_CONNECTION', 'KAFKA_CONNECTION')),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    attributes_json TEXT NOT NULL,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code)
);

CREATE INDEX IF NOT EXISTS idx_resource_data_source_connections_workspace
    ON resource_data_source_connections (workspace_code);

CREATE TABLE IF NOT EXISTS resource_servers (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port > 0 AND port <= 65535),
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code)
);

CREATE INDEX IF NOT EXISTS idx_resource_servers_workspace
    ON resource_servers (workspace_code);

CREATE TABLE IF NOT EXISTS resource_server_accounts (
    id INTEGER PRIMARY KEY,
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    server_workspace_code TEXT NOT NULL COLLATE BINARY,
    server_code TEXT NOT NULL COLLATE BINARY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    username TEXT NOT NULL,
    authentication_type TEXT NOT NULL CHECK (authentication_type IN ('Password', 'PrivateKey')),
    password_variable_key TEXT,
    private_key_variable_key TEXT,
    private_key_passphrase_variable_key TEXT,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)) DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(workspace_code, resource_code),
    FOREIGN KEY (server_workspace_code, server_code)
        REFERENCES resource_servers (workspace_code, resource_code)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_resource_server_accounts_workspace
    ON resource_server_accounts (workspace_code);
CREATE INDEX IF NOT EXISTS idx_resource_server_accounts_server
    ON resource_server_accounts (workspace_code, server_code);

CREATE TABLE IF NOT EXISTS resource_code_registry (
    workspace_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    kind TEXT NOT NULL,
    PRIMARY KEY (workspace_code, resource_code)
);

CREATE TABLE IF NOT EXISTS resource_management_migration_issues (
    scope_kind TEXT NOT NULL,
    scope_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    resource_version TEXT NOT NULL COLLATE BINARY,
    reason TEXT NOT NULL,
    PRIMARY KEY (scope_kind, scope_code, resource_code, resource_version)
);
