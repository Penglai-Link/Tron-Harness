# Database Task: User Management Model

## Execution Contract

- feature_slug: user-management-model
- status: ready
- canonical_report_target: reports/database-result.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DB-GREEN-01 | green | database | database-worker | TEST-RED-02, TEST-RED-06 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/database/schema-migrations | tron-backend/migrations/*; tron-backend/src/domain/user/repository/*; tron-backend/src/domain/user/schema/* | rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_lifecycle | AC-UM-02, AC-UM-06 | reports/database-result.md | ready |
| DB-VERIFY-01 | verify | test | verify-worker | DB-GREEN-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/user-management-model/test/verify-regression | read-only validation over migration files and repository schema assertions | rtk cargo test --manifest-path tron-backend/Cargo.toml user_management_schema_contracts | AC-UM-02, AC-UM-06 | reports/database-result.md | ready |

## Worktree Notes

Database implementation uses `feature/clouds3n/user-management-model/database/schema-migrations`. Verification evidence is consolidated from `feature/clouds3n/user-management-model/test/verify-regression`.

## Integration Notes

All schema/migration changes merge into `feature/clouds3n/user-management-model/base` before backend/frontend verify rows execute.

## Open Decisions

- None.
