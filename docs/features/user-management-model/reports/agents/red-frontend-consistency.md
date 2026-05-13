# TEST-RED-16 Frontend Consistency RED Report

## Agent Identity

- agent_id: test
- subtask_name: red-frontend-consistency
- branch: `feature/clouds3n/user-management-model/test/red-frontend-consistency`
- worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-frontend-consistency`

## Scope

- Task: `TEST-RED-16`
- Canonical task source: @ref:tasks/test-task.md#task-table
- Acceptance source: @ref:spec/acceptance.md#acceptance-criteria-matrix
- TDD source: @ref:spec/test-plan.md#tdd-cycle-table

## Task Results

| task id | result | notes |
| --- | --- | --- |
| TEST-RED-16 | fail_expected_red | Added frontend consistency RED coverage for statistic cards, directory layout, and create-page navigation. |

## TDD Evidence

| cycle id | acceptance ids | expected RED reason |
| --- | --- | --- |
| TDD-UM-16 | AC-UM-09, AC-UM-08 | Statistic cards do not call the four dedicated statistic endpoints. |
| TDD-UM-17 | AC-UM-10, AC-UM-08 | Directory layout lacks Contact and still contains old Space Links/User content. |
| TDD-UM-18 | AC-UM-10, AC-UM-02, AC-UM-08 | Add User does not navigate to `/admin/users/new`. |

## Changed Paths

- `tron-frontend/tests/user-management-smoke.spec.ts`
- `docs/features/user-management-model/reports/agents/red-frontend-consistency.md`

## RED Tests Added

| test title | acceptance ids | TDD cycle ids | expected RED reason |
| --- | --- | --- | --- |
| `consistency refactor TDD-UM-16: statistic cards call four APIs and render data.count` | AC-UM-09, AC-UM-08 | TDD-UM-16 | Fails because `/admin/users` does not call the four dedicated statistic endpoints and still derives card values from the current user page records. |
| `consistency refactor TDD-UM-17: directory renders User Contact columns and Space Assignments copy` | AC-UM-10, AC-UM-08 | TDD-UM-17 | Fails because the table has no `Contact` column, `User` still includes username/email, phone is not rendered in a contact cell, and the card still says `Space Links`. |
| `consistency refactor TDD-UM-18: Add User navigates to create page and submits from page owner` | AC-UM-10, AC-UM-02, AC-UM-08 | TDD-UM-18 | Fails because Add User remains on `/admin/users` and opens inline behavior instead of navigating to `/admin/users/new`. |

## Commands

| command | result | notes |
| --- | --- | --- |
| `rtk make -n` | pass | Makefile-first command discovery. |
| `rtk git diff --check` | pass | No whitespace errors. |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "consistency refactor"` | initial setup fail | Did not reach tests because this worktree was missing frontend dependencies: `Cannot find package '@playwright/test'`. |
| `rtk make fe-install` | pass | Installed frontend dependencies through the Makefile entry point. |
| `rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "consistency refactor"` | fail_expected_red | Three tests executed and failed on expected missing frontend behavior, not syntax, fixture, or setup failure. |

## RED Evidence Details

| cycle id | evidence |
| --- | --- |
| TDD-UM-16 | Focused Playwright run failed on missing `GET /api/v1/admin/users/statistics/active-users`; no statistic endpoint requests were observed. |
| TDD-UM-17 | Focused Playwright run failed because `Contact` was absent, `Space Assignments` was absent, `Space Links` remained visible, the `User` cell still contained username/email, and the second table cell still contained codes instead of email/phone. |
| TDD-UM-18 | Focused Playwright run failed because Add User left the URL at `/admin/users` instead of navigating to `/admin/users/new`. |

## Blockers

- None remaining for RED. The missing frontend dependency setup was resolved with `rtk make fe-install`.
