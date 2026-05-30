# Worktree Plan: RAG 后端底座 - Corpus 存储与 Profile 基础

## Main Agent

- feature_slug: rag-corpus-storage-profile-foundation
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/langtaoxin/rag-corpus-storage-profile-foundation/base
- main_worktree_path: /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-corpus-storage-profile-foundation/base
- status: complete

## Repository State

- default_branch: master
- base_ref: master
- current_branch: master
- note: specs and planning docs were drafted on master per the reset request; behavior-changing implementation must move to the lite-spec base branch before code edits.

## Multi-Agent Base Branch

- branch: feature/langtaoxin/rag-corpus-storage-profile-foundation/base
- base_ref: master
- merge_strategy: worker branches merge back into the base branch; lead owns integration and final validation
- conflict_owner: Main Agent

## Agent Worktrees

| agent id | subtask_name | codex_agent_id | codex_agent_name | ac_refs | tdd_phase | branch | worktree_path | base_ref | status | runtime_agent_type |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| database | schema-seed | 019e7898-9492-7c42-af8d-758cf4882f0f | Faraday | AC-RAG-01, AC-RAG-02, AC-RAG-08 | tdd | feature/langtaoxin/rag-corpus-storage-profile-foundation/database/schema-seed | /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-corpus-storage-profile-foundation/database/schema-seed | feature/langtaoxin/rag-corpus-storage-profile-foundation/base | done | worker |
| backend | service-contracts | 019e78a5-a618-7f32-89f2-b0634af79c20 | Sagan | AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-09 | tdd | feature/langtaoxin/rag-corpus-storage-profile-foundation/backend/service-contracts | /Users/langtaoxin/.codex/worktrees/tron/feature/langtaoxin/rag-corpus-storage-profile-foundation/backend/service-contracts | feature/langtaoxin/rag-corpus-storage-profile-foundation/base | done | worker |

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-RAG-01, AC-RAG-02, AC-RAG-08 | spawn | database-worker | database schema and seed work is isolated from service projection work and benefits from a bounded TDD worker after explicit coding-subagent authorization | database/schema-seed | done |
| AC-RAG-03, AC-RAG-04, AC-RAG-05, AC-RAG-06, AC-RAG-07, AC-RAG-09 | spawn | backend-worker | service, DTO, registry, filter validation, and boundary regression share one backend write scope and should be one bounded TDD worker after explicit coding-subagent authorization | backend/service-contracts | done |

## Branch And Path Rules

- Branches and worktrees follow `eliteforge-git-feature-oriented-lite-spec`.
- Developer segment resolves to `langtaoxin` because the configured git user name is not filesystem-safe for branch segments.
- Worker branch paths mirror the branch tree under `/Users/langtaoxin/.codex/worktrees/tron`.
- Worker branches and worktrees must be created from `feature/langtaoxin/rag-corpus-storage-profile-foundation/base`.

## Merge Strategy

- Integrate database worker first because backend service tests depend on migration and seed records.
- Merge worker branches into `feature/langtaoxin/rag-corpus-storage-profile-foundation/base` with `--no-ff` when possible.
- After each merge, run the command aliases referenced by the merged AC rows.

## Conflict Rules

- Main Agent owns all conflict resolution.
- If database and backend workers unexpectedly edit the same file family, stop parallel dispatch for that file family and integrate sequentially.
- Do not discard worker changes without recording the reason in the integration log.

## Cleanup Plan

- Keep the base branch and worker branches until final evidence passes.
- After final acceptance, remove worker worktrees and delete merged worker branches.
- Merge the base branch back to master only after final validation.

## Open Decisions

- None.
