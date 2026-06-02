# Review Task: Tool Registry

## Execution Contract

- feature_slug: tool-registry
- status: pending
- execution_mode: hybrid

## TDD Contract

Review focuses on behavioral regressions, runtime-policy correctness, and API/UI contract drift.

## Task Rows

- AC-TOOL-01 | phase: review | owner: lead | scope: integrated review of tool catalog correctness and API fit | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-TOOL-02 | phase: review | owner: lead | scope: integrated review of internal-agent policy persistence and regression safety | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-TOOL-03 | phase: review | owner: lead | scope: integrated review of runtime tool derivation and exposure behavior | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-TOOL-04 | phase: review | owner: lead | scope: integrated review of tools page interaction and data fidelity | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-TOOL-05 | phase: review | owner: lead | scope: integrated review of workspace agents page evolution and system-agent preservation | command_ref: CMD-DOCS-FINAL | status: ready.

## Worktree Notes

- Review occurs after tests and browser verification.

## Integration Notes

- Review must explicitly check that system-agent behaviors survive the `/agents` evolution.

## Open Decisions

- None.
