# Test Task: Common Snowflake ID Generation

## Execution Contract

- feature_slug: common-snowflake-id
- status: done
- canonical_report_target: reports/test-report.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TEST-RED-01 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_id_provider.rs; tron-backend/src/shared/id test-only fixtures only if needed | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_id_provider | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-03 | reports/test-report.md | done |
| TEST-RED-02 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_config.rs; tron-backend/tests/fixtures/snowflake/valid.yaml | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_valid | AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03 | reports/test-report.md | done |
| TEST-RED-03 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/fixed_snowflake_id_provider.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml fixed_snowflake_id_provider | AC-SNOWFLAKEID-06, AC-SNOWFLAKEID-01 | reports/test-report.md | done |
| TEST-RED-04 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_serialization.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization_string | AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-01 | reports/test-report.md | done |
| TEST-RED-05 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/common_snowflake_consumer_boundary.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml common_snowflake_consumer_boundary | AC-SNOWFLAKEID-07, AC-SNOWFLAKEID-01 | reports/test-report.md | done |
| TEST-RED-06 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_config.rs; tron-backend/tests/fixtures/snowflake/missing-instance-id.yaml | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_missing_instance_id | AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03 | reports/test-report.md | done |
| TEST-RED-07 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_config.rs; tron-backend/tests/fixtures/snowflake/invalid-instance-id.yaml | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_invalid_instance_id | AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03 | reports/test-report.md | done |
| TEST-RED-08 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_config.rs; tron-backend/tests/fixtures/snowflake/invalid-epoch.yaml | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_config_invalid_epoch | AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03 | reports/test-report.md | done |
| TEST-RED-09 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflaked_adapter_error_mapping.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflaked_adapter_error_mapping | AC-SNOWFLAKEID-03, AC-SNOWFLAKEID-01 | reports/test-report.md | done |
| TEST-RED-10 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_serialization.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization_invalid_string | AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-01 | reports/test-report.md | done |
| TEST-RED-11 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_serialization.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_serialization_large_i64 | AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-05 | reports/test-report.md | done |
| TEST-RED-12 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/snowflake_no_database_objects.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml snowflake_no_database_objects | AC-SNOWFLAKEID-05, AC-SNOWFLAKEID-02 | reports/test-report.md | done |
| TEST-RED-13 | red | test | red-test-worker | BE-SUPPORT-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite | tron-backend/tests/common_snowflake_import_boundary.rs | rtk cargo test --manifest-path tron-backend/Cargo.toml common_snowflake_import_boundary | AC-SNOWFLAKEID-07, AC-SNOWFLAKEID-03 | reports/test-report.md | done |

## Worktree Notes

RED work uses branch `feature/clouds3n/common-snowflake-id/test/red-suite` and worktree `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/test/red-suite`. The RED worker may create or update tests, fixtures, and test-only helpers, but must not implement production Snowflake behavior.

## Integration Notes

The lead agent must integrate valid RED failures into `feature/clouds3n/common-snowflake-id/base` before any dependent GREEN task starts. A valid RED failure must be the expected missing behavior, not a broken Cargo workspace, syntax error, missing dependency install, or invalid test harness.

## Open Decisions

- None.
