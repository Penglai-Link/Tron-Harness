# Review Task: EliteForge Memory Dreaming Script Foundation

## Execution Contract

- feature_slug: eliteforge-memory-dreaming-script-foundation
- status: ready
- execution_mode: hybrid
- canonical_report_target: reports/review-report.md

## TDD Contract

Review checks AC coverage, task evidence, changed paths, validation output, artifact-root semantics, and runtime-loading isolation after implementation workers are integrated.

## Task Rows

- AC-MDSF-01 | phase: review | owner: lead | scope: skill scaffold validity and runtime-loaded path isolation evidence | command_ref: none | status: done.
- AC-MDSF-02 | phase: review | owner: lead | scope: artifact root default path and `--artifact-root` override evidence | command_ref: none | status: done.
- AC-MDSF-03 | phase: review | owner: lead | scope: Codex fixture import and stdout privacy evidence | command_ref: none | status: done.
- AC-MDSF-04 | phase: review | owner: lead | scope: scan decision evidence | command_ref: none | status: done.
- AC-MDSF-05 | phase: review | owner: lead | scope: V1 task state machine evidence | command_ref: none | status: done.
- AC-MDSF-06 | phase: review | owner: lead | scope: temp output validation and promotion evidence | command_ref: none | status: done.
- AC-MDSF-07 | phase: review | owner: lead | scope: REM/Deep dry-run evidence without Memory MCP and no-session Deep no-op boundary | command_ref: none | status: done.
- AC-MDSF-08 | phase: review | owner: lead | scope: status and finalization evidence | command_ref: none | status: done.
- AC-MDSF-09 | phase: review | owner: lead | scope: Makefile-first validation evidence | command_ref: none | status: done.

## Worktree Notes

- Review stays in the selected base worktree after worker branches are merged.
- Review must not claim browser, backend API, database, or Memory MCP evidence for this feature.

## Integration Notes

- Canonical reports must reference RED, GREEN, and VERIFY output by AC.
- Runtime artifacts such as `.intentforge-dreaming/`, `/tmp/eliteforge-memory-dreaming`, and temp outputs must remain uncommitted.

## Open Decisions

- None.
