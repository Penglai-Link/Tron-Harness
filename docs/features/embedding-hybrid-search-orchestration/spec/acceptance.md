# Acceptance: Embedding And Hybrid Search Orchestration

## Acceptance Index

- AC-EMBED-01 [must] [business] Vector-indexed graph writes derive embeddings through the LLM runtime and persist adapter-owned vector payloads without exposing caller-supplied vectors; proof: @ref:spec/test-plan.md#proof-rows.
- AC-EMBED-02 [must] [business] Long vector-indexed writes use chunked embedding derivation and persist chunk-aware vector payloads atomically; proof: @ref:spec/test-plan.md#proof-rows.
- AC-EMBED-03 [must] [business] Hybrid graph search combines structured filters, full-text clauses, vector clauses, labels, and traversal constraints under one resolved graph scope; proof: @ref:spec/test-plan.md#proof-rows.
- AC-EMBED-04 [must] [security] Vector search or vector-indexed writes fail with typed embedding errors when no embedding model is available or the provider fails, and they do not silently degrade; proof: @ref:spec/test-plan.md#proof-rows.
- AC-EMBED-05 [must] [regression] Backend graph boundaries remain internal-only: no graph HTTP route, no caller raw vector field, and no raw Grafeo query leakage from public graph contracts; proof: @ref:spec/test-plan.md#proof-rows.

## Open Decisions

- None.
