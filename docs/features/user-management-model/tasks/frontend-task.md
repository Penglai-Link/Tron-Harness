# Frontend Task: User Management Model

## Execution Contract

- feature_slug: user-management-model
- status: in_progress
- canonical_report_target: reports/frontend-result.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FE-GREEN-01 | green | frontend | frontend-worker | TEST-RED-08, BE-GREEN-01, BE-GREEN-02, BE-GREEN-03 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/green-admin-users | tron-frontend/src/domain/admin/model/*; tron-frontend/src/domain/admin/api/*; tron-frontend/src/domain/admin/pages/UsersPage.tsx; tron-frontend/src/domain/admin/pages/UserDetailsPage.tsx; tron-frontend/src/domain/admin/mock/data.ts | rtk make fe-check | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | reports/frontend-result.md | done |
| FE-VERIFY-01 | verify | test | verify-worker | FE-GREEN-01, BE-VERIFY-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | tron-frontend read-only verification and smoke evidence collection | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | reports/frontend-result.md | done |
| FE-FIX-01 | green | frontend | frontend-fix-worker | TEST-RED-08, REVIEW-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/fix-detail-not-found | tron-frontend/src/domain/admin/pages/UserDetailsPage.tsx; tron-frontend/tests/user-management-smoke.spec.ts | rtk make fe-check | AC-UM-01, AC-UM-08 | reports/frontend-result.md | done |
| FE-GREEN-02 | green | frontend | frontend-interaction-gaps-worker | TEST-RED-09, TEST-RED-10, TEST-RED-11, TEST-RED-12, TEST-RED-13 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/green-interaction-gaps | tron-frontend/src/domain/admin/api/*; tron-frontend/src/domain/admin/hooks/useUserList.ts; tron-frontend/src/domain/admin/pages/UsersPage.tsx; tron-frontend/src/domain/admin/pages/UserDetailsPage.tsx; tron-frontend/src/domain/admin/model/*; tron-frontend/src/domain/admin/mock/data.ts; tron-frontend/tests/user-management-smoke.spec.ts | rtk make fe-check && rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-08 | reports/frontend-result.md | done |
| FE-VERIFY-02 | verify | test | verify-interaction-gaps-worker | FE-GREEN-02 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-interaction-gaps | tron-frontend read-only runtime verification and report evidence updates | rtk make fe-check && rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke | AC-UM-01, AC-UM-02, AC-UM-05, AC-UM-08 | reports/frontend-result.md | done |
| FE-FIX-02 | green | frontend | frontend-reset-contract-worker | TEST-RED-14, REVIEW-03 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/fix-reset-contract | tron-frontend/src/domain/admin/pages/UserDetailsPage.tsx; tron-frontend/tests/user-management-smoke.spec.ts | rtk make fe-check && rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "reset response contract" | AC-UM-05, AC-UM-08 | reports/frontend-result.md | done |
| FE-GREEN-03 | green | frontend | frontend-user-consistency-worker | TEST-RED-16, BE-GREEN-05 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/frontend/user-consistency-refactor | tron-frontend/src/domain/admin/api/*; tron-frontend/src/domain/admin/hooks/*; tron-frontend/src/domain/admin/pages/UsersPage.tsx; tron-frontend/src/domain/admin/pages/CreateUserPage.tsx; tron-frontend/src/domain/admin/model/*; tron-frontend/src/domain/admin/mock/data.ts; tron-frontend/src/router/routes.tsx; tron-frontend/src/router/pageRegistry.ts; tron-frontend/tests/user-management-smoke.spec.ts | rtk make fe-check && rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "consistency refactor" | AC-UM-09, AC-UM-10, AC-UM-02, AC-UM-08 | reports/frontend-result.md | ready |

## Worktree Notes

Frontend implementation uses `feature/clouds3n/user-management-model/frontend/green-admin-users` for retained model alignment and `feature/clouds3n/user-management-model/frontend/green-interaction-gaps` for the evolved interaction gaps. Browser/runtime verification uses `feature/clouds3n/user-management-model/test/verify-regression` for retained evidence and `feature/clouds3n/user-management-model/test/verify-interaction-gaps` for new network/download evidence.

## Integration Notes

Frontend GREEN row starts only after matching RED suite is merged and backend contract rows required by frontend are integrated. `FE-GREEN-02` starts only after `TEST-RED-09..13` are integrated into base.

## Open Decisions

- None.
