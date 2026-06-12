UPDATE rag_corpora
SET
    storage_binding_json = json_set(storage_binding_json, '$.backend', 'SQLITE'),
    updated_at = 1780592400000
WHERE corpus_id IN ('memory-card', 'project-doc', 'skill');

UPDATE rag_corpora
SET
    storage_binding_json = json_set(storage_binding_json, '$.backend', 'GRAPH'),
    updated_at = 1780592400000
WHERE corpus_id IN ('code-graph', 'skill-graph')
   OR json_extract(storage_binding_json, '$.layout') = 'GRAPH_SINGLE_NODE';

UPDATE rag_corpora
SET
    storage_binding_json = json_set(storage_binding_json, '$.backend', 'SQLITE'),
    updated_at = 1780592400000
WHERE json_extract(storage_binding_json, '$.backend') IS NULL;

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
        'skill-memory-project-fusion-v1',
        'RETRIEVAL',
        'enabled',
        'skill-graph',
        '["skill-graph","memory-card","project-doc"]',
        'v1',
        '{"routes":[{"routeId":"skill-graph-fulltext","corpusId":"skill-graph","retriever":"FULLTEXT_GRAPH_SINGLE_NODE_V1","topK":8,"weight":1.0},{"routeId":"skill-graph-vector","corpusId":"skill-graph","retriever":"VECTOR_GRAPH_SINGLE_NODE_V1","topK":8,"weight":1.0},{"routeId":"memory-fulltext","corpusId":"memory-card","retriever":"FULLTEXT_SQLITE_FTS5_SIMPLE_V1","topK":8,"weight":0.9},{"routeId":"memory-vector","corpusId":"memory-card","retriever":"VECTOR_SQLITE_VEC0_COSINE_V1","topK":8,"weight":0.9},{"routeId":"project-doc-fulltext","corpusId":"project-doc","retriever":"FULLTEXT_SQLITE_FTS5_SIMPLE_V1","topK":8,"weight":0.8},{"routeId":"project-doc-vector","corpusId":"project-doc","retriever":"VECTOR_SQLITE_VEC0_COSINE_V1","topK":8,"weight":0.8}],"fusionPolicy":{"type":"RRF","rankConstant":60},"dedupPolicy":{"type":"CANONICAL_TEXT_UNIT"},"topicPolicy":{"enabled":true,"type":"EMBEDDING_MMR_V1","applyStage":"BEFORE_RERANK","candidateWindow":24,"lambda":0.72,"similarityThreshold":0.82,"maxPerTopic":0,"fallback":"KEEP_ORDER"},"rerankPolicy":{"enabled":false},"hydratePolicy":{"type":"TEXT_UNIT"},"graphExpansionPolicy":{"enabled":true,"direction":"OUTGOING","maxDepth":1,"maxRelatedNodes":20,"failurePolicy":"WARN_AND_RETURN_BASE_HITS"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]}}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1780592400000,
        1780592400000
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
