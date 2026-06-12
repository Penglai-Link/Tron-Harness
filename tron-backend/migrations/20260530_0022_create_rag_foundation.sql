CREATE TABLE IF NOT EXISTS rag_corpora (
    corpus_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('enabled', 'reserved', 'disabled')),
    description TEXT,
    capabilities_json TEXT NOT NULL,
    filter_schema_json TEXT NOT NULL,
    field_mapping_json TEXT NOT NULL,
    storage_binding_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rag_profiles (
    profile_id TEXT PRIMARY KEY,
    profile_type TEXT NOT NULL CHECK (profile_type IN ('INGEST', 'RETRIEVAL')),
    status TEXT NOT NULL CHECK (status IN ('enabled', 'reserved', 'disabled')),
    primary_corpus_id TEXT REFERENCES rag_corpora(corpus_id),
    corpus_ids_json TEXT NOT NULL,
    required_source_version TEXT,
    config_json TEXT NOT NULL,
    filters_json TEXT NOT NULL,
    degrade_policy TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rag_profiles_type_status
    ON rag_profiles (profile_type, status);

CREATE INDEX IF NOT EXISTS idx_rag_profiles_primary_corpus
    ON rag_profiles (primary_corpus_id, profile_type, status);

CREATE TABLE IF NOT EXISTS rag_memory_sources (
    id INTEGER PRIMARY KEY,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    source_type TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('USER', 'AGENT', 'SPACE')),
    scope_code TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('PENDING', 'ACTIVE', 'FAILED', 'DISABLED', 'ARCHIVED', 'DELETED')),
    title TEXT,
    content_hash TEXT,
    metadata TEXT NOT NULL,
    memory_type TEXT,
    memory_status TEXT,
    review_status TEXT,
    version TEXT,
    superseded_by TEXT,
    conflict_group TEXT,
    duplicate_group TEXT,
    injectable INTEGER NOT NULL DEFAULT 1,
    importance_score INTEGER,
    confidence_score INTEGER,
    quality_score INTEGER,
    pin_level INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER,
    derived_source_type TEXT,
    derived_source_code TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (source_code, source_version, scope_type, scope_code)
);

CREATE INDEX IF NOT EXISTS idx_rag_memory_sources_scope_status
    ON rag_memory_sources (scope_type, scope_code, status);

CREATE INDEX IF NOT EXISTS idx_rag_memory_sources_code_version
    ON rag_memory_sources (source_code, source_version);

CREATE TABLE IF NOT EXISTS rag_project_doc_sources (
    id INTEGER PRIMARY KEY,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    source_type TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('USER', 'AGENT', 'SPACE')),
    scope_code TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('PENDING', 'ACTIVE', 'FAILED', 'DISABLED', 'ARCHIVED', 'DELETED')),
    title TEXT,
    content_hash TEXT,
    document_code TEXT,
    document_type TEXT,
    path TEXT,
    repo_code TEXT,
    module_code TEXT,
    module_name TEXT,
    body_hash TEXT,
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (source_code, source_version, scope_type, scope_code)
);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_sources_scope_status
    ON rag_project_doc_sources (scope_type, scope_code, status);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_sources_code_version
    ON rag_project_doc_sources (source_code, source_version);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_sources_document_code
    ON rag_project_doc_sources (document_code);

