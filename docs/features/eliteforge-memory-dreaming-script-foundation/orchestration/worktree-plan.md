# Worktree Plan: EliteForge Memory Dreaming Script Foundation

## Main Agent

- feature_slug: eliteforge-memory-dreaming-script-foundation
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: hybrid
- main_branch: feature/baiyihuan/memory/base
- main_worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base
- status: done

## Repository State

- root_worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/base
- default_branch: master
- base_ref: master
- current_branch: feature/baiyihuan/memory/base
- selected_git_skill_verified: true
- note: current branch and worktree mirror the Lite Spec multi-agent base branch for developer `baiyihuan` and task `memory`.

## Multi-Agent Base Branch

- branch: feature/baiyihuan/memory/base
- base_ref: master
- merge_strategy: merge delegated worker branches into the selected multi-agent base branch after evidence review
- conflict_owner: Main Agent

## Dispatch Decisions

One row per behavior-changing task row after AC confirmation. Planned defaults below keep implementation on disjoint worker scopes where possible; they are not dispatched until task rows are approved. Execution should be staged: scaffold and Makefile work integrates first, then importer and workflow workers start from the refreshed base branch if their write scopes remain disjoint.

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-MDSF-01, AC-MDSF-09 | spawn | tdd-slice-worker | skill scaffold and Makefile validation share the repo root and skill metadata scope | WT-DOCS | done |
| AC-MDSF-02, AC-MDSF-03, AC-MDSF-04 | spawn | tdd-slice-worker | artifact root resolution, Codex provider, snapshots, segments, and scan decisions are a bounded skill importer slice | WT-CODEX | done |
| AC-MDSF-05, AC-MDSF-06, AC-MDSF-07, AC-MDSF-08 | spawn | tdd-slice-worker | task state machine, temp promotion, governance dry-run without Memory MCP, status, and finalization are a bounded workflow slice | WT-TASKS | done |

## Agent Worktrees

Worker worktrees are planned but not spawned before AC confirmation.

### WT-DOCS

- git_agent_id: docs
- subtask_name: skill-scaffold
- runtime_agent_type: worker
- codex_agent_id: 019e6ea9-0f90-7b21-80ad-e9e80f70af51
- codex_agent_name: Laplace
- ac_refs: AC-MDSF-01, AC-MDSF-09
- tdd_phase: tdd
- branch: feature/baiyihuan/memory/docs/skill-scaffold
- worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/docs/skill-scaffold
- base_ref: feature/baiyihuan/memory/base
- status: done

### WT-CODEX

- git_agent_id: skill
- subtask_name: codex-importer
- runtime_agent_type: worker
- codex_agent_id: 019e6eb4-9e02-7c00-a8c2-753c95a5ecea
- codex_agent_name: Jason
- ac_refs: AC-MDSF-02, AC-MDSF-03, AC-MDSF-04
- tdd_phase: tdd
- branch: feature/baiyihuan/memory/skill/codex-importer
- worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/skill/codex-importer
- base_ref: feature/baiyihuan/memory/base
- status: done

### WT-TASKS

- git_agent_id: test
- subtask_name: task-workflow
- runtime_agent_type: worker
- codex_agent_id: 019e6ecb-26e8-75d3-b758-ffc93a1db832
- codex_agent_name: Anscombe
- ac_refs: AC-MDSF-05, AC-MDSF-06, AC-MDSF-07, AC-MDSF-08
- tdd_phase: tdd
- branch: feature/baiyihuan/memory/test/task-workflow
- worktree_path: /home/baiyihuan/.codex/worktrees/tron/feature/baiyihuan/memory/test/task-workflow
- base_ref: feature/baiyihuan/memory/base
- status: done

## Branch And Path Rules

- Git skill owns branch/worktree naming; Codex runtime id/name is recorded after spawn and never used to derive git names.
- Base branch is `feature/baiyihuan/memory/base`.
- Worker branches mirror `feature/baiyihuan/memory/<agentId>/<subtaskName>`.
- Worker worktree paths mirror the branch tree under `/home/baiyihuan/.codex/worktrees/tron`.
- No worker edits the base worktree directly.
- Subagent branches merge back only to `feature/baiyihuan/memory/base`.

## Merge Strategy

- strategy: merge delegated worker branches into the selected Lite Spec multi-agent base branch after evidence review.
- integration_owner: Main Agent

## Conflict Rules

- conflict_owner: Main Agent
- worker_conflict_rule: stop and report; do not rewrite another worker's scope.
- preserve unrelated user changes.

## Cleanup Plan

- cleanup_after_merge: pending final validation
- preserve_until_final_validation: true

## Open Decisions

- None.
