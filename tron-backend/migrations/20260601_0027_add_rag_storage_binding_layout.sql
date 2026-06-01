CREATE UNIQUE INDEX IF NOT EXISTS idx_rag_memory_text_units_scope_source_ordinal
    ON rag_memory_text_units (source_code, source_version, scope_type, scope_code, ordinal);

CREATE UNIQUE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_scope_source_ordinal
    ON rag_project_doc_text_units (source_code, source_version, scope_type, scope_code, ordinal);

UPDATE rag_corpora
SET
    storage_binding_json = json_set(storage_binding_json, '$.layout', 'SPLIT'),
    updated_at = 1772294400000
WHERE corpus_id IN ('memory-card', 'project-doc', 'code-graph')
  AND json_extract(storage_binding_json, '$.layout') IS NULL;
