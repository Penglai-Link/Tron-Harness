-- Fresh unified RAG schema. No legacy migrations or data conversion.

CREATE TABLE rag_schema_version (version INTEGER PRIMARY KEY CHECK(version=2));

INSERT INTO rag_schema_version VALUES (2);

CREATE TABLE rag_corpora (
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

CREATE TABLE rag_memory_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_code TEXT NOT NULL,
    search_text TEXT NOT NULL DEFAULT '',
    fulltext_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    embedding_model_code TEXT,
    embedding_version TEXT,
    embedding_text_hash TEXT,
    embedding_dimension INTEGER,
    embedding BLOB,
    embedding_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
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
    derived_source_code TEXT, "body_hash" TEXT,
    UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
);

CREATE TABLE rag_skill_text_units (
    id INTEGER PRIMARY KEY,
    text_unit_code TEXT NOT NULL UNIQUE,
    source_code TEXT NOT NULL,
    source_version TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('USER', 'AGENT', 'SPACE')),
    scope_code TEXT NOT NULL,
    source_status TEXT NOT NULL CHECK (source_status IN ('PENDING', 'ACTIVE', 'FAILED', 'DISABLED', 'ARCHIVED', 'DELETED')),
    title TEXT,
    source_content_hash TEXT,
    source_metadata TEXT NOT NULL,
    source_created_at INTEGER NOT NULL,
    source_updated_at INTEGER NOT NULL,
    skill_name TEXT NOT NULL,
    skill_description TEXT NOT NULL,
    search_text TEXT NOT NULL DEFAULT '',
    fulltext_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    embedding_model_code TEXT,
    embedding_version TEXT,
    embedding_text_hash TEXT,
    embedding_dimension INTEGER,
    embedding BLOB,
    embedding_status TEXT NOT NULL DEFAULT 'NOT_ENABLED',
    unit_type TEXT NOT NULL CHECK (unit_type IN ('LEAF', 'SUMMARY', 'SYMBOL', 'NOTE')),
    text TEXT NOT NULL,
    heading_path TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    token_count INTEGER,
    content_hash TEXT,
    status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'DISABLED', 'DELETED')),
    metadata TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL, "body_hash" TEXT,
    UNIQUE (source_code, source_version, scope_type, scope_code, ordinal)
);

CREATE TABLE rag_diagnostics (
    id INTEGER PRIMARY KEY,
    record_type TEXT NOT NULL CHECK (record_type IN ('INDEXING_WARNING', 'SEARCH_TRACE')),
    record_code TEXT NOT NULL UNIQUE,
    corpus_id TEXT,
    task_id TEXT,
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
    expires_at INTEGER, "details_json" TEXT, "stage" TEXT, "status" TEXT, "updated_at" TEXT,
    CHECK (
        record_type <> 'SEARCH_TRACE'
        OR (
            task_id IS NOT NULL
            AND workspace_code IS NOT NULL
            AND query_text IS NOT NULL
            AND expires_at IS NOT NULL
        )
    )
);

INSERT INTO rag_corpora (corpus_id,display_name,status,description,capabilities_json,filter_schema_json,field_mapping_json,storage_binding_json,created_at,updated_at) VALUES ('memory-card','记忆','enabled','User, agent, and space scoped memory cards.','{"fulltext":true,"vector":true,"metadataGroups":[{"name":"memory","fields":["memoryType","memoryStatus","reviewStatus","version","supersededBy","conflictGroup","duplicateGroup","injectable","importanceScore","confidenceScore","qualityScore","pinLevel","expiresAt","derivedSourceType","derivedSourceCode"]}],"settings":{"fulltext":true,"vector":true,"graph":false,"embeddingModelCode":null,"rerankModelCode":null,"graphModelCode":null},"graph":false}','{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"source_status"},"sourceStatus":{"type":"string","column":"source_status"},"textUnitStatus":{"type":"string","column":"status"},"memoryType":{"type":"string","column":"memory_type"},"memoryStatus":{"type":"string","column":"memory_status"},"reviewStatus":{"type":"string","column":"review_status"},"version":{"type":"string","column":"version"},"supersededBy":{"type":"string","column":"superseded_by"},"conflictGroup":{"type":"string","column":"conflict_group"},"duplicateGroup":{"type":"string","column":"duplicate_group"},"injectable":{"type":"boolean","column":"injectable"},"importanceScore":{"type":"integer","column":"importance_score","operators":["eq","in","gt","gte","lt","lte"]},"confidenceScore":{"type":"integer","column":"confidence_score","operators":["eq","in","gt","gte","lt","lte"]},"qualityScore":{"type":"integer","column":"quality_score","operators":["eq","in","gt","gte","lt","lte"]},"pinLevel":{"type":"integer","column":"pin_level","operators":["eq","in","gt","gte","lt","lte"]},"expiresAt":{"type":"integer","column":"expires_at","operators":["eq","in","gt","gte","lt","lte","gtOrNull"]},"derivedSourceType":{"type":"string","column":"derived_source_type"},"derivedSourceCode":{"type":"string","column":"derived_source_code"}}}','{"source":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","status":"source_status","sourceStatus":"source_status","title":"title","sourceContentHash":"source_content_hash","sourceMetadata":"source_metadata","memoryType":"memory_type","memoryStatus":"memory_status","reviewStatus":"review_status","version":"version","supersededBy":"superseded_by","conflictGroup":"conflict_group","duplicateGroup":"duplicate_group","injectable":"injectable","importanceScore":"importance_score","confidenceScore":"confidence_score","qualityScore":"quality_score","pinLevel":"pin_level","expiresAt":"expires_at","derivedSourceType":"derived_source_type","derivedSourceCode":"derived_source_code","sourceCreatedAt":"source_created_at","sourceUpdatedAt":"source_updated_at"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","textUnitStatus":"status","text":"text"}}','{"mode":"TARGETED","bindings":{"source":"memory-store","textUnit":"memory-store","fulltext":"memory-store","vector":"memory-store","diagnostics":"diagnostics-store"},"targets":[{"id":"memory-store","backend":"SQLITE","layout":"TABLE","table":"rag_memory_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}',1772294400000,1781568000000);

INSERT INTO rag_corpora (corpus_id,display_name,status,description,capabilities_json,filter_schema_json,field_mapping_json,storage_binding_json,created_at,updated_at) VALUES ('skill','技能','enabled','Workspace-scoped skill content and source-linked graph evidence.','{"fulltext":true,"vector":true,"metadataGroups":[{"name":"skill","fields":["skillName","skillDescription"]}],"settings":{"fulltext":true,"vector":true,"graph":true,"embeddingModelCode":null,"rerankModelCode":null,"graphModelCode":null},"graph":true}','{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"source_status"},"sourceStatus":{"type":"string","column":"source_status"},"textUnitStatus":{"type":"string","column":"status"},"skillName":{"type":"string","column":"skill_name"}}}','{"source":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","status":"source_status","sourceStatus":"source_status","title":"title","sourceContentHash":"source_content_hash","sourceMetadata":"source_metadata","skillName":"skill_name","skillDescription":"skill_description"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","textUnitStatus":"status","text":"text"}}','{"mode":"TARGETED","bindings":{"source":"skill-store","textUnit":"skill-store","fulltext":"skill-store","vector":"skill-store","diagnostics":"diagnostics-store"},"targets":[{"id":"skill-store","backend":"SQLITE","layout":"TABLE","table":"rag_skill_text_units"},{"id":"diagnostics-store","backend":"SQLITE","layout":"TABLE","table":"rag_diagnostics"}]}',1780339200000,1780339200000);
