# Test Task: Init Rust Backend

## Execution Contract

Tests run before implementation and must prove missing backend behavior through RED failure evidence.

## TDD Contract

RED rows may edit only tests, fixtures, test utilities, and explicitly assigned test assets.

## Task Table

| id | phase | owner | depends on | scope | acceptance ids | status |
| --- | --- | --- | --- | --- | --- | --- |
| TEST-RED-01 | red | test | none | Add Rust tests for backend router, health response, config defaults, and safe health payload. | AC-BACKEND-01, AC-BACKEND-02, AC-BACKEND-03 | ready |

## Worktree Notes

- Worker branch: `feature/clouds3n/init-rust-backend/test/backend-contract`.
- Worker path: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/init-rust-backend/test/backend-contract`.

## Integration Notes

- RED changes must merge into `feature/clouds3n/init-rust-backend/base` before GREEN starts.

## Open Decisions

- None.
