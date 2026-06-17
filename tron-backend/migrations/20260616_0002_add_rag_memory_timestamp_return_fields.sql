UPDATE rag_corpora
SET
    field_mapping_json = json_set(
        field_mapping_json,
        '$.source.sourceCreatedAt',
        'source_created_at',
        '$.source.sourceUpdatedAt',
        'source_updated_at'
    ),
    updated_at = 1781568000000
WHERE corpus_id = 'memory-card';
