# Worktree Plan: Auth

## Main Agent

- feature_slug: auth
- main_agent: lead agent
- main_codex_runtime_id: current-thread
- main_codex_runtime_name: lead-agent
- git_skill: eliteforge-git-feature-oriented-lite-spec
- execution_mode: solo
- main_branch: feature/clouds3n/auth-ldap-login/base
- main_worktree_path: /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/auth-ldap-login/base
- status: complete

## Repository State

- default_branch: master
- base_ref: master
- current_branch: feature/clouds3n/auth-ldap-login/base
- note: the 2026-06-09 evolution is bounded to adding LDAP credential login under the existing auth feature. Implementation stayed solo because backend config, external identity reuse, login-page ordering, and auth smoke updates all touch the same tightly coupled auth surface where worker fan-out would not reduce risk or wall-clock materially.

## Multi-Agent Base Branch

- branch: feature/clouds3n/auth-ldap-login/base
- base_ref: master
- merge_strategy: lead-local commits on the main feature branch
- conflict_owner: Main Agent

## Agent Worktrees

- None. Solo execution keeps all auth work in the main feature worktree.

## Dispatch Decisions

| ac_refs | decision | execution_owner | rationale | worker_ref | status |
| --- | --- | --- | --- | --- | --- |
| AC-AUTH-01 | solo | lead | trivial bounded lead-owned backend auth bootstrap slice in the completed initial delivery | not_applicable | complete |
| AC-AUTH-02 | solo | lead | trivial single-slice evolution of the shared external-auth path; GitHub reuse, LDAP auto-provision, and provider-subject persistence stay tightly coupled in one worktree | not_applicable | complete |
| AC-AUTH-03 | solo | lead | trivial bounded frontend route-guard and redirect slice | not_applicable | complete |
| AC-AUTH-04 | solo | lead | trivial single-page login-surface evolution; GitHub plus LDAP ordering, labels, and provider loading all stay on one page | not_applicable | complete |
| AC-AUTH-05 | solo | lead | trivial shared backend bearer-contract and middleware slice kept together for integration safety | not_applicable | complete |
| AC-AUTH-06 | solo | lead | trivial persistence evolution because LDAP reuses the existing external identity-link storage without schema fan-out or worker benefit | not_applicable | complete |
| AC-AUTH-07 | solo | lead | trivial shared auth-error evolution because LDAP and OAuth2 rejection mapping stay on the same backend surface | not_applicable | complete |
| AC-AUTH-08 | solo | lead | trivial frontend-only lifecycle regression slice covering idle renewal, stale-token retry, reload, and logout proof | not_applicable | complete |
| AC-AUTH-09 | solo | lead | trivial bounded frontend-only protected-route guard and Playwright proof; explorer subagents were read-only and implementation write scope is tightly coupled | not_applicable | complete |
| AC-AUTH-10 | solo | lead | trivial tightly coupled startup bootstrap plus route-guard exception slice spanning backend SQL, one integration test, and one browser proof | not_applicable | complete |

## Branch And Path Rules

- Branch and worktree mirror `eliteforge-git-feature-oriented-lite-spec`.
- No delegated worker branch or worktree is created unless the execution mode changes from solo.

## Merge Strategy

- Direct lead-local commits on `feature/clouds3n/auth-ldap-login/base`.
- No worker merges are expected in the current plan.

## Conflict Rules

- The main auth worktree owns every doc, backend, frontend, and test edit in this slice.

## Cleanup Plan

- Keep `feature/clouds3n/auth-ldap-login/base` until auth verification and review finish.
- No worker cleanup is required in solo mode.

## Open Decisions

- None.
