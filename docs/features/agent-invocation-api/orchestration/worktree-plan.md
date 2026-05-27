# Worktree Plan: Agent Invocation API

## Main Agent

- feature_slug: agent-invocation-api
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: solo
- main_branch: feature/clouds3n/agent-framework/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-framework/base
- status: done

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/agent-framework/base
- note: selected Lite Spec multi-agent base branch already exists and current worktree mirrors the branch tree.

## Multi-Agent Base Branch

- branch: feature/clouds3n/agent-framework/base
- base_ref: master
- merge_strategy: lead-local work on the selected base branch
- conflict_owner: Main Agent

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-AIA-01 | solo | lead | user approved Codex-owned decisions and current tool policy does not permit implementation subagents without an explicit subagent request | none | done |
| AC-AIA-02 | solo | lead | user approved Codex-owned decisions and validation logic is tightly coupled to the same API route | none | done |
| AC-AIA-03 | solo | lead | user approved Codex-owned decisions and context freezing integrates existing services in one backend slice | none | done |
| AC-AIA-04 | solo | lead | user approved Codex-owned decisions and error mapping shares the same route and service layer | none | done |

## Agent Worktrees

- None. Solo execution stays on the selected Lite Spec base branch in the active worktree.

## Branch And Path Rules

- Branch naming follows `eliteforge-git-feature-oriented-lite-spec`.
- Worktree path mirrors `feature/clouds3n/agent-framework/base` under `/Users/clouds3n/.codex/worktrees/Tron`.

## Merge Strategy

- Direct lead-local edits on `feature/clouds3n/agent-framework/base`.
- No worker merge is expected for this feature.

## Conflict Rules

- Preserve unrelated user changes.
- Keep API code isolated to agent route/service modules and necessary exports.

## Cleanup Plan

- Keep the base worktree until both PRD features are accepted.

## Open Decisions

- None.
