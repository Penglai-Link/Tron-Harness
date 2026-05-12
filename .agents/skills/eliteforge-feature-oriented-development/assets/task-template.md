# {{TASK_KIND}} Task: {{FEATURE_NAME}}

## Execution Contract

- feature_slug: {{FEATURE_SLUG}}
- status: draft

## TDD Contract

Every behavior-changing task must be split into RED, GREEN, and optional REFACTOR/VERIFY rows. RED rows must create or update tests before implementation. GREEN rows must depend on matching RED rows. Subagents work only in their assigned git worktree.

## Task Table

Keep this table small. Worktree paths, branches, validation commands, and report targets belong in `orchestration/worktree-plan.md`, `spec/test-plan.md`, and the matching report file.

| id | phase | owner | depends on | scope | acceptance ids | status |
| --- | --- | --- | --- | --- | --- | --- |

## Worktree Notes

List only execution notes that are not obvious from the table: suggested worker role, generated files, shared artifacts, merge constraints, or sequencing caveats. Actual worker branches and paths are assigned in `orchestration/worktree-plan.md`; do not duplicate them in every task row.

## Integration Notes

Record cross-team contracts, shared DTOs, route dependencies, migration ordering, fixtures, generated clients, and handoff notes.

## Open Decisions

- None.
