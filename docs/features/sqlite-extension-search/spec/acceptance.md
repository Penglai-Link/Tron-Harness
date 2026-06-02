# Acceptance

- AC-SES-01 [must] Shared SQLite extension context detects platform, prepares SQLite connections, and records whether FTS5, sqlite-simple/Jieba, and sqlite-vec are available.
- AC-SES-02 [must] RAG migrations create idempotent FTS5 and vec0 sidecar indexes for memory and project document text-unit tables without moving business fields out of the existing tables.
- AC-SES-03 [must] Existing legacy retrievers `FULLTEXT_SQLITE_LIKE_V1` and `VECTOR_SQLITE_COSINE_V1` keep their current behavior and remain registered.
- AC-SES-04 [must] New full-text retriever `FULLTEXT_SQLITE_FTS5_SIMPLE_V1` uses SQLite FTS search and storage score rather than `LIKE`.
- AC-SES-05 [must] New vector retriever `VECTOR_SQLITE_VEC0_COSINE_V1` uses SQLite vector distance and storage score rather than application-side embedding scan.
- AC-SES-06 [must] Backfill/rebuild indexes existing eligible RAG text-unit rows into sidecar indexes and keeps newly persisted rows searchable.
- AC-SES-07 [must] Extension-backed route unavailability degrades at route level while preserving legacy search and existing API response shape.
