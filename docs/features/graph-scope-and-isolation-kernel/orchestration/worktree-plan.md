# Worktree Plan: Graph Scope And Isolation Kernel

## Main Agent

- feature_slug: graph-scope-and-isolation-kernel
- main_agent: Codex lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/clouds3n/graph-storage-foundation/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/base
- status: ready

## Repository State

- default_branch: master
- base_ref: origin/master
- current_branch: feature/clouds3n/graph-storage-foundation/base
- current_commit: a73a1b51eae48033826bbc0cc0e9ea7465cc3a83
- working_tree_status: feature docs in progress
- note: the selected feature is foundational and must finish before LPG, ontology, Grafeo adapter, and hybrid search feature work.

## Multi-Agent Base Branch

- branch: feature/clouds3n/graph-storage-foundation/base
- base_ref: origin/master
- merge_strategy: worker branch merges back into the multi-agent base branch
- conflict_owner: Main Agent

## Agent Worktrees

One row is one actual git worktree checkout. `agent id` and `subtask name` follow `eliteforge-git-feature-oriented-lite-spec`; Codex runtime identity is recorded separately from git identity.

| agent id | subtask name | codex agent id | codex agent name | ac refs | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | graph-scope-kernel | 019e738f-57b6-7590-9de9-167cb30d7a81 | Dalton | AC-GSI-01, AC-GSI-02, AC-GSI-03, AC-GSI-04, AC-GSI-05, AC-GSI-06, AC-GSI-07 | tdd | feature/clouds3n/graph-storage-foundation/backend/graph-scope-kernel | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/backend/graph-scope-kernel | feature/clouds3n/graph-storage-foundation/base | in_progress |

- worker_ref: backend/graph-scope-kernel
- workflow_owner: tdd-slice-worker
- runtime_agent_type: worker
- runtime_agent_type_source: fallback after `coder-spark` capacity error from Codex runtime `019e738a-a7cc-7281-ba7b-6ca28affa8c7`

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-GSI-01, AC-GSI-02, AC-GSI-03, AC-GSI-04, AC-GSI-05, AC-GSI-06, AC-GSI-07 | spawn | tdd-slice-worker; runtime_agent_type=worker | grouped because all ACs share the graph scope Rust module write scope and command aliases; coder-spark capacity fallback recorded | backend/graph-scope-kernel | in_progress |

## Branch And Path Rules

- Branch and worktree mirror `eliteforge-git-feature-oriented-lite-spec`.
- The main branch is the selected multi-agent base branch.
- The worker branch is created from the multi-agent base branch.
- The worker worktree path mirrors the worker branch under `/Users/clouds3n/.codex/worktrees/Tron`.
- The worker must not edit the lead/base worktree.

## Merge Strategy

- Lead provisions the worker worktree from `feature/clouds3n/graph-storage-foundation/base`.
- Worker completes RED -> GREEN -> VERIFY for the grouped AC slice.
- Lead reviews changed paths, merges `feature/clouds3n/graph-storage-foundation/backend/graph-scope-kernel` into the base branch, and reruns affected validation commands.

## Conflict Rules

- Lead owns conflict resolution on the base branch.
- Any worker diff outside graph domain/application modules, facade exports, test files, or its assigned report fragment pauses integration for review.
- No frontend, HTTP, SQLite migration, or Grafeo dependency changes are allowed in this slice.

## Cleanup Plan

- Keep worker branch and worktree until graph scope reports and review evidence are complete.
- Cleanup can happen only after final validation and user acceptance of the feature slice.

## Open Decisions

- None.
