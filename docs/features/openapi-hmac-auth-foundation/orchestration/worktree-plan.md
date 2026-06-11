# Worktree Plan: OpenAPI HMAC Auth Foundation

## Main Agent

- feature_slug: openapi-hmac-auth-foundation
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: solo
- main_branch: feature/clouds3n/openapi-open-app-management/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/openapi-open-app-management/base
- status: done

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/openapi-open-app-management/base
- note: selected Lite Spec multi-agent base branch was moved from the root checkout to the mirrored `/base` worktree before behavior-changing execution.

## Multi-Agent Base Branch

- branch: feature/clouds3n/openapi-open-app-management/base
- base_ref: master
- merge_strategy: lead-local work on the selected base branch
- conflict_owner: Main Agent

## Agent Worktrees

- None. Current runtime does not expose implementation subagent worktrees in this thread, so bounded execution stays on the selected Lite Spec base worktree.

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-OAH-01 | solo | lead | user approved the bounded feature direction, and the current runtime does not expose implementation subagents in this thread | none | done |
| AC-OAH-02 | solo | lead | user approved the bounded feature direction, and signed auth changes are tightly coupled to the same middleware slice | none | done |
| AC-OAH-03 | solo | lead | user approved the bounded feature direction, and acting-user plus capability checks share the same auth service and fixtures | none | done |
| AC-OAH-04 | solo | lead | user approved the bounded feature direction, and route metadata plus uniform error mapping stay in the same middleware boundary | none | done |

## Branch And Path Rules

- Branch naming follows `eliteforge-git-feature-oriented-lite-spec`.
- Main worktree path mirrors `feature/clouds3n/openapi-open-app-management/base` under `/Users/clouds3n/.codex/worktrees/Tron`.
- No delegated worker may edit the root checkout at `/Users/clouds3n/Coding/open-source/ai/Tron` for this feature.

## Merge Strategy

- Direct lead-local edits on `feature/clouds3n/openapi-open-app-management/base`.
- No worker merge is expected for this feature.

## Conflict Rules

- Preserve unrelated user changes.
- Keep `open-app` foundation files separate from existing app-management modules.
- Do not widen auth fallback paths once HMAC enforcement lands.

## Cleanup Plan

- Keep the base worktree until the dependent `openapi-open-app-management` slice and any remaining PRD follow-up verification close on the same branch.

## Open Decisions

- None.
