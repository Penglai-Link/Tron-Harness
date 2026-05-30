# Acceptance: RAG 后端底座 - 同步写入 DAG

## Acceptance Index

- AC-RID-01 [must] [api] Ingest API validates required request, workspace access, and direct target scope; proof: HTTP contract tests pass.
- AC-RID-02 [must] [business] Ingest Profile DAG compiler rejects invalid corpus, cycles, missing stages, and unregistered strategies; proof: service tests pass.
- AC-RID-03 [must] [database] Memory ingest upserts Source and TextUnits with fulltext and vector fields; proof: SQLite integration tests pass.
- AC-RID-04 [must] [database] Project Doc Markdown ingest persists document fields and split TextUnits; proof: SQLite integration tests pass.
- AC-RID-05 [must] [business] Same sourceCode and sourceVersion retry is idempotent while multiple versions coexist; proof: repository tests pass.
- AC-RID-06 [must] [business] Project Doc vector failure degrades to active with warning while Memory vector failure becomes failed; proof: embedding failure tests pass.
- AC-RID-07 [must] [regression] Ingest adds no search, debug, retry, SummaryTree, Graph, or Memory flow behavior; proof: boundary tests pass.

## Open Decisions

- None.
