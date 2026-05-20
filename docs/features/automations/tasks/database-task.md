# Database Task: Automations

## Execution Contract

- owner: lead
- scope: SQLite migration, repository storage shape, indexes, and execution-history persistence
- mode: solo

## TDD Contract

- Start from failing repository-backed automation API tests that prove empty-state, create, lifecycle, and execution-history persistence.

## Task Rows

- AC-AUTO-03 | phase: tdd | owner: lead | scope: create-schema migration for automations and execution records | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-05 | phase: tdd | owner: lead | scope: execution-history persistence and latest-execution rollup writes | command_ref: CMD-BE-TEST | status: ready.

## Worktree Notes

- Schema work stays in the canonical `feature/clouds3n/automations` worktree.

## Integration Notes

- Keep migration naming and repository timestamp conventions consistent with the task and workspace domains.

## Open Decisions

- None.
