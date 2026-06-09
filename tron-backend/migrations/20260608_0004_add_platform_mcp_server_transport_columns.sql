PRAGMA foreign_keys = ON;

ALTER TABLE platform_mcp_servers
    ADD COLUMN transport_kind TEXT NOT NULL DEFAULT 'streamable_http';

ALTER TABLE platform_mcp_servers
    ADD COLUMN headers_variable_refs_json TEXT NOT NULL DEFAULT '{}';

ALTER TABLE platform_mcp_servers
    ADD COLUMN command TEXT NOT NULL DEFAULT '';

ALTER TABLE platform_mcp_servers
    ADD COLUMN args_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE platform_mcp_servers
    ADD COLUMN env_variable_refs_json TEXT NOT NULL DEFAULT '{}';

ALTER TABLE platform_mcp_servers
    ADD COLUMN env_passthrough_refs_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE platform_mcp_servers
    ADD COLUMN working_directory TEXT;
