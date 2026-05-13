# RED Worker Report: Statistics Consistency

## Identity

- agent_id: test
- subtask_name: red-statistics-consistency
- branch: `feature/clouds3n/user-management-model/test/red-statistics-consistency`
- worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-statistics-consistency`
- task: TEST-RED-15

## Scope

- Added backend RED coverage for `@ref:spec/test-plan.md#TDD-UM-14`.
- Added backend RED coverage for `@ref:spec/test-plan.md#TDD-UM-15`.
- Changed only TEST-RED-15 report/test assets.

## Changed Files

- `tron-backend/tests/user_management_statistics.rs`
- `docs/features/user-management-model/reports/agents/red-statistics-consistency.md`

## RED Evidence

| cycle id | acceptance ids | command | result | status |
| --- | --- | --- | --- | --- |
| TDD-UM-14 | AC-UM-09, AC-UM-11 | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_statistics` | Failed because all four statistic endpoints returned HTTP 404 instead of uniform `{ code, message, data: { count } }` success envelopes. | fail_expected_red |
| TDD-UM-15 | AC-UM-11, AC-UM-01, AC-UM-09 | `rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_statistics` | Focused list/default archive/search/includeArchived/pagination assertions executed; deterministic seed assertion failed because assignment seed rows are not yet larger than user count. | fail_expected_red |

## Failure Details

- `GET /api/v1/admin/users/statistics/total-users`: expected HTTP 200 with `data.count`; actual HTTP 404.
- `GET /api/v1/admin/users/statistics/active-users`: expected HTTP 200 with `data.count`; actual HTTP 404.
- `GET /api/v1/admin/users/statistics/role-assignments`: expected HTTP 200 with `data.count`; actual HTTP 404.
- `GET /api/v1/admin/users/statistics/space-assignments`: expected HTTP 200 with `data.count`; actual HTTP 404.
- Seed data assertion failed because role assignment rows (7) and space assignment rows (7) are not both larger than total users (7), proving assignment-count fixtures are not yet strong enough to distinguish assignment counts from user counts.

The failure is expected RED behavior. The command compiled and executed the new tests; failures are not syntax, fixture setup, dependency, or environment failures.

## Passing Assertions In Same RED Run

- Default list excludes archived users and `data.total` equals the active-user database count before pagination.
- `status=Archived&search=archived` returns archived persisted users only.
- `includeArchived=true` returns both active and archived users and total equals all persisted users.
- Pagination returns a first page smaller than `data.total`, proving total is not current page length.

## Blockers

- None for RED. GREEN must add statistic routes/count DTOs and expand deterministic assignment seed rows.
