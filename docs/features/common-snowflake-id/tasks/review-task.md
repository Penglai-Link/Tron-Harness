# Review Task: Common Snowflake ID Generation

## Execution Contract

- feature_slug: common-snowflake-id
- status: ready
- canonical_report_target: reports/review-report.md
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| REVIEW-01 | review | review | review-worker | BE-VERIFY-01, DB-DOC-01, FE-DOC-01 | /Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/review/final-review | docs/features/common-snowflake-id/reports/review-report.md; read-only review of tron-backend/src/shared/id and feature docs | rtk proxy sh -c 'skill_dir="/Users/clouds3n/Coding/open-source/ai/Tron/.agents/skills/eliteforge-feature-oriented-development"; "$skill_dir/scripts/validate_feature_docs.py" docs/features/common-snowflake-id --phase final' | AC-SNOWFLAKEID-01, AC-SNOWFLAKEID-02, AC-SNOWFLAKEID-03, AC-SNOWFLAKEID-04, AC-SNOWFLAKEID-05, AC-SNOWFLAKEID-06, AC-SNOWFLAKEID-07 | reports/review-report.md | ready |

## Worktree Notes

Review uses branch `feature/clouds3n/common-snowflake-id/review/final-review` and worktree `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/common-snowflake-id/review/final-review`. The worker is read-mostly and should only write its report fragment unless the lead explicitly authorizes small documentation corrections.

## Integration Notes

Final review must compare acceptance criteria, TDD cycles, RED/GREEN evidence, no-database-impact evidence, import-boundary scan, worktree integration evidence, and residual risks.

## Open Decisions

- None.
