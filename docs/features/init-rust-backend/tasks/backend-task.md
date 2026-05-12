# Backend Task: Init Rust Backend

## Execution Contract

Implement the smallest backend runtime needed to pass the approved RED tests and backend checks.

## TDD Contract

GREEN work must not weaken tests. Refactor work may clean structure only after tests pass.

## Task Table

| id | phase | owner | depends on | scope | acceptance ids | status |
| --- | --- | --- | --- | --- | --- | --- |
| BE-GREEN-01 | green | backend | TEST-RED-01 | Initialize Cargo package, Axum router, uniform response envelope, health endpoint, config defaults, and backend Makefile targets. | AC-BACKEND-01, AC-BACKEND-02, AC-BACKEND-03 | ready |
| BE-REFACTOR-01 | refactor | backend | BE-GREEN-01 | Keep backend modules small and aligned with the documented modular architecture. | AC-BACKEND-01, AC-BACKEND-02 | ready |

## Worktree Notes

- Worker branch: `feature/clouds3n/init-rust-backend/backend/rust-service`.
- Worker path: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/init-rust-backend/backend/rust-service`.

## Integration Notes

- Backend worker starts after RED tests are merged or replayed into the base branch.

## Open Decisions

- None.
