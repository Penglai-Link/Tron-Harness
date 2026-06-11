# Worktree Plan: OpenAPI Open-App Management

## Main Agent

- feature_slug: openapi-open-app-management
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
- note: this bounded management slice continues on the same selected Lite Spec base worktree after the HMAC foundation slice.

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
| AC-OAM-01 | solo | lead | user approved continuing this bounded slice lead-locally on the selected base worktree, and lifecycle APIs reuse the same service and repository slice as the HMAC foundation | none | done |
| AC-OAM-02 | solo | lead | user approved continuing this bounded slice lead-locally on the selected base worktree, and catalog reuse must stay in one implementation boundary to avoid drift | none | done |
| AC-OAM-03 | solo | lead | user approved continuing this bounded slice lead-locally on the selected base worktree, and grant replacement shares the same service validation and repository transactions | none | done |
| AC-OAM-04 | solo | lead | user approved continuing this bounded slice lead-locally on the selected base worktree, and the settings page is one bounded surface directly tied to the admin API contract | none | done |
| AC-OAM-05 | solo | lead | user approved continuing this bounded slice lead-locally on the selected base worktree, and browser evidence must validate the exact page built in the same worktree | none | done |

## Branch And Path Rules

- Branch naming follows `eliteforge-git-feature-oriented-lite-spec`.
- Main worktree path mirrors `feature/clouds3n/openapi-open-app-management/base` under `/Users/clouds3n/.codex/worktrees/Tron`.
- No delegated worker may edit the root checkout at `/Users/clouds3n/Coding/open-source/ai/Tron` for this feature.

## Merge Strategy

- Direct lead-local edits on `feature/clouds3n/openapi-open-app-management/base`.
- No worker merge is expected for this feature.

## Conflict Rules

- Preserve unrelated user changes.
- Keep `open-app` management surfaces separate from existing app-management modules.
- Do not duplicate the runtime route-grant registry in frontend or backend management code.

## Cleanup Plan

- Keep the base worktree until the follow-up public adapter migration slice is accepted.

## Open Decisions

- None.
