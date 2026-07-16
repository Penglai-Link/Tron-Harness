CREATE TABLE IF NOT EXISTS resource_definitions (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('PLATFORM', 'WORKSPACE')),
    scope_code TEXT NOT NULL COLLATE BINARY,
    definition_code TEXT NOT NULL COLLATE BINARY,
    definition_version TEXT NOT NULL COLLATE BINARY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    definition_status TEXT NOT NULL CHECK (definition_status IN ('ENABLED', 'DISABLED')),
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)),
    attributes_schema TEXT NOT NULL,
    binary_schema TEXT NOT NULL,
    binary_storage_profile TEXT NOT NULL,
    draft_revision INTEGER NOT NULL,
    validation_status TEXT NOT NULL CHECK (validation_status IN ('VALID', 'INVALID')),
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN ('DRAFT', 'ACTIVE', 'HISTORICAL')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    published_at INTEGER,
    UNIQUE(scope_kind, scope_code, definition_code, definition_version)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_resource_definitions_one_draft
    ON resource_definitions (scope_kind, scope_code, definition_code)
    WHERE lifecycle_status = 'DRAFT';

CREATE UNIQUE INDEX IF NOT EXISTS idx_resource_definitions_one_active
    ON resource_definitions (scope_kind, scope_code, definition_code)
    WHERE lifecycle_status = 'ACTIVE';

CREATE INDEX IF NOT EXISTS idx_resource_definitions_resolution
    ON resource_definitions (scope_kind, scope_code, definition_code, lifecycle_status);

CREATE TABLE IF NOT EXISTS resources (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('PLATFORM', 'WORKSPACE')),
    scope_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    resource_version TEXT NOT NULL COLLATE BINARY,
    definition_scope_kind TEXT NOT NULL CHECK (definition_scope_kind IN ('PLATFORM', 'WORKSPACE')),
    definition_scope_code TEXT NOT NULL COLLATE BINARY,
    definition_code TEXT NOT NULL COLLATE BINARY,
    definition_version TEXT NOT NULL COLLATE BINARY,
    based_on_resource_version TEXT COLLATE BINARY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    attributes_json TEXT NOT NULL,
    allow_inheritance INTEGER NOT NULL CHECK (allow_inheritance IN (0, 1)),
    draft_revision INTEGER NOT NULL,
    validation_status TEXT NOT NULL CHECK (validation_status IN ('VALID', 'INVALID')),
    lifecycle_status TEXT NOT NULL CHECK (lifecycle_status IN ('DRAFT', 'ACTIVE', 'HISTORICAL')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    published_at INTEGER,
    UNIQUE(scope_kind, scope_code, resource_code, resource_version)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_resources_one_draft
    ON resources (scope_kind, scope_code, resource_code)
    WHERE lifecycle_status = 'DRAFT';

CREATE UNIQUE INDEX IF NOT EXISTS idx_resources_one_active
    ON resources (scope_kind, scope_code, resource_code)
    WHERE lifecycle_status = 'ACTIVE';

CREATE INDEX IF NOT EXISTS idx_resources_resolution
    ON resources (scope_kind, scope_code, resource_code, lifecycle_status);

CREATE TABLE IF NOT EXISTS resource_binary_mappings (
    id INTEGER PRIMARY KEY,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('PLATFORM', 'WORKSPACE')),
    scope_code TEXT NOT NULL COLLATE BINARY,
    resource_code TEXT NOT NULL COLLATE BINARY,
    resource_version TEXT NOT NULL COLLATE BINARY,
    binary_name TEXT NOT NULL COLLATE BINARY,
    storage_locator TEXT NOT NULL COLLATE BINARY,
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    created_at INTEGER NOT NULL,
    UNIQUE(scope_kind, scope_code, resource_code, resource_version, binary_name)
);

CREATE INDEX IF NOT EXISTS idx_resource_binary_mappings_locator
    ON resource_binary_mappings (storage_locator);
