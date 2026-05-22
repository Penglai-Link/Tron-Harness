# Worktree Plan: Auth

## Main Agent

- feature_slug: auth
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: solo
- main_branch: feature/clouds3n/auth/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/auth/base
- status: complete

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/auth/base
- note: the interrupted auth pass first surfaced on the root repository worktree while still on `master`; the current 2026-05-22 evolution runs in the mirrored `feature/clouds3n/auth/base` worktree and is bounded to frontend session lifecycle recovery, so no worker worktrees are planned for this slice.

## Multi-Agent Base Branch

- branch: feature/clouds3n/auth/base
- base_ref: master
- merge_strategy: lead-local commits on the main feature branch
- conflict_owner: Main Agent

## Agent Worktrees

- None. Solo execution keeps all auth work in the main feature worktree.

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-AUTH-01 | solo | lead | trivial bounded lead-owned backend auth bootstrap slice in the completed initial delivery | not_applicable | complete |
| AC-AUTH-02 | solo | lead | trivial tightly coupled backend OAuth callback and config slice in the completed initial delivery | not_applicable | complete |
| AC-AUTH-03 | solo | lead | trivial bounded frontend route-guard and redirect slice | not_applicable | complete |
| AC-AUTH-04 | solo | lead | trivial bounded frontend auth UI and provider-loading slice | not_applicable | complete |
| AC-AUTH-05 | solo | lead | trivial shared backend bearer-contract and middleware slice kept together for integration safety | not_applicable | complete |
| AC-AUTH-06 | solo | lead | trivial additive SQLite auth persistence slice with no delegation benefit | not_applicable | complete |
| AC-AUTH-07 | solo | lead | trivial auth rejection and security-boundary slice kept in one lead-owned backend proof pass | not_applicable | complete |
| AC-AUTH-08 | solo | lead | trivial frontend-only lifecycle regression slice covering idle renewal, stale-token retry, reload, and logout proof | not_applicable | complete |

## Branch And Path Rules

- Branch and worktree mirror `eliteforge-git-feature-oriented-lite-spec`.
- No delegated worker branch or worktree is created unless the execution mode changes from solo.

## Merge Strategy

- Direct lead-local commits on `feature/clouds3n/auth/base`.
- No worker merges are expected in the current plan.

## Conflict Rules

- The main auth worktree owns every doc, backend, frontend, and test edit in this slice.

## Cleanup Plan

- Keep `feature/clouds3n/auth/base` until auth verification and review finish.
- No worker cleanup is required in solo mode.

## Open Decisions

- None.
