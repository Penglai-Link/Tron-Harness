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
        '{"dag":{"nodes":["validateSource","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE","inputArtifact":"request","outputArtifact":"source"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","inputArtifact":"source","outputArtifact":"textUnits"},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX","inputArtifact":"textUnits","outputArtifact":"fulltextUnits"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX","inputArtifact":"fulltextUnits","outputArtifact":"vectorUnits"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE","inputArtifact":"vectorUnits","outputArtifact":"persisted"}},"payloadStrategyId":"MEMORY_PLAIN_TEXT_PAYLOAD_V1","embeddingModelCode":"fake-embedding","embeddingVersion":"v1","splitter":{"type":"plainText"}}',
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
        '{"pipelineVersion":"v2","payloadStrategyId":"PROJECT_DOC_MARKDOWN_REF_PAYLOAD_V1","sourceInputPolicy":{"supportedSourceTypes":["PROJECT_DOC_REF"],"directContentEnabled":false},"dag":{"nodes":["validateSource","resolveSource","parseDocument","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"resolveSource"},{"from":"resolveSource","to":"parseDocument"},{"from":"parseDocument","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"PROJECT_DOC_VALIDATE_SOURCE_V1","inputArtifact":"request","outputArtifact":"validatedSource"},"resolveSource":{"strategyId":"PROJECT_DOC_MARKDOWN_RESOLVE_SOURCE_V1","inputArtifact":"validatedSource","outputArtifact":"resolvedDocument"},"parseDocument":{"strategyId":"PROJECT_DOC_MARKDOWN_PARSE_DOCUMENT_V1","inputArtifact":"resolvedDocument","outputArtifact":"parsedDocument"},"splitText":{"strategyId":"PROJECT_DOC_MARKDOWN_SPLIT_TEXT_V1","inputArtifact":"parsedDocument","outputArtifact":"textUnits"},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX","inputArtifact":"textUnits","outputArtifact":"fulltextUnits"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX","inputArtifact":"fulltextUnits","outputArtifact":"vectorUnits"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE","inputArtifact":"vectorUnits","outputArtifact":"persisted"}},"resolverPolicy":{"allowedUriSchemes":["project-doc"],"timeoutMs":5000,"maxBytes":1048576},"parserPolicy":{"formatField":"format","supportedFormats":["MARKDOWN"],"unsupportedFormatPolicy":"FAIL_FAST"},"splitter":{"type":"markdownHeading","maxTokens":800,"overlapTokens":80},"embeddingVersion":"v1","metadataPolicy":{"sourceMetadataFrom":"payload.metadata","recordResolverInfo":true,"recordParserInfo":true,"storeRawContentInMetadata":false}}',
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
