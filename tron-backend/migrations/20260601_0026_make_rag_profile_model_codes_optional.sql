UPDATE rag_profiles
SET config_json = json_remove(
        json_remove(
            json_remove(config_json, '$.embeddingModelCode'),
            '$.rerankModelCode'
        ),
        '$.llmModelCode'
    ),
    updated_at = 1780243200000
WHERE profile_type IN ('INGEST', 'RETRIEVAL')
  AND (
      json_type(config_json, '$.embeddingModelCode') IS NOT NULL
      OR json_type(config_json, '$.rerankModelCode') IS NOT NULL
      OR json_type(config_json, '$.llmModelCode') IS NOT NULL
  );
