UPDATE rag_memory_text_units
SET
    source_type = COALESCE((
        SELECT source_type
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), source_type),
    source_status = COALESCE((
        SELECT status
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), source_status),
    title = COALESCE((
        SELECT title
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), title),
    source_content_hash = COALESCE((
        SELECT content_hash
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), source_content_hash),
    source_metadata = COALESCE((
        SELECT metadata
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), source_metadata),
    source_created_at = COALESCE((
        SELECT created_at
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), source_created_at),
    source_updated_at = COALESCE((
        SELECT updated_at
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), source_updated_at),
    memory_type = COALESCE((
        SELECT memory_type
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), memory_type),
    memory_status = COALESCE((
        SELECT memory_status
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), memory_status),
    review_status = COALESCE((
        SELECT review_status
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), review_status),
    version = COALESCE((
        SELECT version
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), version),
    superseded_by = COALESCE((
        SELECT superseded_by
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), superseded_by),
    conflict_group = COALESCE((
        SELECT conflict_group
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), conflict_group),
    duplicate_group = COALESCE((
        SELECT duplicate_group
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), duplicate_group),
    injectable = COALESCE((
        SELECT injectable
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), injectable),
    importance_score = COALESCE((
        SELECT importance_score
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), importance_score),
    confidence_score = COALESCE((
        SELECT confidence_score
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), confidence_score),
    quality_score = COALESCE((
        SELECT quality_score
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), quality_score),
    pin_level = COALESCE((
        SELECT pin_level
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), pin_level),
    expires_at = COALESCE((
        SELECT expires_at
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), expires_at),
    derived_source_type = COALESCE((
        SELECT derived_source_type
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), derived_source_type),
    derived_source_code = COALESCE((
        SELECT derived_source_code
        FROM rag_memory_sources src
        WHERE src.source_code = rag_memory_text_units.source_code
          AND src.source_version = rag_memory_text_units.source_version
          AND src.scope_type = rag_memory_text_units.scope_type
          AND src.scope_code = rag_memory_text_units.scope_code
    ), derived_source_code)
WHERE EXISTS (
    SELECT 1
    FROM rag_memory_sources src
    WHERE src.source_code = rag_memory_text_units.source_code
      AND src.source_version = rag_memory_text_units.source_version
      AND src.scope_type = rag_memory_text_units.scope_type
      AND src.scope_code = rag_memory_text_units.scope_code
);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_source_status
    ON rag_memory_text_units (scope_type, scope_code, source_status);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_domain_status
    ON rag_memory_text_units (memory_type, memory_status, review_status, injectable);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_domain_rank
    ON rag_memory_text_units (pin_level, importance_score, confidence_score, quality_score);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_expires
    ON rag_memory_text_units (expires_at);

UPDATE rag_corpora
SET
    filter_schema_json = '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"source_status"},"sourceStatus":{"type":"string","column":"source_status"},"textUnitStatus":{"type":"string","column":"status"},"memoryType":{"type":"string","column":"memory_type"},"memoryStatus":{"type":"string","column":"memory_status"},"reviewStatus":{"type":"string","column":"review_status"},"version":{"type":"string","column":"version"},"supersededBy":{"type":"string","column":"superseded_by"},"conflictGroup":{"type":"string","column":"conflict_group"},"duplicateGroup":{"type":"string","column":"duplicate_group"},"injectable":{"type":"boolean","column":"injectable"},"importanceScore":{"type":"integer","column":"importance_score","operators":["eq","in","gt","gte","lt","lte"]},"confidenceScore":{"type":"integer","column":"confidence_score","operators":["eq","in","gt","gte","lt","lte"]},"qualityScore":{"type":"integer","column":"quality_score","operators":["eq","in","gt","gte","lt","lte"]},"pinLevel":{"type":"integer","column":"pin_level","operators":["eq","in","gt","gte","lt","lte"]},"expiresAt":{"type":"integer","column":"expires_at","operators":["eq","in","gt","gte","lt","lte"]},"derivedSourceType":{"type":"string","column":"derived_source_type"},"derivedSourceCode":{"type":"string","column":"derived_source_code"}}}',
    field_mapping_json = '{"source":{"sourceCode":"source_code","sourceVersion":"source_version","sourceType":"source_type","scopeType":"scope_type","scopeCode":"scope_code","status":"source_status","sourceStatus":"source_status","title":"title","sourceContentHash":"source_content_hash","sourceMetadata":"source_metadata","memoryType":"memory_type","memoryStatus":"memory_status","reviewStatus":"review_status","version":"version","supersededBy":"superseded_by","conflictGroup":"conflict_group","duplicateGroup":"duplicate_group","injectable":"injectable","importanceScore":"importance_score","confidenceScore":"confidence_score","qualityScore":"quality_score","pinLevel":"pin_level","expiresAt":"expires_at","derivedSourceType":"derived_source_type","derivedSourceCode":"derived_source_code"},"textUnit":{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","textUnitStatus":"status","text":"text"}}',
    storage_binding_json = '{"layout":"EMBEDDED_SOURCE","table":"rag_memory_text_units","diagnosticsTable":"rag_diagnostics"}',
    updated_at = 1772294400000
WHERE corpus_id = 'memory-card';

DROP TABLE IF EXISTS rag_memory_sources;
