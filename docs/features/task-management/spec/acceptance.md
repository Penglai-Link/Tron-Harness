# Acceptance: Task Management

## Acceptance Index

- AC-TASK-01 [must] [api] Backend exposes final status, priority, task type, and source type enums from code; proof: `rtk make be-test`.
- AC-TASK-02 [must] [business] List and kanban return workspace-scoped persisted tasks with filters and hide archived tasks by default; proof: `rtk make be-test`.
- AC-TASK-03 [must] [business] Create requires active workspace, generated taskCode, valid workspace assignees, and unchecked agent/app links; proof: `rtk make be-test`.
- AC-TASK-04 [must] [business] Detail returns parent, subtasks, assignments, links, and enforces same-workspace parent creation; proof: `rtk make be-test`.
- AC-TASK-05 [must] [business] Edit fields and dedicated status updates support `Testing`, `Reopen`, progress, and completed progress forcing; proof: `rtk make be-test`.
- AC-TASK-06 [must] [business] Archive stores previous status, blocks edits, hides by default, and restore returns previous status; proof: `rtk make be-test`.
- AC-TASK-07 [must] [interaction] Frontend supports API-backed board, create, detail, status, subtask, archive, restore, and execute Toast flows; proof: Playwright task workflow.
- AC-TASK-08 [must] [interaction] Task board exposes no import or export entry points in v1; proof: board action verification.

## Open Decisions

- None.
