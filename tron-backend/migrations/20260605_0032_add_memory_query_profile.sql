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
VALUES (
    'memory-query-v1',
    'RETRIEVAL',
    'enabled',
    'memory-card',
    '["memory-card"]',
    'v1',
    '{"routes":[{"routeId":"memory-query-fulltext","retriever":"FULLTEXT_SQLITE_FTS5_SIMPLE_V1"},{"routeId":"memory-query-vector","retriever":"VECTOR_SQLITE_VEC0_COSINE_V1"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":true},"hydratePolicy":{"type":"TEXT_UNIT"},"contextPolicy":{"supportedStrategies":["RESULT_LIST"]}}',
    '{}',
    'SKIP_VECTOR_ON_UNAVAILABLE',
    1780617600000,
    1780617600000
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
