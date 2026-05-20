# Worktree Plan: Automations

## Main Agent

- feature_slug: automations
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: solo
- main_branch: feature/clouds3n/automations
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/automations
- status: in_progress

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/automations
- note: implementation runs from the Lite Spec mirrored worktree because the original detached `4b89` checkout is not a valid feature branch execution surface.

## Multi-Agent Base Branch

- branch: feature/clouds3n/automations
- base_ref: master
- merge_strategy: lead-local work on the feature branch
- conflict_owner: Main Agent

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-AUTO-01, AC-AUTO-02, AC-AUTO-03 | solo | lead | user explicitly requested direct feature implementation, and current developer rules do not allow subagents without an explicit subagent request | none | in_progress |
| AC-AUTO-04, AC-AUTO-05, AC-AUTO-06 | solo | lead | user approved direct implementation, and the lifecycle, execution, and scheduler slices stay tightly coupled in one bounded backend change set | none | in_progress |
| AC-AUTO-07, AC-AUTO-08 | solo | lead | user explicitly requested direct feature implementation, and the frontend and browser work is a bounded domain slice without subagent approval | none | in_progress |

## Agent Worktrees

- None. Solo execution stays on the selected Lite Spec feature branch in the active checkout.

## Branch And Path Rules

- Branch naming follows `eliteforge-git-feature-oriented-lite-spec`.
- The active implementation branch is `feature/clouds3n/automations`.
- The active worktree path mirrors the branch tree under `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/automations`.
- No delegated worker branches or worktrees are created unless the user explicitly requests subagents.

## Merge Strategy

- Direct lead-local edits on `feature/clouds3n/automations`.
- No worker merges are expected for this feature evolution slice.

## Conflict Rules

- This feature owns automations docs, backend domain files, frontend automation files, and new smoke coverage in this slice.
- Existing task, workspace, auth, and variable domains must not be behavior-regressed while introducing the automation scheduler.

## Cleanup Plan

- Keep `feature/clouds3n/automations` until validation is complete and the user decides whether to push or merge.

## Open Decisions

- None.
