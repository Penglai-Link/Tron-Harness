# Database Task: OpenAPI HMAC Auth Foundation

## Execution Contract

- feature_slug: openapi-hmac-auth-foundation
- status: done
- execution_mode: solo

## TDD Contract

Database work stays on the selected Lite Spec base worktree and proves schema plus repository behavior before the public auth path depends on it.

## Task Rows

- AC-OAH-01 | phase: tdd | owner: lead | scope: open-app SQLite migration, repository, credential lifecycle persistence, grant persistence, and nonce replay reservation | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/openapi-open-app-management/base`.
- Branch: `feature/clouds3n/openapi-open-app-management/base`.

## Integration Notes

- Schema must remain independent from app-management tables because `open-app` is a distinct platform concept.
- Migration constants and repository constructor wiring must both be updated.

## Open Decisions

- None.
