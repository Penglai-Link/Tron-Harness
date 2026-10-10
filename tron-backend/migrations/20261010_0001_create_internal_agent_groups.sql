-- Empty scope is platform-owned; workspace codes isolate local catalogs.
CREATE TABLE IF NOT EXISTS internal_agent_groups (
    scope_code TEXT NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 60),
    PRIMARY KEY (scope_code, name)
);
