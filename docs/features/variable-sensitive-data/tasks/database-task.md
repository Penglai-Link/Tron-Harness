# Database Task: Variable Sensitive Data

## Execution Contract

- feature_slug: variable-sensitive-data
- status: done
- execution_mode: hybrid

## TDD Contract

Database work is grouped with the backend sensitive storage worker because migration, repository model, encryption metadata, and service behavior share the same Rust write scope.

## Task Rows

- AC-VSD-01 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/migrations; tron-backend/src/adapters/outbound/persistence/sqlite; tron-backend/tests/variable_management_api.rs | command_ref: CMD-BE-VARIABLE-TEST | status: done.
- AC-VSD-02 | phase: tdd | owner: tdd-slice-worker | scope: tron-backend/Cargo.toml; tron-backend/src/adapters/outbound/crypto; tron-backend/src/application/variable; tron-backend/src/domain/variable | command_ref: CMD-BE-VARIABLE-TEST | status: done.

## Worktree Notes

- Planned worker: @ref:orchestration/worktree-plan.md#wt-01.
- The worker must not edit frontend files.

## Integration Notes

- Migration must be integrated before frontend display fields are treated as stable.
- The cipher wrapper must hide raw `gmcrypto-core` API details from variable service code.

## Open Decisions

- None.
