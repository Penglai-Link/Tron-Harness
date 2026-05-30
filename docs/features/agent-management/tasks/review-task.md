# Review Task: Agent Management

## Execution Contract

- feature_slug: agent-management
- status: complete
- execution_mode: hybrid
- canonical_report_target: reports/review-report.md
- main_agent_owned: true

## TDD Contract

Final review reused the integrated verification evidence and focused on contract drift, real API mismatches, and workspace-policy semantics.

## Task Rows

- AC-AGENT-01 | phase: review | owner: lead | scope: backend runtime and persistence integration | command_ref: CMD-BE-CHECK | status: done.
- AC-AGENT-02 | phase: review | owner: lead | scope: scan lifecycle, policy retention, and unavailable-state behavior | command_ref: CMD-AGENT-BE-TEST | status: done.
- AC-AGENT-03 | phase: review | owner: lead | scope: workspace-effective semantics and platform dominance | command_ref: CMD-AGENT-BE-TEST | status: done.
- AC-AGENT-04 | phase: review | owner: lead | scope: platform and workspace UI interactions, route wiring, and test coverage | command_ref: CMD-AGENT-BROWSER | status: done.

## Worktree Notes

- Final review completed on the integrated base worktree only.

## Integration Notes

- Review must treat hidden `Main Agent` on `/agents` as a hard requirement, not a cosmetic choice.

## Open Decisions

- None.
