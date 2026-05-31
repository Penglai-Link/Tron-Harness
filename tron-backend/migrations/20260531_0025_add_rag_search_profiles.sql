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
        '{"routes":[{"routeId":"memory-fulltext","retriever":"FULLTEXT"},{"routeId":"memory-vector","retriever":"VECTOR"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":true},"hydratePolicy":{"type":"TEXT_UNIT"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]},"embeddingModelCode":"fake-embedding","rerankModelCode":"fake-rerank"}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1772380800000,
        1772380800000
    ),
    (
        'project-doc-related-v1',
        'RETRIEVAL',
        'enabled',
        'project-doc',
        '["project-doc"]',
        'v1',
        '{"routes":[{"routeId":"project-doc-fulltext","retriever":"FULLTEXT"},{"routeId":"project-doc-vector","retriever":"VECTOR"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":true},"hydratePolicy":{"type":"TEXT_UNIT"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]},"embeddingModelCode":"fake-embedding","rerankModelCode":"fake-rerank"}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1772380800000,
        1772380800000
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

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_search_scope
    ON rag_memory_text_units (source_version, scope_type, scope_code, status, fulltext_status);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_search_scope
    ON rag_project_doc_text_units (source_version, scope_type, scope_code, status, fulltext_status);

CREATE INDEX IF NOT EXISTS idx_rag_memory_text_units_vector
    ON rag_memory_text_units (embedding_model_code, embedding_version, embedding_status);

CREATE INDEX IF NOT EXISTS idx_rag_project_doc_text_units_vector
    ON rag_project_doc_text_units (embedding_model_code, embedding_version, embedding_status);
