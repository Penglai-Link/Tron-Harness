UPDATE rag_profiles
SET config_json = json_remove(config_json, '$.embeddingModelCode'),
    updated_at = 1780243200000
WHERE profile_id IN ('memory-card-ingest-v1', 'memory-related-v1')
  AND json_extract(config_json, '$.embeddingModelCode') = 'fake-embedding';

UPDATE rag_profiles
SET config_json = json_remove(config_json, '$.rerankModelCode'),
    updated_at = 1780243200000
WHERE profile_id IN ('memory-related-v1', 'project-doc-related-v1')
  AND json_extract(config_json, '$.rerankModelCode') = 'fake-rerank';

UPDATE rag_profiles
SET config_json = json_remove(config_json, '$.llmModelCode'),
    updated_at = 1780243200000
WHERE json_extract(config_json, '$.llmModelCode') IN ('fake-llm', 'fake-chat');
