# Test Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: pending
- execution_mode: hybrid

## TDD Contract

Verification consolidates backend suite, frontend build, and browser smoke after implementation integration.

## Task Rows

- AC-TOOL-01 | phase: verify | owner: lead | scope: backend compilation and catalog tests | command_ref: CMD-TOOL-BE-TEST | status: ready.
- AC-TOOL-02 | phase: verify | owner: lead | scope: backend policy and system-agent regression tests | command_ref: CMD-TOOL-BE-TEST | status: ready.
- AC-TOOL-03 | phase: verify | owner: lead | scope: runtime invocation and materialization tests | command_ref: CMD-TOOL-BE-TEST | status: ready.
- AC-TOOL-04 | phase: verify | owner: lead | scope: tools page frontend build and browser smoke | command_ref: CMD-TOOL-BROWSER | status: ready.
- AC-TOOL-05 | phase: verify | owner: lead | scope: workspace agents frontend build and browser smoke | command_ref: CMD-TOOL-BROWSER | status: ready.

## Worktree Notes

- Verification runs on `feature/clouds3n/tool-registry/base` after integration.

## Integration Notes

- Update the browser command if the smoke coverage is split into dedicated specs during implementation.

## Open Decisions

- None.
