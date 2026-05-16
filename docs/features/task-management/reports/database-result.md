# Database Report: Task Management

## Summary

SQLite persistence is implemented for tasks, user assignments, agent links, app links, archive state, and starter records.

## Evidence Rows

- AC-TASK-02 | red: no task tables or repository existed for persisted kanban/list projection | green: `rtk make be-test` passed with SQLite-backed task API tests | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-06 | red: no archived status, previous status, archived timestamp persistence, or active-subtask parent guard existed | green: archive/restore lifecycle test passed in `task_management_api` and full `be-test` | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.

## Changed Paths

- `tron-backend/migrations/20260516_0008_create_task_management_tasks.sql`
- `tron-backend/src/adapters/outbound/persistence/sqlite/migrations.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite/task_repository.rs`
- `tron-backend/src/adapters/outbound/persistence/sqlite.rs`

## Commands

- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api`: pass.
- `rtk make be-test`: pass.

## Blockers

- None.
