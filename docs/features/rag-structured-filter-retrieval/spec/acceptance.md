# Acceptance: RAG Structured Filter Retrieval

## Acceptance Index

- AC-RSF-01 [must] [business] `STRUCTURED_FILTER_SQLITE_V1` is registered, compiles in Retrieval Profile routes, and remains explicit instead of overloading full-text blank-query behavior; proof: profile/compiler contract test.
- AC-RSF-02 [must] [business] A blank or missing query executes only structured-filter routes, skips full-text/vector routes, embedding, and rerank, and rejects profiles with no runnable structured route; proof: service contract tests.
- AC-RSF-03 [must] [database] SQLite structured-filter retrieval returns active, scoped, source-versioned rows constrained by merged filters, stable order, positive score, return fields, and limit; proof: repository-backed search contract test.
- AC-RSF-04 [must] [api] `POST /api/v1/rag/search` accepts missing query for structured-filter profiles while preserving uniform success/error envelopes and filter validation; proof: HTTP API contract tests.
- AC-RSF-05 [must] [regression] Existing query-based RAG search, vector degrade behavior, context pack assembly, and GraphSingleNode boundaries remain stable; proof: focused and full backend regression commands.

## Open Decisions

- None.
