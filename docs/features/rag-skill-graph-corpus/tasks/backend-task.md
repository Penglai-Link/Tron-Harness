# Backend Task: RAG Skill Graph Corpus

## Execution Contract

- feature_slug: rag-skill-graph-corpus
- status: done
- execution_mode: lead_takeover_after_worker_no_output

## TDD Contract

Lead-owned TDD ran in the isolated base worktree after a worker spawn produced no files. Model-dependent tests use fake embedding/rerank runtime and temporary local graph storage only.

## Task Rows

- AC-RSG-01 | phase: tdd | owner: lead | scope: RAG corpus/profile regression tests and storage layout routing | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-02 | phase: tdd | owner: lead | scope: migrations and RAG config repository seed loading | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-03 | phase: tdd | owner: lead | scope: graph single-node ingest repository and graph node write tests | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-04 | phase: tdd | owner: lead | scope: relatedGraph payload validation and graph edge/node upsert tests | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-05 | phase: tdd | owner: lead | scope: graph fulltext/vector search routes and scoped CandidateHit rows | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-06 | phase: tdd | owner: lead | scope: ranking dedup key implementation and regression coverage | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-07 | phase: tdd | owner: lead | scope: post-rerank relation expansion tests and implementation | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-08 | phase: tdd | owner: lead | scope: graph-aware debug source and retry index routing tests | command_ref: CMD-BE-TEST | status: done.
- AC-RSG-09 | phase: verify | owner: lead | scope: Makefile verification, fake runtime evidence, docs final validation | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Main base worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-corpus/base`.
- Backend worker worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-corpus/backend/skill-graph-backend`.
- Worker `019e8e67-10d6-77c3-8ebd-fcb45942423d` produced no file changes before shutdown; lead completed implementation in the base worktree.
- The original project worktree was not used for implementation edits.

## Integration Notes

- Shared contracts updated: `RagStorageLayout`, `RagStorageBinding`, `RagSearchRepository`, RAG debug/retry routing, and app wiring.
- Graph writes use prepared graph node/edge APIs and reuse the ingest stage embedding payload; tests assert the fake embedding call count.
- Existing `skill` SQLite behavior remains default unless callers choose `skill-graph` profiles.
- `make be-check`, `make be-test`, `cargo fmt --check`, focused skill-graph contracts, and architecture guard all pass.

## Open Decisions

- None.
