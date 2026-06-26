CREATE TABLE IF NOT EXISTS rag_skill_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('USER', 'AGENT', 'SPACE')),
    scope_code TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_status TEXT NOT NULL CHECK (source_status IN ('PENDING', 'ACTIVE', 'FAILED', 'DISABLED', 'ARCHIVED', 'DELETED')),
    title TEXT,
    source_content_hash TEXT,
    source_metadata TEXT NOT NULL,
    source_created_at INTEGER NOT NULL,
    source_updated_at INTEGER NOT NULL,
    skill_name TEXT NOT NULL,
    skill_description TEXT NOT NULL,
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
    UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_rag_skill_text_units_scope_search
    ON rag_skill_text_units (source_version, scope_type, scope_code, status);

CREATE INDEX IF NOT EXISTS idx_rag_skill_text_units_skill_name
    ON rag_skill_text_units (scope_type, scope_code, skill_name);

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
        'skill',
        'Skill',
        'enabled',
        'Workspace-scoped skill names and descriptions.',
        '{"fulltext":true,"vector":true,"metadataGroups":[{"name":"skill","fields":["skillName","skillDescription"]}]}',
        '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"source_status"},"sourceStatus":{"type":"string","column":"source_status"},"textUnitStatus":{"type":"string","column":"status"},"skillName":{"type":"string","column":"skill_name"}}}',
        '{"source":{"sourceCode":"source_code","sourceVersion":"source_version","sourceType":"source_type","scopeType":"scope_type","scopeCode":"scope_code","status":"source_status","sourceStatus":"source_status","title":"title","sourceContentHash":"source_content_hash","sourceMetadata":"source_metadata","skillName":"skill_name","skillDescription":"skill_description"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","textUnitStatus":"status","text":"text"}}',
        '{"mode":"TARGETED","bindings":{"source":"skill-store","textUnit":"skill-store","fulltext":"skill-store","vector":"skill-store","diagnostics":"diagnostics-store"},"targets":[{"id":"skill-store","backend":"SQLITE","layout":"TABLE","table":"rag_skill_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}',
        1780339200000,
        1780339200000
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
        'skill-ingest-v1',
        'INGEST',
        'enabled',
        'skill',
        '["skill"]',
        NULL,
        '{"pipelineVersion":"v3","sourceInputPolicy":{"supportedSourceTypes":["SKILL"],"directContentEnabled":true},"dag":{"nodes":["validateSource","preparePayload","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"preparePayload"},{"from":"preparePayload","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE"},"preparePayload":{"strategyId":"RAG_TEXT_CONTRACT_PAYLOAD_V1"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","config":{"splitter":{"type":"plainText","maxTokens":800,"overlapTokens":80}}},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE"}},"embeddingVersion":"v1"}',
        '{}',
        'SKIP_VECTOR_WITH_WARNING',
        1780339200000,
        1780339200000
    ),
    (
        'skill-related-v1',
        'RETRIEVAL',
        'enabled',
        'skill',
        '["skill"]',
        'v1',
        '{"routes":[{"routeId":"skill-fulltext","retriever":"FULLTEXT_SQLITE_FTS5_SIMPLE_V1"},{"routeId":"skill-vector","retriever":"VECTOR_SQLITE_VEC0_COSINE_V1"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":false},"hydratePolicy":{"type":"TEXT_UNIT"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]}}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1780339200000,
        1780339200000
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
