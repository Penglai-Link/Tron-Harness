# Review Task: Automations

## Execution Contract

- owner: lead
- scope: final code-path review, docs validation, and residual-risk closure
- mode: solo

## TDD Contract

- Review reuses fresh verify evidence unless backend or frontend behavior changes after verification.

## Task Rows

- AC-AUTO-01 | phase: review | owner: lead | scope: review options contract and error envelope alignment | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-02 | phase: review | owner: lead | scope: review workspace scoping and archived defaults | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-03 | phase: review | owner: lead | scope: review create validation and cron normalization path | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-04 | phase: review | owner: lead | scope: review lifecycle to scheduler consistency | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-05 | phase: review | owner: lead | scope: review execution-history and rollup consistency | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-06 | phase: review | owner: lead | scope: review startup bootstrap and Active-only registration | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-07 | phase: review | owner: lead | scope: review browser evidence and workspace-bound operator flow | command_ref: CMD-DOCS-FINAL | status: ready.
- AC-AUTO-08 | phase: review | owner: lead | scope: review unsupported affordance removal and contract parity | command_ref: CMD-DOCS-FINAL | status: ready.

## Worktree Notes

- No delegated review worktree is created because current developer rules do not allow subagents without an explicit user request.

## Integration Notes

- Final docs validation runs only after task statuses, orchestration notes, and report evidence are all closed.

## Open Decisions

- None.
