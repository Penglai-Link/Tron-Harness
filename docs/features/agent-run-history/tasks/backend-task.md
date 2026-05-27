# Backend Task: Agent Run History

## Execution Contract

- feature_slug: agent-run-history
- status: done
- execution_mode: solo

## TDD Contract

Backend rows use bounded AC slices with RED before GREEN and focused VERIFY. Lead executes on the selected Lite Spec base worktree because the user approved Codex-owned decisions and current subagent tool policy does not permit implementation workers without an explicit subagent request.

## Task Rows

- AC-ARH-01 | phase: tdd | owner: lead | scope: SQLite schema, repository, invocation persistence, and success test | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-02 | phase: tdd | owner: lead | scope: protected list/detail routes and response DTOs | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-03 | phase: tdd | owner: lead | scope: safe preview and response redaction assertions | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-04 | phase: tdd | owner: lead | scope: missing run error mapping | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Lead worktree: `/Users/clouds3n/.codex/worktrees/Tron/feature/clouds3n/agent-framework/base`.
- Branch: `feature/clouds3n/agent-framework/base`.

## Integration Notes

- Invocation API must keep working while adding history writes.
- Runtime kernel must remain persistence-agnostic.

## Open Decisions

- None.
