# Frontend Task: Common Snowflake ID Generation

## Execution Contract

- feature_slug: common-snowflake-id
- status: not_applicable
- canonical_report_target: reports/frontend-result.md
- main_agent_owned: true

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FE-DOC-01 | docs | lead | lead | BE-VERIFY-01 | /Users/clouds3n/Coding/open-source/ai/Tron | docs/features/common-snowflake-id/reports/frontend-result.md | rtk make fe-check if frontend DTO or generated client files are touched | AC-SNOWFLAKEID-04 | reports/frontend-result.md | not_applicable |

## Worktree Notes

No frontend worker worktree is required unless implementation unexpectedly touches frontend DTO or generated client files. This feature introduces no page, route, browser journey, or visible frontend state.

## Integration Notes

If frontend code changes later, `make fe-check` becomes mandatory before final review. Otherwise frontend reporting should record that frontend work is not applicable and DTO precision is covered by backend serialization tests.

## Open Decisions

- None.
