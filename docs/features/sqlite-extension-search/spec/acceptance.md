# Acceptance

- AC-SES-01 [must] Shared SQLite extension context detects platform and prepares SQLite connections without persisting or caching capability state.
- AC-SES-02 [must] RAG migrations create idempotent FTS5 and fixed 4096-dimension vec0 sidecar indexes for memory and project document text-unit tables, and remove full-text/vector index payload columns from the business text-unit tables.
- AC-SES-03 [must] Existing retriever IDs `FULLTEXT_SQLITE_LIKE_V1` and `VECTOR_SQLITE_COSINE_V1` remain registered and read through the reusable sidecar index contract.
- AC-SES-04 [must] Memory and project document default full-text routes use `FULLTEXT_SQLITE_FTS5_SIMPLE_V1` with SQLite FTS search and storage score rather than `LIKE`.
- AC-SES-05 [must] Memory and project document default vector routes use `VECTOR_SQLITE_VEC0_COSINE_V1` with SQLite vector distance and storage score rather than application-side embedding scan.
- AC-SES-06 [must] Ingest/retry writes newly persisted full-text and vector payload directly into sidecar indexes, and migration backfills FTS plus any eligible legacy vector rows.
- AC-SES-07 [must] Sidecar route unavailability degrades at route level while preserving existing API response shape.
