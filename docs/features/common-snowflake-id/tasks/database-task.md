# Database Task: Common Snowflake ID Generation

## Execution Contract

- feature_slug: common-snowflake-id
- status: not_applicable
- canonical_report_target: reports/database-result.md
- main_agent_owned: true

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DB-DOC-01 | docs | lead | lead | TEST-RED-12 | /Users/clouds3n/Coding/open-source/ai/Tron | docs/features/common-snowflake-id/spec/database-design.md; docs/features/common-snowflake-id/reports/database-result.md | rtk proxy sh -c 'skill_dir="/Users/clouds3n/Coding/open-source/ai/Tron/.agents/skills/eliteforge-feature-oriented-development"; "$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase spec --spec-stage database' | AC-SNOWFLAKEID-05 | reports/database-result.md | not_applicable |

## Worktree Notes

No database worker worktree is required because Phase 1B approved `impact: no`. The RED no-database-object check lives in `tasks/test-task.md`; final reporting must record that no table, migration, generated-ID tracking table, or instance-allocation table was created.

## Integration Notes

Consuming features own future SQLite migration tests for their own tables. This common capability owns only the persistence convention that Snowflake-backed internal IDs fit SQLite signed `INTEGER`.

## Open Decisions

- None.
