# Test Task: Automations

## Execution Contract

- owner: lead
- scope: backend regression tests, frontend smoke coverage, and final verification commands
- mode: solo

## TDD Contract

- RED evidence comes from backend API tests and the first failing automation smoke test.
- GREEN evidence comes from passing focused automation tests before broader check commands.

## Task Rows

- AC-AUTO-01 | phase: verify | owner: lead | scope: options endpoint verification | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-02 | phase: verify | owner: lead | scope: list filtering and pagination verification | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-03 | phase: verify | owner: lead | scope: create validation verification | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-04 | phase: verify | owner: lead | scope: lifecycle and scheduler eligibility verification | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-05 | phase: verify | owner: lead | scope: execution-history and latest-result verification | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-06 | phase: verify | owner: lead | scope: scheduler bootstrap verification | command_ref: CMD-BE-TEST | status: ready.
- AC-AUTO-07 | phase: verify | owner: lead | scope: browser automation workflow verification | command_ref: CMD-AUTO-BROWSER | status: ready.
- AC-AUTO-08 | phase: verify | owner: lead | scope: frontend contract and affordance regression verification | command_ref: CMD-FE-CHECK | status: ready.

## Worktree Notes

- Verification runs reuse the same solo worktree and fresh local services.

## Integration Notes

- Slow browser evidence is collected once after relevant UI behavior stabilizes and then reused for review unless stale.

## Open Decisions

- None.
