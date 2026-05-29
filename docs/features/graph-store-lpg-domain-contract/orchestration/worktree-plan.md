# Worktree Plan: Graph Store LPG Domain Contract

## Main Agent

- feature_slug: graph-store-lpg-domain-contract
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
- current_commit: 8bce976252f4869e447be8561bab1c7abbeffc27
- working_tree_status: LPG contract docs in progress
- note: graph scope kernel is complete and supplies `ResolvedGraphScope` for this feature.

## Multi-Agent Base Branch

- branch: feature/clouds3n/graph-storage-foundation/base
- base_ref: origin/master
- merge_strategy: worker branch merges back into the multi-agent base branch
- conflict_owner: Main Agent

## Agent Worktrees

One row is one actual git worktree checkout. `agent id` and `subtask name` follow `eliteforge-git-feature-oriented-lite-spec`; Codex runtime identity is recorded separately from git identity.

| agent id | subtask name | codex agent id | codex agent name | ac refs | tdd phase | branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| backend | lpg-domain-contract | 019e73a0-2714-79f3-8f98-a694417d114d | Halley | AC-LPG-01, AC-LPG-02, AC-LPG-03, AC-LPG-04, AC-LPG-05, AC-LPG-06, AC-LPG-07 | tdd | feature/clouds3n/graph-storage-foundation/backend/lpg-domain-contract | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/graph-storage-foundation/backend/lpg-domain-contract | feature/clouds3n/graph-storage-foundation/base | in_progress |

- worker_ref: backend/lpg-domain-contract
- workflow_owner: tdd-slice-worker
- runtime_agent_type: worker
- runtime_agent_type_source: fallback after prior `coder-spark` capacity error in this thread

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-LPG-01, AC-LPG-02, AC-LPG-03, AC-LPG-04, AC-LPG-05, AC-LPG-06, AC-LPG-07 | spawn | tdd-slice-worker; runtime_agent_type=worker | grouped because all ACs share the graph LPG Rust module write scope and command aliases; coder-spark capacity fallback recorded | backend/lpg-domain-contract | in_progress |

## Branch And Path Rules

- Branch and worktree mirror `eliteforge-git-feature-oriented-lite-spec`.
- The worker branch is created from the multi-agent base branch.
- The worker worktree path mirrors the worker branch under `/Users/clouds3n/.codex/worktrees/Tron`.
- The worker must not edit the lead/base worktree.

## Merge Strategy

- Lead provisions the worker worktree from `feature/clouds3n/graph-storage-foundation/base`.
- Worker completes RED -> GREEN -> VERIFY for the grouped AC slice.
- Lead reviews changed paths, merges `feature/clouds3n/graph-storage-foundation/backend/lpg-domain-contract` into the base branch, and reruns affected validation commands.

## Conflict Rules

- Lead owns conflict resolution on the base branch.
- Any worker diff outside graph domain/application modules, facade exports, tests, or its assigned report fragment pauses integration for review.
- No frontend, HTTP, SQLite migration, Grafeo dependency, RDF contract, or embedding runtime call is allowed in this slice.

## Cleanup Plan

- Keep worker branch and worktree until LPG contract reports and review evidence are complete.
- Cleanup can happen only after final validation and user acceptance of the feature slice.

## Open Decisions

- None.
