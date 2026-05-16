# Backend Report: Task Management

## Summary

Backend task-management v1 is implemented and verified through the API suite and full backend test target.

## Evidence Rows

- AC-TASK-01 | red: task options route and final enum contract were absent | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-02 | red: task kanban/list routes were missing | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-03 | red: create route lacked final workspace, assignee, type, source, and link validation | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-04 | red: parent/child same-workspace validation and archived-subtask filtering were not wired | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-05 | red: dedicated status route and final progress rules were missing | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.
- AC-TASK-06 | red: archive/restore routes, archived mutation guard, and active-subtask parent guard were missing | green: `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api` passed | command_ref: CMD-BE-TEST | browser: clicks=create/edit/save/add subtask/child-parent links/archive guard/archive/restore/execute; inputs=workspace/search/type/source/url/users/agents/apps/progress; network=options/kanban/create/detail/update/status/archive guard/archive/restore asserted; feedback=created/saved/subtask/archive guard/archive/restore/execution Toasts; artifacts=Playwright line output | status: pass.

## Changed Paths

- `tron-backend/src/domain/task.rs`
- `tron-backend/src/application/task.rs`
- `tron-backend/src/adapters/inbound/http/tasks/**`
- `tron-backend/src/adapters/outbound/persistence/sqlite/task_repository.rs`
- `tron-backend/migrations/20260516_0008_create_task_management_tasks.sql`
- `tron-backend/tests/task_management_api.rs`

## Commands

- `rtk make be-check`: pass.
- `rtk cargo test --manifest-path tron-backend/Cargo.toml --test task_management_api`: pass.
- `rtk make be-test`: pass.

## Blockers

- None.
