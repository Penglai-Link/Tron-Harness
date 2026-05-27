# Test Task: Agent Run History

## Execution Contract

- feature_slug: agent-run-history
- status: done
- execution_mode: solo

## TDD Contract

Verification rows reuse backend commands and inspect API JSON for safety.

## Task Rows

- AC-ARH-01 | phase: verify | owner: lead | scope: persisted summary regression | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-02 | phase: verify | owner: lead | scope: list/detail API regression | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-03 | phase: verify | owner: lead | scope: history masking regression | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-04 | phase: verify | owner: lead | scope: missing run error regression | command_ref: CMD-BE-TEST | status: done.
- AC-ARH-05 | phase: verify | owner: lead | scope: previous feature regression | command_ref: CMD-BE-TEST | status: done.

## Worktree Notes

- Same selected Lite Spec base worktree as feature1 and feature2.

## Integration Notes

- Full backend suite is the regression surface.

## Open Decisions

- None.
