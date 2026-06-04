INSERT INTO rag_corpora (
    corpus_id,
    display_name,
    status,
    description,
    capabilities_json,
    filter_schema_json,
    field_mapping_json,
    storage_binding_json,
    created_at,
    updated_at
)
VALUES
    (
        'skill-graph',
        'Skill Graph',
        'enabled',
        'Graph-backed workspace-scoped skill nodes and related graph data.',
        '{"fulltext":true,"vector":true,"graph":true,"reserved":false,"metadataGroups":[{"name":"skill","fields":["skillName","skillDescription"]},{"name":"graph","fields":["relatedGraph"]}]}',
        '{"fields":{"scopeType":{"type":"string","column":"scopeType"},"scopeCode":{"type":"string","column":"scopeCode"},"sourceCode":{"type":"string","column":"sourceCode"},"sourceVersion":{"type":"string","column":"sourceVersion"},"status":{"type":"string","column":"sourceStatus"},"sourceStatus":{"type":"string","column":"sourceStatus"},"textUnitStatus":{"type":"string","column":"textUnitStatus"},"skillName":{"type":"string","column":"skillName"}}}',
        '{"source":{"sourceCode":"sourceCode","sourceVersion":"sourceVersion","sourceType":"sourceType","scopeType":"scopeType","scopeCode":"scopeCode","status":"sourceStatus","sourceStatus":"sourceStatus","title":"title","sourceMetadata":"sourceMetadataJson","skillName":"skillName","skillDescription":"skillDescription"},"textUnit":{"sourceCode":"sourceCode","sourceVersion":"sourceVersion","scopeType":"scopeType","scopeCode":"scopeCode","targetCode":"targetCode","textUnitStatus":"textUnitStatus","text":"text","skillName":"skillName","skillDescription":"skillDescription"}}',
        '{"layout":"GRAPH_SINGLE_NODE","namespace":"rag_skill","nodeLabel":"RagSkill","codeStrategy":"CORPUS_SOURCE_VERSION_SCOPE_HASH","diagnosticsTable":"rag_diagnostics"}',
        1780502400000,
        1780502400000
    )
ON CONFLICT (corpus_id) DO UPDATE SET
    display_name = excluded.display_name,
    status = excluded.status,
    description = excluded.description,
    capabilities_json = excluded.capabilities_json,
    filter_schema_json = excluded.filter_schema_json,
    field_mapping_json = excluded.field_mapping_json,
    storage_binding_json = excluded.storage_binding_json,
    updated_at = excluded.updated_at;

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
        'skill-graph-ingest-v1',
        'INGEST',
        'enabled',
        'skill-graph',
        '["skill-graph"]',
        NULL,
        '{"pipelineVersion":"v1","sourceInputPolicy":{"supportedSourceTypes":["SKILL"],"directContentEnabled":true},"dag":{"nodes":["validateSource","preparePayload","splitText","fulltextIndex","vectorIndex","persistSource"],"edges":[{"from":"validateSource","to":"preparePayload"},{"from":"preparePayload","to":"splitText"},{"from":"splitText","to":"fulltextIndex"},{"from":"fulltextIndex","to":"vectorIndex"},{"from":"vectorIndex","to":"persistSource"}]},"stages":{"validateSource":{"strategyId":"RAG_VALIDATE_SOURCE"},"preparePayload":{"strategyId":"RAG_TEXT_CONTRACT_PAYLOAD_V1"},"splitText":{"strategyId":"RAG_SPLIT_TEXT","config":{"splitter":{"type":"singleNode","maxTokens":1200,"overlapTokens":0}}},"fulltextIndex":{"strategyId":"RAG_FULLTEXT_INDEX"},"vectorIndex":{"strategyId":"RAG_VECTOR_INDEX"},"persistSource":{"strategyId":"RAG_PERSIST_SOURCE"}},"embeddingVersion":"v1","graphWritePolicy":{"allowedNodeKinds":["TOOL","CONFIG","VARIABLE","AGENT","TAG"],"allowedRelationships":["REQUIRES_TOOL","HAS_CONFIG","USES_VARIABLE","USED_BY_AGENT","HAS_TAG"],"maxRelatedNodes":50,"maxRelatedEdges":100}}',
        '{}',
        'SKIP_VECTOR_WITH_WARNING',
        1780502400000,
        1780502400000
    ),
    (
        'skill-graph-related-v1',
        'RETRIEVAL',
        'enabled',
        'skill-graph',
        '["skill-graph"]',
        'v1',
        '{"routes":[{"routeId":"skill-graph-fulltext","retriever":"FULLTEXT_GRAPH_SINGLE_NODE_V1"},{"routeId":"skill-graph-vector","retriever":"VECTOR_GRAPH_SINGLE_NODE_V1"}],"fusionPolicy":{"type":"SUM_SCORE"},"dedupPolicy":{"type":"TEXT_UNIT"},"rerankPolicy":{"enabled":false},"hydratePolicy":{"type":"TEXT_UNIT"},"graphExpansionPolicy":{"enabled":true,"direction":"OUTGOING","maxDepth":1,"maxRelatedNodes":20,"failurePolicy":"WARN_AND_RETURN_BASE_HITS"},"contextPolicy":{"supportedStrategies":["RESULT_LIST","TOKEN_BUDGET_PACK","MEMORY_CONTEXT_PACK","DOC_QA_CONTEXT_PACK"]}}',
        '{}',
        'SKIP_VECTOR_ON_UNAVAILABLE',
        1780502400000,
        1780502400000
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
