# Backend Task: Common Snowflake ID Generation

## Execution Contract

- feature_slug: common-snowflake-id
- status: in_progress
- canonical_report_target: reports/backend-result.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| BE-SUPPORT-01 | support | backend | backend-worker | none | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/bootstrap-harness | tron-backend/Cargo.toml; tron-backend/src/lib.rs; tron-backend/src/shared/mod.rs; tron-backend/src/shared/id/mod.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml --no-run | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03, AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-06, AC-SNOWFLAKEID-07 | reports/backend-result.md | done |
| BE-GREEN-01 | green | backend | backend-worker | TEST-RED-01, TEST-RED-09 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/green-implementation | tron-backend/src/shared/id/mod.rs; tron-backend/src/shared/id/snowflake.rs; tron-backend/Cargo.toml | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_id_provider | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-03 | reports/backend-result.md | done |
| BE-GREEN-02 | green | backend | backend-worker | TEST-RED-02, TEST-RED-06, TEST-RED-07, TEST-RED-08 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/green-implementation | tron-backend/src/shared/id/config.rs; tron-backend/src/shared/id/mod.rs; tron-backend/Cargo.toml | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config | AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03 | reports/backend-result.md | done |
| BE-GREEN-03 | green | backend | backend-worker | TEST-RED-03 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/green-implementation | tron-backend/src/shared/id/testing.rs; tron-backend/src/shared/id/mod.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml fixed_snowflake_id_provider | AC-SNOWFLAKEID-06, AC-SNOWFLAKEID-01 | reports/backend-result.md | done |
| BE-GREEN-04 | green | backend | backend-worker | TEST-RED-04, TEST-RED-10, TEST-RED-11 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/green-implementation | tron-backend/src/shared/id/serialization.rs; tron-backend/src/shared/id/mod.rs; tron-backend/src/shared/id/persistence_policy.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization | AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-05 | reports/backend-result.md | done |
| BE-GREEN-05 | green | backend | backend-worker | TEST-RED-05, TEST-RED-13 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/backend/green-implementation | tron-backend/src/shared/id/mod.rs; tron-backend/src/shared/mod.rs; tron-backend/src/lib.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml common_snowflake | AC-SNOWFLAKEID-07, AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-03 | reports/backend-result.md | done |
| BE-VERIFY-01 | verify | test | verify-worker | BE-GREEN-01, BE-GREEN-02, BE-GREEN-03, BE-GREEN-04, BE-GREEN-05 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/verify-regression | tron-backend read-only verification; docs/features/common-snowflake-id/reports/agents report fragment | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03, AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-06, AC-SNOWFLAKEID-07 | reports/backend-result.md | ready |

## Worktree Notes

`BE-SUPPORT-01` creates only the minimal backend Cargo harness needed for RED tests to compile and fail for missing Snowflake behavior. `backend/green-implementation` owns production shared ID implementation. `test/verify-regression` is read-mostly and records regression evidence after integration.

## Integration Notes

GREEN implementation starts only after matching RED rows are integrated. The production dependency must be `snowflaked = { version = "1.0.3", features = ["sync"] }`, and direct crate usage must stay inside `tron-backend/src/shared/id`.

## Open Decisions

- None.
