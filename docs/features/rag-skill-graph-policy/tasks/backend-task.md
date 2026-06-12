# Backend Task: RAG Skill Graph Policy

## Execution Contract

- feature_slug: rag-skill-graph-policy
- status: done
- execution_mode: hybrid

## TDD Contract

Lead-owned TDD ran in the isolated base worktree. A read-only explorer scanned hardcoded graph policy risks in parallel. Model-dependent tests use fake embedding/rerank runtime and temporary local graph storage only.

## Task Rows

- AC-RSGP-01 | phase: tdd | owner: lead | scope: optional default graph write policy accepts custom kind and relationship | command_ref: CMD-RSGP-FOCUSED | status: done.
- AC-RSGP-02 | phase: tdd | owner: lead | scope: configured graph write whitelists reject out-of-list values | command_ref: CMD-RSGP-FOCUSED | status: done.
- AC-RSGP-03 | phase: verify | owner: explorer/static-scan | scope: static review of hardcoded business write-policy constants | command_ref: CMD-RSGP-STATIC | status: done.
- AC-RSGP-04 | phase: tdd | owner: lead | scope: field-mapped graph business properties write and return | command_ref: CMD-RSGP-FOCUSED | status: done.
- AC-RSGP-05 | phase: tdd | owner: lead | scope: edge scalar property write and expansion return | command_ref: CMD-RSGP-FOCUSED | status: done.
- AC-RSGP-06 | phase: verify | owner: lead | scope: backend regression suite for existing RAG corpora | command_ref: CMD-BE-TEST | status: done.
- AC-RSGP-07 | phase: verify | owner: lead | scope: fake runtime evidence for model-dependent tests | command_ref: CMD-RSGP-FOCUSED | status: done.

## Worktree Notes

- Base worktree: `/Users/langtaoxin/.codex/worktrees/tron/feature/xinlangtao/rag-skill-graph-policy/base`.
- Explorer agent was read-only and did not create file changes.

## Integration Notes

- Existing root worktree was not edited directly for this implementation; current uncommitted prerequisite diff was copied into the isolated base worktree.
- No worker merge was required because implementation files were tightly coupled and the only parallel agent was read-only.

## Open Decisions

- None.
