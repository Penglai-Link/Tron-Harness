# Acceptance: RAG Multi-Corpus RRF And Topic Strategy

## Acceptance Index

- AC-RRT-01 [must] [api] Retrieval profiles parse optional RRF, canonical dedup, and topic policy while legacy profiles keep defaults; proof: backend contract tests.
- AC-RRT-02 [must] [business] RRF ranks cross-route and cross-corpus candidates by route rank and route weight; proof: multi-corpus search test.
- AC-RRT-03 [must] [business] Dedup keeps unrelated cross-corpus rows separate unless canonical text matches; proof: ranking contract tests.
- AC-RRT-04 [must] [business] Embedding MMR promotes a relevant different-topic candidate before near-duplicate overflow; proof: multi-corpus MMR test.
- AC-RRT-05 [must] [regression] Missing candidate embeddings keep fused order and do not fail search; proof: fallback search test.
- AC-RRT-06 [must] [regression] Existing RAG search API, filters, trace, rerank, and context contracts stay stable; proof: full backend test run.

## Open Decisions

- None.
