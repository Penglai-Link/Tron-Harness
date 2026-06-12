# Test Plan: RAG Multi-Corpus RRF And Topic Strategy

## Test Plan Decision

- status: approved_with_defaults
- approved_by: langtaoxin
- approved_at: 2026-06-03

## Proof Rows

- AC-RRT-01 | layer: backend-unit | case: profile compiler parses RRF and topic policy with legacy defaults | red: compiler rejects explicit policies or changes default policy | green: compiler exposes expected policy values and legacy profiles compile | command_ref: CMD-BE-TEST.
- AC-RRT-02 | layer: backend-integration | case: multi-corpus RRF ranks by route rank and weight | red: raw-score summation or route order determines final order | green: RRF candidate score and order follow rank formula | command_ref: CMD-BE-TEST.
- AC-RRT-03 | layer: backend-unit | case: equal target codes from different corpora remain separate without a canonical content match | red: candidates from different corpora collapse by targetCode only | green: identity fallback includes corpus identity | command_ref: CMD-BE-TEST.
- AC-RRT-04 | layer: backend-unit | case: embedding MMR promotes a relevant but different topic into the rerank window | red: top window is filled by near-duplicate embedded candidates | green: selected order includes the diverse candidate before near-duplicate overflow | command_ref: CMD-BE-TEST.
- AC-RRT-05 | layer: backend-unit | case: topic policy fallback keeps fused order when embeddings are missing | red: missing embeddings error or unstable ordering | green: search succeeds and preserves fused order | command_ref: CMD-BE-TEST.
- AC-RRT-06 | layer: backend-regression | case: existing RAG search contract and HTTP contract tests remain stable | red: existing tests fail from response or filter behavior drift | green: focused and full backend tests pass | command_ref: CMD-BE-TEST.

## Command Index

- CMD-BE-TEST | command: `make be-test` | cost: medium | when: verify | rerun: after RAG search, profile, repository projection, or backend tests change | expect: backend tests pass.
- CMD-BE-CHECK | command: `make be-check` | cost: medium | when: verify | rerun: after backend module wiring changes | expect: backend compile check passes.
- CMD-DOCS-FINAL | command: `python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/rag-rrf-topic-strategy --phase final` | cost: fast | when: verify | rerun: after feature docs, tasks, orchestration, or reports change | expect: final docs validation passes.

## Evidence Rules

- RED evidence may be a focused failing backend test or documented pre-change source assertion.
- GREEN evidence must use Makefile commands first.
- Trace evidence must not expose raw provider payloads.
- Browser evidence is not required because this feature has no frontend surface.
