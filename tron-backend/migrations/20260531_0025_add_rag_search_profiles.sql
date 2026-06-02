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
        '{"routes":[{"routeId":"memory-fulltext","retriever":"FULLTEXT_SQLITE_FTS5_SIMPLE_V1"},{"routeId":"memory-vector","retriever":"VECTOR_SQLITE_VEC0_COSINE_V1"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":true},"hydratePolicy":{"type":"TEXT_UNIT"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]}}',
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
        '{"routes":[{"routeId":"project-doc-fulltext","retriever":"FULLTEXT_SQLITE_FTS5_SIMPLE_V1"},{"routeId":"project-doc-vector","retriever":"VECTOR_SQLITE_VEC0_COSINE_V1"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":true},"hydratePolicy":{"type":"TEXT_UNIT"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]}}',
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
