# Frontend Task: User Management Model

## Execution Contract

- feature_slug: user-management-model
- status: ready
- canonical_report_target: reports/frontend-result.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FE-GREEN-01 | green | frontend | frontend-worker | TEST-RED-08, BE-GREEN-01, BE-GREEN-02, BE-GREEN-03 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/green-admin-users | tron-frontend/src/domain/admin/model/*; tron-frontend/src/domain/admin/api/*; tron-frontend/src/domain/admin/pages/UsersPage.tsx; tron-frontend/src/domain/admin/pages/UserDetailsPage.tsx; tron-frontend/src/domain/admin/mock/data.ts | rtk make fe-check | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | reports/frontend-result.md | ready |
| FE-VERIFY-01 | verify | test | verify-worker | FE-GREEN-01, BE-VERIFY-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | tron-frontend read-only verification and smoke evidence collection | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | reports/frontend-result.md | ready |

## Worktree Notes

Frontend implementation uses `feature/clouds3n/user-management-model/frontend/green-admin-users`. Browser/runtime verification is consolidated in `feature/clouds3n/user-management-model/test/verify-regression`.

## Integration Notes

Frontend GREEN row starts only after matching RED suite is merged and backend contract rows required by frontend are integrated.

## Open Decisions

- None.
