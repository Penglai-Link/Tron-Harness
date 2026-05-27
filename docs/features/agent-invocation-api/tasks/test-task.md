# Test Task: Agent Invocation API

## Execution Contract

- feature_slug: agent-invocation-api
- status: done
- execution_mode: solo

## TDD Contract

Test rows verify API, backend, frontend build, and browser smoke evidence.

## Task Rows

- AC-AIA-01 | phase: verify | owner: lead | scope: successful invocation API regression | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-02 | phase: verify | owner: lead | scope: auth and invalid input regression | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-03 | phase: verify | owner: lead | scope: frozen snapshot regression | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-04 | phase: verify | owner: lead | scope: runtime error and secret-safety regression | command_ref: CMD-BE-TEST | status: done.
- AC-AIA-05 | phase: verify | owner: lead | scope: frontend build and browser smoke regression | command_ref: CMD-FE-CHECK | status: done.

## Worktree Notes

- Backend and frontend validation run from the selected base worktree.

## Integration Notes

- Browser evidence is required for AC-AIA-05.

## Open Decisions

- None.
