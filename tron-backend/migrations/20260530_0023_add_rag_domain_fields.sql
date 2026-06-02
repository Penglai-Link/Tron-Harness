CREATE INDEX IF NOT EXISTS idx_rag_memory_sources_domain_status
    ON rag_memory_sources (memory_type, memory_status, review_status, injectable);

CREATE INDEX IF NOT EXISTS idx_rag_memory_sources_domain_rank
    ON rag_memory_sources (pin_level, importance_score, confidence_score, quality_score);

CREATE INDEX IF NOT EXISTS idx_rag_memory_sources_expires
    ON rag_memory_sources (expires_at);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_sources_repo_module
    ON rag_project_doc_sources (repo_code, module_code, document_type);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_sources_path
    ON rag_project_doc_sources (path);

UPDATE rag_corpora
SET
    filter_schema_json = '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"status"},"memoryType":{"type":"string","column":"memory_type"},"memoryStatus":{"type":"string","column":"memory_status"},"reviewStatus":{"type":"string","column":"review_status"},"version":{"type":"string","column":"version"},"supersededBy":{"type":"string","column":"superseded_by"},"conflictGroup":{"type":"string","column":"conflict_group"},"duplicateGroup":{"type":"string","column":"duplicate_group"},"injectable":{"type":"boolean","column":"injectable"},"importanceScore":{"type":"integer","column":"importance_score","operators":["eq","in","gt","gte","lt","lte"]},"confidenceScore":{"type":"integer","column":"confidence_score","operators":["eq","in","gt","gte","lt","lte"]},"qualityScore":{"type":"integer","column":"quality_score","operators":["eq","in","gt","gte","lt","lte"]},"pinLevel":{"type":"integer","column":"pin_level","operators":["eq","in","gt","gte","lt","lte"]},"expiresAt":{"type":"integer","column":"expires_at","operators":["eq","in","gt","gte","lt","lte"]},"derivedSourceType":{"type":"string","column":"derived_source_type"},"derivedSourceCode":{"type":"string","column":"derived_source_code"}}}',
    field_mapping_json = '{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","status":"status","memoryType":"memory_type","memoryStatus":"memory_status","reviewStatus":"review_status","version":"version","supersededBy":"superseded_by","conflictGroup":"conflict_group","duplicateGroup":"duplicate_group","injectable":"injectable","importanceScore":"importance_score","confidenceScore":"confidence_score","qualityScore":"quality_score","pinLevel":"pin_level","expiresAt":"expires_at","derivedSourceType":"derived_source_type","derivedSourceCode":"derived_source_code","text":"text","searchText":"search_text","embedding":"embedding"}',
    updated_at = 1772294400000
WHERE corpus_id = 'memory-card';

UPDATE rag_corpora
SET
    filter_schema_json = '{"fields":{"scopeType":{"type":"string","column":"scope_type"},"scopeCode":{"type":"string","column":"scope_code"},"sourceCode":{"type":"string","column":"source_code"},"sourceVersion":{"type":"string","column":"source_version"},"status":{"type":"string","column":"status"},"documentCode":{"type":"string","column":"document_code"},"documentType":{"type":"string","column":"document_type"},"path":{"type":"string","column":"path"},"repoCode":{"type":"string","column":"repo_code"},"moduleCode":{"type":"string","column":"module_code"},"moduleName":{"type":"string","column":"module_name"}}}',
    field_mapping_json = '{"sourceCode":"source_code","sourceVersion":"source_version","scopeType":"scope_type","scopeCode":"scope_code","status":"status","documentCode":"document_code","documentType":"document_type","path":"path","bodyHash":"body_hash","repoCode":"repo_code","moduleCode":"module_code","moduleName":"module_name","text":"text","searchText":"search_text","embedding":"embedding"}',
    updated_at = 1772294400000
WHERE corpus_id = 'project-doc';
