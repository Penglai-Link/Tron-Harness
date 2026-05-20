# Acceptance: Automations

## Acceptance Index

- AC-AUTO-01 [must] [api] Backend exposes automation and execution status options for frontend lifecycle rendering; proof: `rtk make be-test`.
- AC-AUTO-02 [must] [business] Automation list is workspace-scoped, searchable, filterable, paginated, and hides archived records by default; proof: `rtk make be-test`.
- AC-AUTO-03 [must] [business] Create validates active workspace, required long prompt, and cron syntax, then generates backend-owned automation codes; proof: `rtk make be-test`.
- AC-AUTO-04 [must] [business] Update, activate, pause, archive, and restore preserve lifecycle rules and never schedule non-Active automations; proof: `rtk make be-test`.
- AC-AUTO-05 [must] [business] Manual and scheduled execution both persist execution-instance history and roll up the latest execution result on the parent automation; proof: `rtk make be-test`.
- AC-AUTO-06 [must] [business] Backend startup reloads Active automations into the scheduler while Paused and Archived records remain unscheduled; proof: `rtk make be-test`.
- AC-AUTO-07 [must] [interaction] Frontend list, create, and detail flows follow the shell workspace selector and expose run-now, lifecycle actions, long prompt editing, and execution history; proof: Playwright automation workflow.
- AC-AUTO-08 [must] [regression] Unsupported import and sync affordances are absent from the automations UI while mock and API modes share the same contract; proof: `rtk make fe-check`.

## Open Decisions

- None.
