# Test Plan: Automations

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-19

## Proof Rows

- AC-AUTO-01 | layer: api | case: options expose automation and execution statuses | red: options route is missing or omits one of the lifecycle enums | green: `/api/v1/automations/options` returns `Active`, `Paused`, `Archived` plus execution result statuses | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTO-02 | layer: api | case: workspace list stays scoped and hides archived by default | red: list ignores workspace or includes archived rows without explicit request | green: list queries honor `workspaceCode`, `search`, `status`, `current`, `size`, and archived filtering defaults | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTO-03 | layer: api | case: create validates workspace, prompt, and cron text | red: create accepts blank prompt, invalid workspace, or invalid cron text | green: valid create returns generated automation detail and invalid writes return stable automation errors | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTO-04 | layer: api | case: lifecycle transitions keep scheduler eligibility aligned with status | red: paused or archived records remain schedulable or archived records stay editable | green: activate registers, pause unregisters, archive unregisters, and restore returns previous status or `Paused` | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTO-05 | layer: api | case: execution history and latest-result rollup stay consistent | red: manual or scheduled runs fail to write history or parent latest-result fields | green: execute endpoints and scheduler callbacks create execution rows and update parent rollup fields consistently | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTO-06 | layer: api | case: backend startup reloads only Active automations into runtime scheduling | red: startup misses active records or reloads paused or archived records | green: service bootstrap registers persisted Active automations and leaves other statuses unscheduled | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-AUTO-07 | layer: browser | case: operator completes the workspace-scoped automation loop from list to execution history | red: browser cannot create or edit automation in the active workspace, lifecycle actions do not refresh detail, or execution history does not update after run-now | green: smoke proves workspace-scoped list, create, detail editing, activate or pause, run-now, execution history refresh, and long prompt persistence | interaction: select workspace -> open automations -> create automation -> save -> open detail -> edit prompt -> save -> run now -> inspect execution history -> pause or activate -> verify list badges | command_ref: CMD-AUTO-BROWSER.
- AC-AUTO-08 | layer: regression | case: unsupported import or sync affordances are removed without breaking mock or api builds | red: automations pages still expose import or sync controls or frontend typecheck breaks after contract alignment | green: no import or sync actions render on automations surfaces and frontend check passes in the unified contract | interaction: not applicable | command_ref: CMD-FE-CHECK.

## Command Index

- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify | rerun: after backend automation changes | expect: automation API tests and backend suite pass.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: verify/final | rerun: after backend automation changes | expect: backend compiles.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: verify/final | rerun: after frontend automation changes | expect: frontend typecheck and build pass.
- CMD-AUTO-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test tests/automation-management-smoke.spec.ts --reporter=line | cost: slow/browser | when: verify | rerun: review-if-stale | expect: concrete workspace-scoped automation clicks, inputs, network assertions, feedback, and execution-history evidence.
- CMD-DOCS-SPEC | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/automations --phase spec --complete-spec | cost: medium | when: spec | rerun: after spec edits | expect: automation spec docs validate.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/automations --phase final | cost: medium | when: final | rerun: after feature doc or report edits | expect: automation feature docs validate end to end.

## Evidence Rules

- Every acceptance row must have RED, GREEN, and VERIFY evidence in the final reports.
- Browser evidence must include concrete clicks, inputs, network assertions, visible feedback, and referenced artifacts.
- Slow browser verification runs once per relevant automation change and review reuses fresh passing evidence unless it becomes stale.
