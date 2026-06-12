# Acceptance: RAG 后端底座 - 检索编排与 Context Pack

## Acceptance Index

- AC-RSC-01 [must] [api] Search API validates required request, authenticated user, workspace access, direct search scope and uniform error envelopes; proof: HTTP contract tests pass.
- AC-RSC-02 [must] [business] Retrieval Profile compiler rejects missing version config, reserved route, unsupported context policy and invalid route configuration; proof: service tests pass.
- AC-RSC-03 [must] [business] Search merges Profile filters and Request filters by filterSchema and rejects undeclared or mistyped fields; proof: filter contract tests pass.
- AC-RSC-04 [must] [database] Memory search retrieves only active scoped source versions through fulltext/vector routes and returns unified CandidateHit rows; proof: SQLite integration tests pass.
- AC-RSC-05 [must] [database] Project Doc search hydrates active Source/TextUnit fields and returns heading/path metadata for document QA; proof: SQLite integration tests pass.
- AC-RSC-06 [must] [business] Fusion, dedup and optional rerank produce stable ordering without leaking provider responses; proof: fake runtime tests pass.
- AC-RSC-07 [must] [business] Context assembly returns RESULT_LIST, TOKEN_BUDGET_PACK, MEMORY_CONTEXT_PACK and DOC_QA_CONTEXT_PACK within limit and tokenBudget; proof: context tests pass.
- AC-RSC-08 [must] [regression] Search adds no debug, retry, trace persistence, SummaryTree, GraphService, GRAPH route execution or Memory governance flow behavior; proof: boundary tests pass.

## Open Decisions

- None.
