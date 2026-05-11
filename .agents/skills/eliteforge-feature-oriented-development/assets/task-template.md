# {{TASK_KIND}} Task: {{FEATURE_NAME}}

## Execution Contract

- feature_slug: {{FEATURE_SLUG}}
- status: draft
- canonical_report_target: {{REPORT_TARGET}}
- main_agent_owned: false

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

| id | tdd phase | owner | agent role | dependencies | worktree | write scope | validation | acceptance ids | report target | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |

## Worktree Notes

List required worktree branch, base ref, generated files, shared artifacts, and merge constraints for the rows in this task file. Worker paths must use `eliteforge/<agent-id>/<feature-slug>` order.

## Integration Notes

Record cross-team contracts, shared DTOs, route dependencies, migration ordering, fixtures, generated clients, and handoff notes.

## Open Decisions

- None.
