# Backend Report: Task Management

## Summary

Backend task management now uses the superpower role-aware status model, returns `statusRules` and `defaultStatuses`, validates status/type compatibility, and conditionally resets legacy task schema/data without reseeding starter rows.

## Evidence Rows

- AC-TASK-01 | red: task options still exposed the old global status/type enum contract | green: `rtk make be-test` passed with `/api/v1/tasks/options` asserting `Feature | Bug`, the new status universe, `statusRules`, and `defaultStatuses` | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-02 | red: kanban/list could still reflect seeded legacy rows or stale status columns | green: `rtk make be-test` passed with workspace-scoped list/kanban assertions and fresh databases starting empty | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-03 | red: create still defaulted to old statuses instead of role-owned `Open` | green: `rtk make be-test` passed with create assertions for active workspace, effective assignees, generated `taskCode`, and role-default `Open` | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-04 | red: parent-child detail behavior could regress while switching role logic | green: `rtk make be-test` passed with same-workspace parent validation, active-subtask detail filtering, and parent summaries | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-05 | red: status updates and task-type edits still allowed role-incompatible states | green: `rtk make be-test` passed with `TASK_STATUS_ROLE_INVALID` coverage for invalid feature-root, feature-child, and task-type change cases | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-06 | red: restore still fell back to `Backlog` and archive rules could regress | green: `rtk make be-test` passed with active-subtask archive guard retention and restore fallback to `Open` when `previousStatus` is absent | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.
- AC-TASK-08 | red: backend bootstrap still seeded legacy tasks and preserved old schema/data | green: `rtk make be-test` passed with a legacy SQLite schema fixture that was conditionally rebuilt to the new enum contract and left with zero task rows | command_ref: CMD-BE-TEST | browser:not applicable | status: pass.

## Changed Paths

- `tron-backend/src/domain/task.rs`
- `tron-backend/src/application/task.rs`
- `tron-backend/src/adapters/inbound/http/tasks/error_mapper.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/task_repository.rs`
- `tron-backend/migrations/20260516_0008_create_task_management_tasks.sql`
- `tron-backend/migrations/20260519_0010_reset_task_management_superpower.sql`
- `tron-backend/tests/task_management_api.rs`

## Commands

- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api`: pass.
- `rtk make be-test`: pass.

## Blockers

- None.
