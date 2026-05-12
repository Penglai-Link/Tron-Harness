# Worktree Plan: Init Rust Backend

## Main Agent

- feature_slug: init-rust-backend
- git_skill: eliteforge-git-feature-oriented-lite-spec
- main_branch: feature/clouds3n/init-rust-backend/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/init-rust-backend/base
- active_codex_worktree_path: /Users/clouds3n/.codex/worktrees/1045/Tron
- status: in_progress

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/init-rust-backend/base
- note: Codex attached the lead agent to `/Users/clouds3n/.codex/worktrees/1045/Tron`; worker worktrees are created with Lite Spec mirror paths.

## Multi-Agent Base Branch

- branch: feature/clouds3n/init-rust-backend/base
- created_from: master
- integration_role: merge worker branches after RED/GREEN evidence

## Agent Worktrees

| agent id | subtask name | task ids | tdd phase | worker branch | worktree path | base ref | status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| test | backend-contract | TEST-RED-01 | red | feature/clouds3n/init-rust-backend/test/backend-contract | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/init-rust-backend/test/backend-contract | feature/clouds3n/init-rust-backend/base | merged |
| backend | rust-service | BE-GREEN-01, BE-REFACTOR-01 | green/refactor | feature/clouds3n/init-rust-backend/backend/rust-service | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/init-rust-backend/backend/rust-service | feature/clouds3n/init-rust-backend/base | planned |
| review | final-review | REVIEW-01 | review | feature/clouds3n/init-rust-backend/review/final-review | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/init-rust-backend/review/final-review | feature/clouds3n/init-rust-backend/base | planned |

## Branch And Path Rules

- Branch and worktree names follow `eliteforge-git-feature-oriented-lite-spec`.
- Subagent `agentId` values are role names only: `test`, `backend`, and `review`.
- Worker paths mirror their worker branch tree under `/Users/clouds3n/.codex/worktrees/Tron`.

## Merge Strategy

- Merge RED test worker into `/base` first.
- Merge backend GREEN worker into `/base` after RED evidence exists.
- Review worker reads final `/base` state and reports findings.

## Conflict Rules

- Test worker owns test assets only.
- Backend worker owns backend implementation and root Makefile backend targets.
- Lead agent resolves integration conflicts in `/base`.

## Cleanup Plan

- Keep worker branches until final acceptance evidence is recorded.
- Remove worker worktrees only after user confirms cleanup.

## Open Decisions

- None.
