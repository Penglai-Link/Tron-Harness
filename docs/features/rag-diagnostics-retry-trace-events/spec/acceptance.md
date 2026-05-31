# Acceptance: RAG 后端底座 - 诊断、重试、Trace 与事件

## Acceptance Index

- AC-RDT-01 [must] [api] Debug API validates required query fields, authenticated user, workspace access, direct source scope and uniform error envelopes; proof: HTTP contract tests pass.
- AC-RDT-02 [must] [business] Debug API returns read-only Profile, Source, TextUnit summary and warnings without embedding BLOB or provider payload; proof: service and HTTP tests pass.
- AC-RDT-03 [must] [api] Retry API accepts only fulltextIndex/vectorIndex stages, rejects missing Source/TextUnits, and does not run parse/normalize/split; proof: retry contract tests pass.
- AC-RDT-04 [must] [database] Fulltext retry rebuilds search_text/fulltext_status from persisted TextUnits and records safe stage results; proof: SQLite integration tests pass.
- AC-RDT-05 [must] [database] Vector retry reuses LlmRuntimeService embedding, updates embedding fields/status, persists safe warnings on failure, and never stores provider raw responses; proof: fake runtime tests pass.
- AC-RDT-06 [must] [database] Search trace writes lightweight SEARCH_TRACE rows only when enabled, with query_text, route counts, warnings, candidate refs, result count, token usage and 7-day TTL; proof: search trace tests pass.
- AC-RDT-07 [must] [business] RAG ingest/search/retry/stage/route publish EventBus lifecycle events with safe payload and non-blocking publish failure behavior; proof: event contract tests pass.
- AC-RDT-08 [must] [regression] Diagnostics stage adds no SummaryTree, GraphService, GRAPH route execution, code-graph execution, Profile UI or Memory governance flow behavior; proof: boundary tests pass.

## Open Decisions

- None.
