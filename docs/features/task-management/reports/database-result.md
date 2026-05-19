# Database Report: Task Management

## Summary

SQLite task persistence now stores only the final superpower enums. Legacy task schema/data is conditionally rebuilt once, and backend task seed rows are removed.

## Evidence Rows

- AC-TASK-08 | red: task SQLite bootstrap still used old enum checks and starter seed rows | green: `rtk make be-test` passed with a legacy-schema fixture proving the repository rebuilds task tables under the new contract and leaves them empty | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.

## Changed Paths

- `tron-backend/migrations/20260516_0008_create_task_management_tasks.sql`
- `tron-backend/migrations/20260519_0010_reset_task_management_superpower.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/task_repository.rs`

## Commands

- `rtk make be-test`: pass.

## Blockers

- None.
