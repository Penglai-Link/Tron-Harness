UPDATE rag_profiles
SET degrade_policy = 'SKIP_VECTOR_WITH_WARNING'
WHERE profile_id = 'memory-card-ingest-v1';