CREATE TABLE IF NOT EXISTS rag_memory_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_code TEXT NOT NULL,
    unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
    text TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    token_count INTEGER,
    content_hash TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    source_type TEXT NOT NULL DEFAULT 'MEMORY_CARD',
    source_status TEXT NOT NULL DEFAULT 'PENDING',
    title TEXT,
    source_content_hash TEXT,
    source_metadata TEXT NOT NULL DEFAULT '{}',
    source_created_at INTEGER NOT NULL DEFAULT 0,
    source_updated_at INTEGER NOT NULL DEFAULT 0,
    memory_type TEXT,
    memory_status TEXT,
    review_status TEXT,
    version TEXT,
    superseded_by TEXT,
    conflict_group TEXT,
    duplicate_group TEXT,
    injectable INTEGER NOT NULL DEFAULT 1,
    importance_score INTEGER,
    confidence_score INTEGER,
    quality_score INTEGER,
    pin_level INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER,
    derived_source_type TEXT,
    derived_source_code TEXT,
    UNIQUE (source_code, source_version, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_source
    ON rag_memory_text_units (source_code, source_version, ordinal);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_scope_status
    ON rag_memory_text_units (scope_type, scope_code, status);

CREATE TABLE IF NOT EXISTS rag_project_doc_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    document_code TEXT,
    scope_type TEXT NOT NULL,
    scope_code TEXT NOT NULL,
    unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
    text TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    token_count INTEGER,
    content_hash TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (source_code, source_version, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_source
    ON rag_project_doc_text_units (source_code, source_version, ordinal);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_scope_status
    ON rag_project_doc_text_units (scope_type, scope_code, status);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_document
    ON rag_project_doc_text_units (document_code);

CREATE TABLE IF NOT EXISTS rag_diagnostics (
    id INTEGER PRIMARY KEY,
    record_type TEXT NOT NULL CHECK (record_type IN ('INDEXING_WARNING', 'SEARCH_TRACE')),
    record_code TEXT NOT NULL UNIQUE,
    corpus_id TEXT,
    profile_id TEXT,
    source_code TEXT,
    source_version TEXT,
    scope_type TEXT,
    scope_code TEXT,
    workspace_code TEXT,
    user_code TEXT,
    agent_code TEXT,
    stage_id TEXT,
    query_text TEXT,
    message TEXT,
    payload_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    CHECK (
        record_type <> 'SEARCH_TRACE'
        OR (
            profile_id IS NOT NULL
            AND workspace_code IS NOT NULL
            AND query_text IS NOT NULL
            AND expires_at IS NOT NULL
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_rag_diagnostics_type_created
    ON rag_diagnostics (record_type, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_rag_diagnostics_source
    ON rag_diagnostics (record_type, corpus_id, source_code, source_version);

CREATE INDEX IF NOT EXISTS idx_rag_diagnostics_workspace
    ON rag_diagnostics (record_type, workspace_code, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_rag_diagnostics_expires
    ON rag_diagnostics (record_type, expires_at);

INSERT INTO rag_corpora (
    corpus_id,
    display_name,
    status,
    description,
    capabilities_json,
    filter_schema_json,
    field_mapping_json,
    storage_binding_json,
    created_at,
    updated_at
)
VALUES
    (
        'memory-card',
        'Memory Card',
        'enabled',
        'User, agent, and space scoped memory cards.',
        '{"fulltext":true,"vector":true,"reserved":false,"metadataGroups":[{"name":"memory","fields":["memoryType","memoryStatus","reviewStatus","version","supersededBy","conflictGroup","duplicateGroup","injectable","importanceScore","confidenceScore","qualityScore","pinLevel","expiresAt","derivedSourceType","derivedSourceCode"]}]}',
        '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"status":{"type":"string","column":"status"}}}',
        '{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","text":"text"}',
        '{"mode":"TARGETED","bindings":{"source":"memory-source-store","textUnit":"memory-text-store","fulltext":"memory-text-store","vector":"memory-text-store","diagnostics":"diagnostics-store"},"targets":[{"id":"memory-source-store","backend":"SQLITE","layout":"TABLE","table":"rag_memory_sources"},{"id":"memory-text-store","backend":"SQLITE","layout":"TABLE","table":"rag_memory_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}',
        1772294400000,
        1772294400000
    ),
    (
        'project-doc',
        'Project Document',
        'enabled',
        'Project documentation sources and text units.',
        '{"fulltext":true,"vector":true,"reserved":false,"metadataGroups":[{"name":"document","fields":["documentCode","documentType","path","repoCode","moduleCode","moduleName"]}]}',
        '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"documentCode":{"type":"string","column":"document_code"},"status":{"type":"string","column":"status"}}}',
        '{"source":{"sourceCode":"source_code","sourceVersion":"source_version","sourceType":"source_type","scopeType":"scope_type","scopeCode":"scope_code","status":"status","title":"title","contentHash":"content_hash","metadata":"metadata","documentCode":"document_code"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","documentCode":"document_code","text":"text"}}',
        '{"mode":"TARGETED","bindings":{"source":"project-doc-source-store","textUnit":"project-doc-text-store","fulltext":"project-doc-text-store","vector":"project-doc-text-store","diagnostics":"diagnostics-store"},"targets":[{"id":"project-doc-source-store","backend":"SQLITE","layout":"TABLE","table":"rag_project_doc_sources"},{"id":"project-doc-text-store","backend":"SQLITE","layout":"TABLE","table":"rag_project_doc_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}',
        1772294400000,
        1772294400000
    ),
    (
        'code-graph',
        'Code Graph',
        'reserved',
        'Reserved Corpus for future code graph retrieval.',
        '{"fulltext":false,"vector":false,"reserved":true}',
        '{"fields":{}}',
        '{}',
        '{"mode":"TARGETED","bindings":{"diagnostics":"diagnostics-store"},"targets":[{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}',
        1772294400000,
        1772294400000
    )
ON CONFLICT (corpus_id) DO UPDATE SET
    display_name = excluded.display_name,
    status = excluded.status,
    description = excluded.description,
    capabilities_json = excluded.capabilities_json,
    filter_schema_json = excluded.filter_schema_json,
    field_mapping_json = excluded.field_mapping_json,
    storage_binding_json = excluded.storage_binding_json,
    updated_at = excluded.updated_at;

INSERT INTO rag_profiles (
    profile_id,
    profile_type,
    status,
    primary_corpus_id,
    corpus_ids_json,
    required_source_version,
    config_json,
    filters_json,
    degrade_policy,
    created_at,
    updated_at
)
VALUES
    (
        'memory-related-v1',
        'RETRIEVAL',
        'enabled',
        'memory-card',
        '["memory-card"]',
        'v1',
        '{"retrievers":["FULLTEXT_SQLITE_FTS5_SIMPLE_V1","VECTOR_SQLITE_VEC0_COSINE_V1"],"policies":["SKIP_VECTOR_ON_UNAVAILABLE","DEDUP_BY_SOURCE","HYDRATE_TEXT_UNIT","CONTEXT_PACK_BASIC"]}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1772294400000,
        1772294400000
    ),
    (
        'project-doc-related-v1',
        'RETRIEVAL',
        'enabled',
        'project-doc',
        '["project-doc"]',
        'v1',
        '{"retrievers":["FULLTEXT_SQLITE_FTS5_SIMPLE_V1","VECTOR_SQLITE_VEC0_COSINE_V1"],"policies":["SKIP_VECTOR_ON_UNAVAILABLE","DEDUP_BY_SOURCE","HYDRATE_TEXT_UNIT","CONTEXT_PACK_BASIC"]}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1772294400000,
        1772294400000
    ),
    (
        'code-graph-reserved-v1',
        'RETRIEVAL',
        'reserved',
        'code-graph',
        '["code-graph"]',
        'v1',
        '{"retrievers":["GRAPH"],"policies":["SUMMARY_TREE"]}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1772294400000,
        1772294400000
    )
ON CONFLICT (profile_id) DO UPDATE SET
    profile_type = excluded.profile_type,
    status = excluded.status,
    primary_corpus_id = excluded.primary_corpus_id,
    corpus_ids_json = excluded.corpus_ids_json,
    required_source_version = excluded.required_source_version,
    config_json = excluded.config_json,
    filters_json = excluded.filters_json,
    degrade_policy = excluded.degrade_policy,
    updated_at = excluded.updated_at;
