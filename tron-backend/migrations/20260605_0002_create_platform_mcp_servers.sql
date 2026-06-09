PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS platform_mcp_servers (
    server_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    url TEXT NOT NULL,
    auth_kind TEXT NOT NULL,
    auth_payload_json TEXT,
    headers_json TEXT,
    enabled INTEGER NOT NULL,
    last_health_status TEXT NOT NULL CHECK (last_health_status IN ('never_checked', 'healthy', 'unhealthy')),
    last_health_message TEXT,
    last_health_checked_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_platform_mcp_servers_enabled_updated_at
    ON platform_mcp_servers (enabled, updated_at);
