# Test Task: User Management Model

## Execution Contract

- feature_slug: user-management-model
- status: in_progress
- canonical_report_target: reports/test-report.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TEST-RED-01 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/user_management_list_detail.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_list_detail | AC-UM-01 | reports/test-report.md | done |
| TEST-RED-02 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/user_management_lifecycle.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_lifecycle | AC-UM-02 | reports/test-report.md | done |
| TEST-RED-03 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/user_role_assignments.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml user_role_assignments | AC-UM-03 | reports/test-report.md | done |
| TEST-RED-04 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/user_space_assignments.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml user_space_assignments | AC-UM-04 | reports/test-report.md | done |
| TEST-RED-05 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/password_reset.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml password_reset | AC-UM-05 | reports/test-report.md | done |
| TEST-RED-06 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/stable_code_contracts.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml stable_code_contracts | AC-UM-06 | reports/test-report.md | done |
| TEST-RED-07 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-backend/tests/user_api_authorization_boundary.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml user_api_authorization_boundary | AC-UM-07 | reports/test-report.md | done |
| TEST-RED-08 | red | test | red-test-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-suite | tron-frontend/tests/user-management-smoke.spec.ts; tron-frontend/src/domain/admin/mock/data.ts | rtk make fe-check && rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke | AC-UM-08 | reports/test-report.md | done |
| TEST-RED-09 | red | test | red-interaction-gaps-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-interaction-gaps | tron-frontend/tests/user-management-smoke.spec.ts | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "directory query parameters" | AC-UM-01, AC-UM-08 | reports/test-report.md | done |
| TEST-RED-10 | red | test | red-interaction-gaps-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-interaction-gaps | tron-frontend/tests/user-management-smoke.spec.ts | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "add user" | AC-UM-02, AC-UM-08 | reports/test-report.md | done |
| TEST-RED-11 | red | test | red-interaction-gaps-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-interaction-gaps | tron-frontend/tests/user-management-smoke.spec.ts | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "export users" | AC-UM-01, AC-UM-08 | reports/test-report.md | done |
| TEST-RED-12 | red | test | red-interaction-gaps-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-interaction-gaps | tron-frontend/tests/user-management-smoke.spec.ts | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "reset password" | AC-UM-05, AC-UM-08 | reports/test-report.md | done |
| TEST-RED-13 | red | test | red-interaction-gaps-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/red-interaction-gaps | tron-frontend/tests/user-management-smoke.spec.ts | rtk npm --prefix tron-frontend exec playwright -- test user-management-smoke --grep "detail mutations" | AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-08 | reports/test-report.md | done |
| TEST-VERIFY-01 | verify | test | verify-worker | DB-GREEN-01, BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04, FE-GREEN-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | consolidated regression verification for backend and frontend suites | rtk make be-test && rtk make fe-check | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-06, AC-UM-07, AC-UM-08 | reports/test-report.md | done |

## Worktree Notes

RED coverage uses `feature/clouds3n/user-management-model/test/red-suite` for retained initial coverage and `feature/clouds3n/user-management-model/test/red-interaction-gaps` for the evolved frontend interaction gaps. Cross-stack verify evidence uses `feature/clouds3n/user-management-model/test/verify-regression`.

## Integration Notes

All RED rows must merge into base before any dependent GREEN row starts. `TEST-RED-09..13` must fail for the expected current interaction gaps before `FE-GREEN-02` starts. `TEST-VERIFY-01` is the retained consolidated regression gate; the evolution adds `FE-VERIFY-02` before `REVIEW-03`.

## Open Decisions

- None.
