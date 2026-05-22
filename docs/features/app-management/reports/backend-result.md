# Backend Report: App Management

## Summary

Backend now exposes workspace-scoped app APIs and task app validation. Highest-risk delta was wiring `AppService` into `TaskService` without changing unrelated task rules.

## Evidence Rows

- AC-APP-04 | red: no app archive or restore endpoint existed | green: lifecycle endpoints and active-option filtering passed backend integration coverage | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-APP-05 | red: list, create, update, and detail app routes were absent | green: `app_management_api.rs` proves filters, duplicate-name conflict, create, update, archive, and restore behavior | command_ref: CMD-BE-TEST | browser:not applicable | status: done.
- AC-APP-06 | red: task API accepted non-registry app codes | green: `task_management_api.rs` proves unknown, archived, and foreign-workspace app codes return `TASK_APP_INVALID` | command_ref: CMD-BE-TEST | browser:not applicable | status: done.

## Changed Paths

- `tron-backend/src/domain/app.rs`
- `tron-backend/src/application/app.rs`
- `tron-backend/src/adapters/inbound/http/apps/*`
- `tron-backend/src/application/task.rs`
- `tron-backend/src/domain/task.rs`
- `tron-backend/tests/app_management_api.rs`
- `tron-backend/tests/task_management_api.rs`

## Commands

- @ref:spec/test-plan.md#command-index: `CMD-BE-TEST` passed.

## Blockers

- None.
