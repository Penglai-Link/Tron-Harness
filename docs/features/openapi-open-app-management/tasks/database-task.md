# Database Task: OpenAPI Open-App Management

## Execution Contract

- feature_slug: openapi-open-app-management
- status: done
- execution_mode: solo

## TDD Contract

Database work in this slice is repository evolution only. No new schema is introduced, but RED before GREEN still applies to summary queries and transactional grant replacement behavior on the existing tables.

## Task Rows

- AC-OAM-01 | phase: refactor | owner: lead | scope: extend open-app repository summary and detail queries without changing the foundational schema | command_ref: CMD-BE-TEST | status: done.
- AC-OAM-03 | phase: refactor | owner: lead | scope: implement transactional replacement for user grants and capability-operation grants on existing tables | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/openapi-open-app-management/base`.
- Branch: `feature/clouds3n/openapi-open-app-management/base`.

## Integration Notes

- Repository evolution must remain compatible with the foundation migration and fixture setup.
- Summary and replacement helpers must not leak cleartext key material.

## Constraints

- Reuse the existing open-app schema and indexes.
- Keep one active credential invariant unchanged.

## Open Decisions

- None.
