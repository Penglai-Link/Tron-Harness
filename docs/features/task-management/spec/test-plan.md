# Test Plan: Task Management

## Test Plan Decision

- status: approved_with_defaults
- approved_by: clouds3n
- approved_at: 2026-05-16

## Proof Rows

- AC-TASK-01 | layer: api | case: options include backend-owned v1 enums | red: `/api/v1/tasks/options` is missing or exposes stale enums | green: same request returns `Backlog`, `Todo`, `InProgress`, `Testing`, `Review`, `Reopen`, `Completed`, `Archived`, `Low`, `Medium`, `High`, `Feature`, `BugFix`, `Refactor`, `Internal`, `GitHub`, `GitLab` | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-02 | layer: api | case: kanban and list are workspace scoped, searchable, filterable, and hide archived by default | red: task routes return 404 or mock-only data | green: persisted workspace tasks are returned through list and `/api/v1/tasks/kanban`; archived tasks require explicit archived filter or `includeArchived=true` | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-03 | layer: api | case: create root task validates workspace, effective assignee, taskCode generation, optional source URL, and agent/app string links | red: create route is missing or rejects valid links/source fields | green: valid create persists detail and invalid workspace/assignee writes return stable errors | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-04 | layer: api | case: detail and same-workspace subtask creation through `parentTaskCode` | red: detail and parent validation are missing | green: same requests return parent and active-child records, exclude archived subtasks from parent detail/statistics, and reject cross-workspace parent | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-05 | layer: api | case: update editable fields plus status/progress lifecycle values | red: patch route or status route is missing | green: PATCH updates fields/type/source/links and status update supports `Testing`, `Reopen`, and `Completed` progress forcing | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-06 | layer: api | case: archive/restore lifecycle | red: archive route is missing or archived tasks remain in default list | green: archive stores previous status, blocks normal mutation, refuses parent archive while active subtasks exist, default list hides archived, and restore returns previous status | interaction: not applicable | command_ref: CMD-BE-TEST.
- AC-TASK-07 | layer: browser | case: frontend task workflow | red: browser cannot prove API-backed task interactions | green: smoke proves workspace selector, board search, priority filter, create, read-only detail without tags, edit/save including status/progress, active-subtask archive guard, parent-child navigation, subtask, archive/restore, and Execute Toast | interaction: select workspace -> search -> priority filter -> create -> inspect read-only detail -> edit -> save fields/status/progress -> add subtask -> archive blocked -> jump child -> archive child -> jump parent -> archive -> restore -> execute Toast | command_ref: CMD-TASK-BROWSER.
- AC-TASK-08 | layer: frontend | case: import/export entry points are absent from v1 task board | red: board exposes import/export controls | green: board actions only expose refresh and create | interaction: inspect board actions during smoke | command_ref: CMD-FE-CHECK.

## Command Index

- CMD-BE-TEST | command: rtk make be-test | cost: medium | when: red/green/verify | rerun: after backend task changes | expect: task API tests and full backend suite pass.
- CMD-BE-CHECK | command: rtk make be-check | cost: medium | when: verify/final | rerun: after backend changes | expect: backend compiles.
- CMD-FE-CHECK | command: rtk make fe-check | cost: medium | when: verify/final | rerun: after frontend changes | expect: frontend typecheck and build pass.
- CMD-TASK-BROWSER | command: rtk npm --prefix tron-frontend exec playwright -- test tests/task-management-smoke.spec.ts --reporter=line | cost: slow/browser | when: verify | rerun: review-if-stale | expect: concrete clicks, inputs, task API network assertions, and visible Toast evidence.
- CMD-DOCS-SPEC | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/task-management --phase spec --complete-spec | cost: medium | when: spec | rerun: after spec edits | expect: spec feature docs validate.
- CMD-DOCS-FINAL | command: rtk python3 /Users/clouds3n/.agents/skills/eliteforge-codex-superpower/scripts/validate_feature_docs.py docs/features/task-management --phase final | cost: medium | when: final | rerun: after feature doc/report edits | expect: final feature docs validate.

## Evidence Rules

- Every acceptance row must have RED/GREEN/VERIFY evidence in the matching report.
- Browser evidence must record clicks, inputs, network assertions, feedback, and artifacts.
- Slow browser verification runs once per relevant change and review reuses fresh passing evidence unless stale.
