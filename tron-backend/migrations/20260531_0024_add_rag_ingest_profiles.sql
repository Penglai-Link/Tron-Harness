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
        'memory-card-ingest-v1',
        'INGEST',
        'enabled',
        'memory-card',
        '["memory-card"]',
        NULL,
        '{"pipelineVersion":"v3","sourceInputPolicy":{"directContentEnabled":true},"dag":{"nodes":["validateSource","preparePayload","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"preparePayload"},{"from":"preparePayload","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE"},"preparePayload":{"strategyId":"RAG_TEXT_CONTRACT_PAYLOAD_V1"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","config":{"splitter":{"type":"plainText","maxTokens":800,"overlapTokens":80}}},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE"}},"embeddingVersion":"v1"}',
        '{}',
        'FAIL_FAST',
        1772380800000,
        1772380800000
    ),
    (
        'project-doc-ingest-v1',
        'INGEST',
        'enabled',
        'project-doc',
        '["project-doc"]',
        NULL,
        '{"pipelineVersion":"v3","sourceInputPolicy":{"directContentEnabled":false},"dag":{"nodes":["validateSource","preparePayload","resolveSource","parseDocument","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"preparePayload"},{"from":"preparePayload","to":"resolveSource"},{"from":"resolveSource","to":"parseDocument"},{"from":"parseDocument","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE"},"preparePayload":{"strategyId":"RAG_DOCUMENT_REF_CONTRACT_PAYLOAD_V1"},"resolveSource":{"strategyId":"PROJECT_DOC_RESOLVE_SOURCE","config":{"allowedUriSchemes":["project-doc"],"timeoutMs":5000,"maxBytes":1048576}},"parseDocument":{"strategyId":"RAG_PARSE_MARKDOWN_DOCUMENT","config":{"format":"MARKDOWN","unsupportedFormatPolicy":"FAIL_FAST"}},"splitText":{"strategyId":"RAG_SPLIT_TEXT","config":{"splitter":{"type":"markdownHeading","headingDepth":6,"maxTokens":800,"overlapTokens":80}}},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE"}},"embeddingVersion":"v1"}',
        '{}',
        'SKIP_VECTOR_WITH_WARNING',
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
