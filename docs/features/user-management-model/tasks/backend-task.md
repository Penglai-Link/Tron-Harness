# Backend Task: User Management Model

## Execution Contract

- feature_slug: user-management-model
- status: done
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| BE-GREEN-01 | green | backend | backend-worker | TEST-RED-01, TEST-RED-02 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/green-user-api | tron-backend/src/domain/user/*; tron-backend/src/http/admin/users/* | rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_list_detail | AC-UM-01, AC-UM-02 | reports/backend-result.md | done |
| BE-GREEN-02 | green | backend | backend-worker | TEST-RED-03, TEST-RED-04 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/green-user-api | tron-backend/src/domain/user/assignments/*; tron-backend/src/http/admin/users/* | rtk cargo test --manifest-path tron-backend/Cargo.toml user_role_assignments user_space_assignments | AC-UM-03, AC-UM-04 | reports/backend-result.md | done |
| BE-GREEN-03 | green | backend | backend-worker | TEST-RED-05 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/green-user-api | tron-backend/src/domain/user/password/*; tron-backend/src/http/admin/users/password_reset.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml password_reset | AC-UM-05 | reports/backend-result.md | done |
| BE-GREEN-04 | green | backend | backend-worker | TEST-RED-06, TEST-RED-07 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/green-user-api | tron-backend/src/http/admin/users/dto/*; tron-backend/src/http/admin/users/contract/* | rtk cargo test --manifest-path tron-backend/Cargo.toml stable_code_contracts user_api_authorization_boundary | AC-UM-06, AC-UM-07 | reports/backend-result.md | done |
| BE-VERIFY-01 | verify | test | verify-worker | BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | tron-backend read-only regression verification and report evidence updates | rtk make be-test | AC-UM-01, AC-UM-02, AC-UM-03, AC-UM-04, AC-UM-05, AC-UM-06, AC-UM-07 | reports/backend-result.md | done |
| BE-FIX-01 | green | backend | backend-fix-worker | TEST-RED-02, TEST-RED-05, REVIEW-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/fix-create-validation | tron-backend/src/domain/user/mod.rs; tron-backend/src/http/admin/users/mod.rs; tron-backend/tests/user_management_lifecycle.rs | rtk make be-test | AC-UM-02, AC-UM-05 | reports/backend-result.md | done |
| BE-FIX-02 | green | backend | backend-contract-fix-worker | TEST-RED-02, TEST-RED-06, REVIEW-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/backend/fix-status-contract | tron-backend/src/domain/user/mod.rs; tron-backend/src/http/admin/users/mod.rs; tron-backend/tests/user_management_lifecycle.rs | rtk make be-test | AC-UM-02, AC-UM-06 | reports/backend-result.md | done |

## Worktree Notes

Backend implementation and refactor rows use `feature/clouds3n/user-management-model/backend/green-user-api`. Verification rows use `feature/clouds3n/user-management-model/test/verify-regression`.

## Integration Notes

GREEN backend rows require merged RED evidence first. `BE-VERIFY-01` runs after database/backend/frontend green rows are integrated to base.

## Open Decisions

- None.
