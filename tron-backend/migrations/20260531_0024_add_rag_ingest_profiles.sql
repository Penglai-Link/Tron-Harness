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
        '{"dag":{"nodes":["validateSource","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE","inputArtifact":"request","outputArtifact":"source"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","inputArtifact":"source","outputArtifact":"textUnits"},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX","inputArtifact":"textUnits","outputArtifact":"fulltextUnits"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX","inputArtifact":"fulltextUnits","outputArtifact":"vectorUnits"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE","inputArtifact":"vectorUnits","outputArtifact":"persisted"}},"payloadStrategyId":"RAG_JSON_TEXT_PAYLOAD_V1","payloadMapping":{"supportedSourceTypes":["PLAIN_TEXT"],"required":["text","version"],"title":"$.title","text":"$.text","metadataFrom":"$.metadata","fields":{"memoryType":{"path":"$.memoryType","target":"memory_type","type":"string"},"memoryStatus":{"path":"$.memoryStatus","target":"memory_status","type":"string"},"reviewStatus":{"path":"$.reviewStatus","target":"review_status","type":"string"},"version":{"path":"$.version","target":"version","type":"string"},"supersededBy":{"path":"$.supersededBy","target":"superseded_by","type":"string"},"conflictGroup":{"path":"$.conflictGroup","target":"conflict_group","type":"string"},"duplicateGroup":{"path":"$.duplicateGroup","target":"duplicate_group","type":"string"},"injectable":{"path":"$.injectable","target":"injectable","type":"boolean","omitIfMissing":true},"importanceScore":{"path":"$.importanceScore","target":"importance_score","type":"integer"},"confidenceScore":{"path":"$.confidenceScore","target":"confidence_score","type":"integer"},"qualityScore":{"path":"$.qualityScore","target":"quality_score","type":"integer"},"pinLevel":{"path":"$.pinLevel","target":"pin_level","type":"integer","omitIfMissing":true},"expiresAt":{"path":"$.expiresAt","target":"expires_at","type":"integer"},"derivedSourceType":{"path":"$.derivedSourceType","target":"derived_source_type","type":"string"},"derivedSourceCode":{"path":"$.derivedSourceCode","target":"derived_source_code","type":"string"}}},"embeddingVersion":"v1","splitter":{"type":"plainText"}}',
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
        '{"pipelineVersion":"v2","payloadStrategyId":"PROJECT_DOC_MARKDOWN_REF_PAYLOAD_V1","sourceInputPolicy":{"supportedSourceTypes":["PROJECT_DOC_REF"],"directContentEnabled":false},"payloadMapping":{"supportedSourceTypes":["PROJECT_DOC_REF"],"required":["documentUri","format"],"title":"$.title","metadataFrom":"$.metadata","fields":{"documentCode":{"path":"$.documentCode","target":"document_code","type":"string"},"documentType":{"path":"$.documentType","target":"document_type","type":"string"},"path":{"path":"$.path","target":"path","type":"string"},"repoCode":{"path":"$.repoCode","target":"repo_code","type":"string"},"moduleCode":{"path":"$.moduleCode","target":"module_code","type":"string"},"moduleName":{"path":"$.moduleName","target":"module_name","type":"string"}},"computedFields":{"bodyHash":{"source":"parsedDocument.contentText","compute":"stableHash","target":"body_hash"}}},"dag":{"nodes":["validateSource","resolveSource","parseDocument","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"resolveSource"},{"from":"resolveSource","to":"parseDocument"},{"from":"parseDocument","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"PROJECT_DOC_VALIDATE_SOURCE_V1","inputArtifact":"request","outputArtifact":"validatedSource"},"resolveSource":{"strategyId":"PROJECT_DOC_MARKDOWN_RESOLVE_SOURCE_V1","inputArtifact":"validatedSource","outputArtifact":"resolvedDocument"},"parseDocument":{"strategyId":"PROJECT_DOC_MARKDOWN_PARSE_DOCUMENT_V1","inputArtifact":"resolvedDocument","outputArtifact":"parsedDocument"},"splitText":{"strategyId":"PROJECT_DOC_MARKDOWN_SPLIT_TEXT_V1","inputArtifact":"parsedDocument","outputArtifact":"textUnits"},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX","inputArtifact":"textUnits","outputArtifact":"fulltextUnits"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX","inputArtifact":"fulltextUnits","outputArtifact":"vectorUnits"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE","inputArtifact":"vectorUnits","outputArtifact":"persisted"}},"resolverPolicy":{"allowedUriSchemes":["project-doc"],"timeoutMs":5000,"maxBytes":1048576},"parserPolicy":{"formatField":"format","supportedFormats":["MARKDOWN"],"unsupportedFormatPolicy":"FAIL_FAST"},"splitter":{"type":"markdownHeading","headingDepth":6,"maxTokens":800,"overlapTokens":80},"embeddingVersion":"v1","metadataPolicy":{"sourceMetadataFrom":"payload.metadata","recordResolverInfo":true,"recordParserInfo":true,"storeRawContentInMetadata":false}}',
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
