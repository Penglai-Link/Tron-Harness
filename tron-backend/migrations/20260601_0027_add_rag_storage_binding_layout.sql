CREATE UNIQUE INDEX IF NOT EXISTS idx_rag_memory_text_units_scope_source_ordinal
    ON rag_memory_text_units (source_code, source_version, scope_type, scope_code, ordinal);

CREATE UNIQUE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_scope_source_ordinal
    ON rag_project_doc_text_units (source_code, source_version, scope_type, scope_code, ordinal);

-- Storage topology is now stored directly as TARGETED bindings and targets.
