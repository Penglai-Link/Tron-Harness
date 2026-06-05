# Test Plan: Task Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-19

## Proof Rows

- AC-TASK-01 | layer: api | case: options include the final task types, status universe, role-based `statusRules`, and `defaultStatuses` | red: `/api/v1/tasks/options` is missing or exposes old enum values | green: the options request returns `Feature`, `Bug`, the superpower status enum, and role-keyed rules/defaults | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-02 | layer: api | case: kanban and list remain workspace scoped, searchable, filterable, and hide archived by default | red: task routes return stale schema data or old seeded tasks | green: persisted workspace tasks are returned through list and `/api/v1/tasks/kanban`; archived tasks require explicit archived filter or `includeArchived=true` | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-03 | layer: api | case: create infers the task role and assigns backend-owned default status `Open` | red: create keeps old default statuses or accepts invalid assignees/workspaces | green: valid create persists detail with `Open` and invalid workspace/assignee writes return stable errors | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-04 | layer: api | case: detail and same-workspace subtask creation continue to honor parent-child rules under the new role model | red: detail and parent validation are missing or archived subtasks still leak into parent detail/statistics | green: same requests return parent and active-child records, exclude archived subtasks from parent detail/statistics, and reject cross-workspace parent | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-05 | layer: api | case: role-aware status updates and task-type edits reject incompatible combinations | red: status route still accepts any non-archived status or task type can change without validating the current status | green: invalid status/role combinations return `TASK_STATUS_ROLE_INVALID`, valid updates still force completed progress and keep archived mutation denial | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-06 | layer: api | case: archive/restore lifecycle preserves previous status and falls back to `Open` | red: archive route is missing, parent active-subtask guard regresses, or restore still falls back to `Backlog` | green: archive stores previous status, blocks active-subtask parents, default list hides archived, and restore returns previous status or `Open` | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-07 | layer: browser | case: frontend task workflow reflects role-aware types and statuses | red: browser still sees `BugFix`, `Refactor`, `Backlog`, `Todo`, or `Testing`, or role-filtered dropdowns do not match task role | green: smoke proves debounced board search, create with `Feature | Bug`, route transition into detail after create, role-filtered detail statuses, parent-child navigation, child type inheritance, archive guard, archive/restore, and Execute Toast | interaction: preload the persisted shell selection as `TRON_CORE` for board and detail routes -> open `/tasks` -> search -> priority filter -> create -> confirm detail route transition -> inspect role-filtered detail status -> edit -> save -> add subtask -> archive blocked -> jump child -> verify child statuses -> archive child -> jump parent -> archive -> restore -> execute Toast | command_ref: CMD-TASK-BROWSER.
- AC-TASK-08 | layer: persistence | case: backend legacy task data is reset while frontend mock/demo remains available under the new contract | red: task repository still seeds old tasks or old schema data survives the reset path | green: backend tests start from empty task tables with no seed records, legacy schema reset path is covered, and frontend check/mock paths build with the new contract | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-09 | layer: browser | case: kanban drag/drop persists valid cross-status moves and blocks invalid moves | red: task cards cannot be dragged to another status column, or drops do not call `POST /api/v1/tasks/{taskCode}/status` with `{ status }` | green: smoke proves a valid drop updates the board through the status API, invalid role drops do not call the status API and show feedback, same-column drops are ignored, failure drops restore the previous board, and card click navigation still works | interaction: open `/tasks` with `TRON_CORE` shell selection -> drag a Feature root card from Open to Review -> assert POST status body -> verify visible card move -> attempt a role-invalid drop -> assert no status POST and toast -> simulate status update failure -> assert rollback and backend error toast -> click a card and verify detail navigation | command_ref: CMD-TASK-BROWSER.

## Command Index

- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify | rerun: after backend task changes | expect: task API tests and full backend suite pass.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: verify/final | rerun: after backend changes | expect: backend compiles.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: verify/final | rerun: after frontend changes | expect: frontend typecheck and build pass.
- CMD-TASK-BROWSER | command: make fe-test TEST='task-management-smoke.spec.ts' | cost: slow/browser | when: red/green/verify | rerun: after task board interaction changes | expect: concrete clicks, inputs, drag/drop interactions, role-aware task API network assertions, and visible Toast evidence.
- CMD-DOCS-SPEC | command: python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/task-management --phase spec --complete-spec | cost: medium | when: spec | rerun: after spec edits | expect: spec feature docs validate.
- CMD-DOCS-FINAL | command: python3 /Users/langtaoxin/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/task-management --phase final | cost: medium | when: final | rerun: after feature doc/report edits | expect: final feature docs validate.

## Evidence Rules

- Every acceptance row must have RED/GREEN/VERIFY evidence in the matching report.
- Browser evidence must record clicks, inputs, network assertions, feedback, and artifacts.
- Slow browser verification runs once per relevant change and review reuses fresh passing evidence unless stale.
