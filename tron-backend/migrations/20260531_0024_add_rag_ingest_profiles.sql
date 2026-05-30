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
        '{"dag":{"nodes":["validateSource","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE","inputArtifact":"request","outputArtifact":"source"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","inputArtifact":"source","outputArtifact":"textUnits"},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX","inputArtifact":"textUnits","outputArtifact":"fulltextUnits"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX","inputArtifact":"fulltextUnits","outputArtifact":"vectorUnits"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE","inputArtifact":"vectorUnits","outputArtifact":"persisted"}},"embeddingModelCode":"fake-embedding","embeddingVersion":"v1","splitter":{"type":"plainText"}}',
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
        '{"dag":{"nodes":["validateSource","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE","inputArtifact":"request","outputArtifact":"source"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","inputArtifact":"source","outputArtifact":"textUnits"},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX","inputArtifact":"textUnits","outputArtifact":"fulltextUnits"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX","inputArtifact":"fulltextUnits","outputArtifact":"vectorUnits"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE","inputArtifact":"vectorUnits","outputArtifact":"persisted"}},"embeddingModelCode":"fake-embedding","embeddingVersion":"v1","splitter":{"type":"markdownHeading"}}',
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
